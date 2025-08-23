//! This module contains the full, feature-complete implementation of the Blackjack game.

use crate::commands::games::card::{Card, Rank};
use crate::commands::games::deck::Deck;
use crate::commands::games::engine::{Game, GamePayout, GameUpdate};
use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::id::UserId;
use serenity::model::user::User;
use serenity::prelude::Context;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum GamePhase {
    WaitingForPlayers,
    Betting,
    Insurance,
    PlayerTurns,
    DealerTurn,
    GameOver,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HandStatus {
    Playing,
    Stood,
    Busted,
    Blackjack,
    Surrendered,
}

struct Hand {
    cards: Vec<Card>,
    bet: i64,
    status: HandStatus,
}

impl Hand {
    fn new(bet: i64) -> Self {
        Self {
            cards: Vec::new(),
            bet,
            status: HandStatus::Playing,
        }
    }
    fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }
    fn score(&self) -> u8 {
        let (mut score, mut ace_count): (u8, u8) = (0, 0);
        for card in &self.cards {
            let (val, _) = card.rank.value();
            score = score.saturating_add(val);
            if card.rank == Rank::Ace {
                ace_count += 1;
            }
        }
        while ace_count > 0 && score.saturating_add(10) <= 21 {
            score += 10;
            ace_count -= 1;
        }
        score
    }
    fn can_split(&self) -> bool {
        self.cards.len() == 2 && self.cards[0].rank.value().0 == self.cards[1].rank.value().0
    }
    fn can_double_down(&self) -> bool {
        self.cards.len() == 2
    }
    fn can_surrender(&self) -> bool {
        self.cards.len() == 2
    }
    fn display(&self, min_bet: i64) -> String {
        let cards_str = format!(
            "[ {} ]",
            self.cards
                .iter()
                .map(|c| c.to_string())
                .collect::<Vec<_>>()
                .join(" ")
        );
        let bet_str = if min_bet > 0 {
            format!("(Bet: 💰{})", self.bet)
        } else {
            "".to_string()
        };
        format!("{}  `Score: {}` {}", cards_str, self.score(), bet_str)
    }
}

struct Player {
    user: Arc<User>,
    hands: Vec<Hand>,
    insurance: i64,
    current_bet: i64,
    insurance_decision_made: bool,
}

pub struct BlackjackGame {
    host_id: u64,
    players: Vec<Player>,
    dealer_hand: Hand,
    deck: Deck,
    phase: GamePhase,
    min_bet: i64,
    pot: i64,
    ready_players: HashSet<UserId>,
    current_player_index: usize,
    current_hand_index: usize,
    last_action_time: Instant,
}

impl BlackjackGame {
    pub fn new(host: Arc<User>, min_bet: i64) -> Self {
        Self {
            host_id: host.id.get(),
            players: vec![Player {
                user: host,
                hands: Vec::new(),
                insurance: 0,
                current_bet: min_bet,
                insurance_decision_made: false,
            }],
            dealer_hand: Hand::new(0),
            deck: Deck::new(),
            phase: GamePhase::WaitingForPlayers,
            min_bet,
            pot: 0,
            ready_players: HashSet::new(),
            current_player_index: 0,
            current_hand_index: 0,
            last_action_time: Instant::now(),
        }
    }

    pub fn is_in_lobby(&self) -> bool {
        self.phase == GamePhase::WaitingForPlayers
    }

    fn start_game(&mut self) {
        self.phase = if self.min_bet == 0 {
            GamePhase::PlayerTurns
        } else {
            GamePhase::Betting
        };
        if self.phase == GamePhase::PlayerTurns {
            self.deal_new_round();
        }
    }

    fn deal_new_round(&mut self) {
        self.deck = Deck::new();
        self.deck.shuffle();
        self.dealer_hand = Hand::new(0);
        self.pot = 0;
        for player in self.players.iter_mut() {
            let bet = if self.min_bet == 0 {
                0
            } else {
                player.current_bet
            };
            player.hands = vec![Hand::new(bet)];
            player.insurance = 0;
            player.insurance_decision_made = false;
            self.pot += bet;
        }
        for _ in 0..2 {
            for player in self.players.iter_mut() {
                if let Some(card) = self.deck.deal_one() {
                    player.hands[0].add_card(card);
                }
            }
            if let Some(card) = self.deck.deal_one() {
                self.dealer_hand.add_card(card);
            }
        }
        for player in self.players.iter_mut() {
            if player.hands[0].score() == 21 {
                player.hands[0].status = HandStatus::Blackjack;
            }
        }
        if self
            .dealer_hand
            .cards
            .first()
            .is_some_and(|c| c.rank == Rank::Ace)
            && self.min_bet > 0
        {
            self.phase = GamePhase::Insurance;
        } else {
            self.phase = GamePhase::PlayerTurns;
            self.find_next_hand();
        }
        self.last_action_time = Instant::now();
    }

    fn reset_for_next_round(&mut self) {
        self.ready_players.clear();
        self.pot = 0;
        for player in self.players.iter_mut() {
            player.current_bet = self.min_bet;
        }
        self.phase = GamePhase::Betting;
    }

    fn find_next_hand(&mut self) -> bool {
        let (start_p_idx, start_h_idx) = (self.current_player_index, self.current_hand_index);
        for h_idx in (start_h_idx + 1)..self.players[start_p_idx].hands.len() {
            if self.players[start_p_idx].hands[h_idx].status == HandStatus::Playing {
                self.current_hand_index = h_idx;
                return true;
            }
        }
        for i in 1..=self.players.len() {
            let p_idx = (start_p_idx + i) % self.players.len();
            for h_idx in 0..self.players[p_idx].hands.len() {
                if self.players[p_idx].hands[h_idx].status == HandStatus::Playing {
                    self.current_player_index = p_idx;
                    self.current_hand_index = h_idx;
                    return true;
                }
            }
        }
        false
    }

    fn advance_turn(&mut self) {
        self.last_action_time = Instant::now();
        if !self.find_next_hand() {
            self.play_dealer_turn();
        }
    }

    fn play_dealer_turn(&mut self) {
        self.phase = GamePhase::DealerTurn;
        while self.dealer_hand.score() < 17 {
            if let Some(card) = self.deck.deal_one() {
                self.dealer_hand.add_card(card);
            } else {
                break;
            }
        }
        self.phase = GamePhase::GameOver;
    }

    fn calculate_payouts(&self) -> (String, Vec<GamePayout>) {
        if self.min_bet == 0 {
            return ("Friendly game, no payouts!".to_string(), Vec::new());
        }
        let dealer_score = self.dealer_hand.score();
        let dealer_busted = dealer_score > 21;
        let dealer_has_bj = self.dealer_hand.score() == 21 && self.dealer_hand.cards.len() == 2;
        let mut overall_results = Vec::new();
        let mut payouts = HashMap::new();
        for player in &self.players {
            let mut total_winnings = 0;
            let mut player_results = Vec::new();
            if player.insurance > 0 {
                if dealer_has_bj {
                    total_winnings += player.insurance * 2;
                    player_results.push(format!(
                        "**<@{}>**: Insurance paid **💰{}**",
                        player.user.id,
                        player.insurance * 2
                    ));
                } else {
                    total_winnings -= player.insurance;
                    player_results.push(format!(
                        "**<@{}>**: Insurance lost **💰{}**",
                        player.user.id, player.insurance
                    ));
                }
            }
            for (i, hand) in player.hands.iter().enumerate() {
                let hand_num = if player.hands.len() > 1 {
                    format!(" (Hand {})", i + 1)
                } else {
                    "".to_string()
                };
                let (result_str, net) = match hand.status {
                    HandStatus::Surrendered => ("Surrendered".to_string(), -(hand.bet / 2)),
                    HandStatus::Busted => ("Busted!".to_string(), -hand.bet),
                    HandStatus::Blackjack => {
                        if dealer_has_bj {
                            ("Push".to_string(), 0)
                        } else {
                            let winnings = (hand.bet * 3) / 2;
                            (format!("**Blackjack!** Wins 💰{}", winnings), winnings)
                        }
                    }
                    _ if dealer_busted || hand.score() > dealer_score => {
                        (format!("Wins 💰{}", hand.bet), hand.bet)
                    }
                    _ if hand.score() == dealer_score => ("Push".to_string(), 0),
                    _ => (format!("Loses 💰{}", hand.bet), -hand.bet),
                };
                player_results.push(format!(
                    "**<@{}>**{}: {}",
                    player.user.id, hand_num, result_str
                ));
                total_winnings += net;
            }
            payouts.insert(player.user.id, total_winnings);
            overall_results.push(player_results.join("\n"));
        }
        let final_payouts = payouts
            .into_iter()
            .map(|(user_id, amount)| GamePayout { user_id, amount })
            .collect();
        (overall_results.join("\n\n"), final_payouts)
    }

    async fn send_ephemeral_response(
        &self,
        ctx: &Context,
        interaction: &ComponentInteraction,
        content: &str,
    ) {
        let builder = CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true);
        let response = CreateInteractionResponse::Message(builder);
        interaction.create_response(&ctx.http, response).await.ok();
    }
}

#[async_trait]
impl Game for BlackjackGame {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    async fn handle_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        if self.phase == GamePhase::PlayerTurns
            && self.last_action_time.elapsed() > Duration::from_secs(60)
        {
            self.players[self.current_player_index].hands[self.current_hand_index].status =
                HandStatus::Stood;
            self.advance_turn();
        }

        // (✓) FIXED E0004: Added the missing match arm for `DealerTurn`.
        match self.phase {
            GamePhase::WaitingForPlayers => self.handle_lobby(ctx, interaction).await,
            GamePhase::Betting => self.handle_betting(ctx, interaction).await,
            GamePhase::Insurance => self.handle_insurance(ctx, interaction).await,
            GamePhase::PlayerTurns => self.handle_player_turn(ctx, interaction).await,
            GamePhase::GameOver => self.handle_game_over(ctx, interaction).await,
            GamePhase::DealerTurn => {
                // No player actions are allowed during the dealer's turn.
                self.send_ephemeral_response(
                    ctx,
                    interaction,
                    "Please wait, the dealer is playing their hand.",
                )
                .await;
                GameUpdate::NoOp
            }
        }
    }

    fn render(&self) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        let content = if self.phase == GamePhase::WaitingForPlayers {
            "**Blackjack Lobby**".to_string()
        } else {
            "**Blackjack Table**".to_string()
        };
        let (embed, components) = match self.phase {
            GamePhase::WaitingForPlayers => self.render_lobby(),
            GamePhase::Betting => self.render_betting(),
            _ => self.render_table(),
        };
        (content, embed, components)
    }
}

// Handler methods
impl BlackjackGame {
    async fn handle_lobby(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        match interaction.data.custom_id.as_str() {
            "bj_join" => {
                if !self
                    .players
                    .iter()
                    .any(|p| p.user.id == interaction.user.id)
                {
                    self.players.push(Player {
                        user: Arc::new(interaction.user.clone()),
                        hands: Vec::new(),
                        insurance: 0,
                        current_bet: self.min_bet,
                        insurance_decision_made: false,
                    });
                    interaction.defer(&ctx.http).await.ok();
                    GameUpdate::ReRender
                } else {
                    self.send_ephemeral_response(ctx, interaction, "You have already joined.")
                        .await;
                    GameUpdate::NoOp
                }
            }
            "bj_start" => {
                if interaction.user.id.get() == self.host_id {
                    self.start_game();
                    interaction.defer(&ctx.http).await.ok();
                    GameUpdate::ReRender
                } else {
                    self.send_ephemeral_response(ctx, interaction, "Only the host can start.")
                        .await;
                    GameUpdate::NoOp
                }
            }
            _ => GameUpdate::NoOp,
        }
    }

    async fn handle_betting(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        let player = match self
            .players
            .iter_mut()
            .find(|p| p.user.id == interaction.user.id)
        {
            Some(p) => p,
            None => {
                self.send_ephemeral_response(ctx, interaction, "You are not in this game.")
                    .await;
                return GameUpdate::NoOp;
            }
        };
        if self.ready_players.contains(&player.user.id) {
            self.send_ephemeral_response(ctx, interaction, "You have already confirmed your bet.")
                .await;
            return GameUpdate::NoOp;
        }
        match interaction.data.custom_id.as_str() {
            "bj_bet_10" => player.current_bet += 10,
            "bj_bet_100" => player.current_bet += 100,
            "bj_bet_1000" => player.current_bet += 1000,
            "bj_bet_clear" => player.current_bet = self.min_bet,
            "bj_bet_confirm" => {
                if player.current_bet < self.min_bet {
                    self.send_ephemeral_response(
                        ctx,
                        interaction,
                        &format!(
                            "Your bet must be at least the table minimum of 💰{}.",
                            self.min_bet
                        ),
                    )
                    .await;
                    return GameUpdate::NoOp;
                }
                self.ready_players.insert(interaction.user.id);
            }
            _ => return GameUpdate::NoOp,
        }
        interaction.defer(&ctx.http).await.ok();
        if self.ready_players.len() == self.players.len() {
            self.deal_new_round();
        }
        GameUpdate::ReRender
    }

    async fn handle_insurance(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        let player = match self
            .players
            .iter_mut()
            .find(|p| p.user.id == interaction.user.id)
        {
            Some(p) => p,
            None => return GameUpdate::NoOp,
        };
        if player.insurance_decision_made {
            self.send_ephemeral_response(
                ctx,
                interaction,
                "You have already made your insurance decision.",
            )
            .await;
            return GameUpdate::NoOp;
        }
        match interaction.data.custom_id.as_str() {
            "bj_insure_yes" => {
                player.insurance = self.min_bet / 2;
                player.insurance_decision_made = true;
            }
            "bj_insure_no" => {
                player.insurance = 0;
                player.insurance_decision_made = true;
            }
            _ => return GameUpdate::NoOp,
        };
        interaction.defer(&ctx.http).await.ok();
        let all_decided = self
            .players
            .iter()
            .all(|p| p.insurance_decision_made || p.hands[0].status == HandStatus::Blackjack);
        if all_decided {
            if self.dealer_hand.score() == 21 && self.dealer_hand.cards.len() == 2 {
                self.phase = GamePhase::GameOver;
            } else {
                self.phase = GamePhase::PlayerTurns;
                self.find_next_hand();
            }
        }
        if self.phase == GamePhase::GameOver {
            let (message, payouts) = self.calculate_payouts();
            GameUpdate::GameOver { message, payouts }
        } else {
            GameUpdate::ReRender
        }
    }

    async fn handle_player_turn(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        if interaction.user.id != self.players[self.current_player_index].user.id {
            self.send_ephemeral_response(ctx, interaction, "It's not your turn.")
                .await;
            return GameUpdate::NoOp;
        }
        interaction.defer(&ctx.http).await.ok();
        match interaction.data.custom_id.as_str() {
            "bj_hit" => {
                let hand =
                    &mut self.players[self.current_player_index].hands[self.current_hand_index];
                if let Some(card) = self.deck.deal_one() {
                    hand.add_card(card);
                }
                if hand.score() >= 21 {
                    hand.status = if hand.score() > 21 {
                        HandStatus::Busted
                    } else {
                        HandStatus::Stood
                    };
                    self.advance_turn();
                }
            }
            "bj_stand" => {
                self.players[self.current_player_index].hands[self.current_hand_index].status =
                    HandStatus::Stood;
                self.advance_turn();
            }
            "bj_double" => {
                let hand =
                    &mut self.players[self.current_player_index].hands[self.current_hand_index];
                if hand.can_double_down() {
                    hand.bet *= 2;
                    if let Some(card) = self.deck.deal_one() {
                        hand.add_card(card);
                    }
                    hand.status = if hand.score() > 21 {
                        HandStatus::Busted
                    } else {
                        HandStatus::Stood
                    };
                    self.advance_turn();
                }
            }
            "bj_split" => {
                let player = &mut self.players[self.current_player_index];
                if player.hands[self.current_hand_index].can_split() {
                    let hand = &mut player.hands[self.current_hand_index];
                    let split_card = hand.cards.pop().unwrap();
                    let mut new_hand = Hand::new(self.min_bet);
                    new_hand.add_card(split_card);
                    if let Some(card) = self.deck.deal_one() {
                        hand.add_card(card);
                    }
                    if let Some(card) = self.deck.deal_one() {
                        new_hand.add_card(card);
                    }
                    if hand.cards[0].rank == Rank::Ace {
                        hand.status = HandStatus::Stood;
                        new_hand.status = HandStatus::Stood;
                    }
                    player.hands.insert(self.current_hand_index + 1, new_hand);
                }
            }
            "bj_surrender" => {
                let hand =
                    &mut self.players[self.current_player_index].hands[self.current_hand_index];
                if hand.can_surrender() {
                    hand.status = HandStatus::Surrendered;
                    self.advance_turn();
                }
            }
            _ => return GameUpdate::NoOp,
        }
        if self.phase == GamePhase::GameOver {
            let (message, payouts) = self.calculate_payouts();
            GameUpdate::GameOver { message, payouts }
        } else {
            GameUpdate::ReRender
        }
    }

    async fn handle_game_over(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        if interaction.data.custom_id == "bj_next_round"
            && interaction.user.id.get() == self.host_id
        {
            interaction.defer(&ctx.http).await.ok();
            self.reset_for_next_round();
            GameUpdate::ReRender
        } else {
            self.send_ephemeral_response(
                ctx,
                interaction,
                "Only the host can start the next round.",
            )
            .await;
            GameUpdate::NoOp
        }
    }
}

// Rendering methods
impl BlackjackGame {
    fn render_lobby(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let players_list = self
            .players
            .iter()
            .map(|p| format!("<@{}>", p.user.id))
            .collect::<Vec<_>>()
            .join("\n");
        let desc = if self.min_bet > 0 {
            format!(
                "<@{}> has started a Blackjack table with a minimum bet of **💰{}**!",
                self.host_id, self.min_bet
            )
        } else {
            format!(
                "<@{}> has started a friendly (no betting) game of Blackjack!",
                self.host_id
            )
        };
        let embed = CreateEmbed::new()
            .title("♦️ Blackjack Lobby ♥️")
            .description(desc)
            .field("Players Joined", players_list, false)
            .color(0xFFA500)
            .footer(CreateEmbedFooter::new("Lobby expires in 2 minutes."));
        let buttons = vec![
            CreateButton::new("bj_join")
                .label("Join")
                .style(ButtonStyle::Success),
            CreateButton::new("bj_start")
                .label("Start Game (Host)")
                .style(ButtonStyle::Primary),
        ];
        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    fn render_betting(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let betting_status = self
            .players
            .iter()
            .map(|p| {
                let status_icon = if self.ready_players.contains(&p.user.id) {
                    "✅"
                } else {
                    "🤔"
                };
                format!(
                    "{} <@{}> — Bet: **💰{}**",
                    status_icon, p.user.id, p.current_bet
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        let embed = CreateEmbed::new()
            .title("♦️ Blackjack - Place Your Bets ♥️")
            .description(format!("Minimum Bet: **💰{}**", self.min_bet))
            .field("Betting Status", betting_status, false)
            .color(0x5865F2)
            .footer(CreateEmbedFooter::new(
                "The round will begin once all players confirm their bets.",
            ));
        let buttons1 = vec![
            CreateButton::new("bj_bet_10")
                .label("+10")
                .style(ButtonStyle::Secondary),
            CreateButton::new("bj_bet_100")
                .label("+100")
                .style(ButtonStyle::Secondary),
            CreateButton::new("bj_bet_1000")
                .label("+1K")
                .style(ButtonStyle::Secondary),
        ];
        let buttons2 = vec![
            CreateButton::new("bj_bet_clear")
                .label("Reset Bet")
                .style(ButtonStyle::Danger),
            CreateButton::new("bj_bet_confirm")
                .label("Confirm Bet")
                .style(ButtonStyle::Success),
        ];
        (
            embed,
            vec![
                CreateActionRow::Buttons(buttons1),
                CreateActionRow::Buttons(buttons2),
            ],
        )
    }

    fn render_table(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let mut embed = CreateEmbed::new().title("♦️ Blackjack Table ♥️");
        let mut components = Vec::new();

        let dealer_display =
            if self.phase == GamePhase::PlayerTurns || self.phase == GamePhase::Insurance {
                if let Some(card) = self.dealer_hand.cards.first() {
                    format!("{}  **?**", card)
                } else {
                    "Dealing...".to_string()
                }
            } else {
                self.dealer_hand
                    .cards
                    .iter()
                    .map(|c| c.to_string())
                    .collect::<Vec<_>>()
                    .join("  ")
            };
        embed = embed.field(
            format!(
                "👑 Dealer's Hand (`{}`)",
                if self.phase == GamePhase::PlayerTurns || self.phase == GamePhase::Insurance {
                    self.dealer_hand.cards[0].rank.value().0
                } else {
                    self.dealer_hand.score()
                }
            ),
            dealer_display,
            false,
        );

        if self.pot > 0 {
            embed = embed.field("Total Pot", format!("💰{}", self.pot), false);
        }

        for (p_idx, player) in self.players.iter().enumerate() {
            let turn_indicator =
                if self.phase == GamePhase::PlayerTurns && p_idx == self.current_player_index {
                    "▶️ "
                } else {
                    ""
                };
            let hands_display = player
                .hands
                .iter()
                .enumerate()
                .map(|(h_idx, hand)| {
                    let hand_indicator = if player.hands.len() > 1 {
                        format!("(Hand {})", h_idx + 1)
                    } else {
                        "".to_string()
                    };
                    let status_indicator = match hand.status {
                        HandStatus::Stood => " ✅",
                        HandStatus::Blackjack => " ⭐",
                        HandStatus::Busted => " ❌",
                        HandStatus::Surrendered => " 🏳️",
                        HandStatus::Playing => "",
                    };
                    let current_hand_marker = if p_idx == self.current_player_index
                        && h_idx == self.current_hand_index
                        && self.phase == GamePhase::PlayerTurns
                    {
                        "**>** "
                    } else {
                        ""
                    };
                    format!(
                        "{}{}{}: {}",
                        current_hand_marker,
                        hand_indicator,
                        status_indicator,
                        hand.display(self.min_bet)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            embed = embed.field(
                format!("{}👤 {}", turn_indicator, player.user.name),
                hands_display,
                true,
            );
        }

        if self.phase == GamePhase::GameOver {
            let (results_str, _) = self.calculate_payouts();
            embed = embed
                .description(format!("**--- Final Results ---**\n\n{}", results_str))
                .color(0x00FF00);
            if self.min_bet > 0 {
                components.push(CreateActionRow::Buttons(vec![
                    CreateButton::new("bj_next_round")
                        .label("Next Round (Host)")
                        .style(ButtonStyle::Primary),
                ]));
            }
        } else if self.phase == GamePhase::Insurance {
            embed = embed
                .description("The dealer is showing an Ace. **Place your insurance bets!**")
                .color(0x5865F2);
            components.push(CreateActionRow::Buttons(vec![
                CreateButton::new("bj_insure_yes")
                    .label("Insure (0.5x bet)")
                    .style(ButtonStyle::Success),
                CreateButton::new("bj_insure_no")
                    .label("No Insurance")
                    .style(ButtonStyle::Danger),
            ]));
        } else {
            // PlayerTurns
            let footer_text = format!(
                "It's <@{}>'s turn. You have 60 seconds to act.",
                self.players[self.current_player_index].user.id
            );
            embed = embed
                .footer(CreateEmbedFooter::new(footer_text))
                .color(0x5865F2);

            let mut buttons = vec![
                CreateButton::new("bj_hit")
                    .label("Hit")
                    .style(ButtonStyle::Success),
                CreateButton::new("bj_stand")
                    .label("Stand")
                    .style(ButtonStyle::Danger),
            ];

            let current_hand =
                &self.players[self.current_player_index].hands[self.current_hand_index];
            if current_hand.can_double_down() {
                buttons.push(
                    CreateButton::new("bj_double")
                        .label("Double")
                        .style(ButtonStyle::Primary),
                );
            }
            if current_hand.can_split() {
                buttons.push(
                    CreateButton::new("bj_split")
                        .label("Split")
                        .style(ButtonStyle::Secondary),
                );
            }
            if current_hand.can_surrender() {
                buttons.push(
                    CreateButton::new("bj_surrender")
                        .label("Surrender")
                        .style(ButtonStyle::Secondary),
                );
            }

            components.push(CreateActionRow::Buttons(buttons));
        }

        (embed, components)
    }
}
