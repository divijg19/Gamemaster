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
use serenity::model::user::User;
use serenity::prelude::Context;
use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum GamePhase {
    WaitingForPlayers,
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
    fn display(&self) -> String {
        let cards_str = self
            .cards
            .iter()
            .map(|c| c.to_string())
            .collect::<Vec<_>>()
            .join("  ");
        format!(
            "**Cards:** {}\n**Score:** `{}` | **Bet:** `💰{}`",
            cards_str,
            self.score(),
            self.bet
        )
    }
}

struct Player {
    user: Arc<User>,
    hands: Vec<Hand>,
    insurance: i64,
    insurance_decision_made: bool,
}

pub struct BlackjackGame {
    host_id: u64,
    players: Vec<Player>,
    dealer_hand: Hand,
    deck: Deck,
    phase: GamePhase,
    base_bet: i64,
    current_player_index: usize,
    current_hand_index: usize,
    last_action_time: Instant,
}

impl BlackjackGame {
    pub fn new(host: Arc<User>, bet: i64) -> Self {
        Self {
            host_id: host.id.get(),
            players: vec![Player {
                user: host,
                hands: vec![Hand::new(bet)],
                insurance: 0,
                insurance_decision_made: false,
            }],
            dealer_hand: Hand::new(0),
            deck: Deck::new(),
            phase: GamePhase::WaitingForPlayers,
            base_bet: bet,
            current_player_index: 0,
            current_hand_index: 0,
            last_action_time: Instant::now(),
        }
    }

    pub fn is_in_lobby(&self) -> bool {
        self.phase == GamePhase::WaitingForPlayers
    }

    fn start_game(&mut self) {
        self.deck.shuffle();
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

        // (✓) FIXED: Replaced `map_or` with the more idiomatic `is_some_and`.
        if self
            .dealer_hand
            .cards
            .first()
            .is_some_and(|c| c.rank == Rank::Ace)
        {
            self.phase = GamePhase::Insurance;
        } else {
            self.phase = GamePhase::PlayerTurns;
            self.find_next_hand();
        }
        self.last_action_time = Instant::now();
    }

    fn find_next_hand(&mut self) -> bool {
        let (start_p, start_h) = (self.current_player_index, self.current_hand_index);

        let initial_h_idx = if self.players[start_p].hands[start_h].status == HandStatus::Playing {
            start_h
        } else {
            start_h + 1
        };

        for h_idx in initial_h_idx..self.players[start_p].hands.len() {
            if self.players[start_p].hands[h_idx].status == HandStatus::Playing {
                self.current_hand_index = h_idx;
                return true;
            }
        }

        for p_idx in (start_p + 1)..self.players.len() {
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
                    // (✓) FIXED: Combined identical `if` blocks for player win conditions.
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

        match self.phase {
            GamePhase::WaitingForPlayers => self.handle_lobby(ctx, interaction).await,
            GamePhase::Insurance => self.handle_insurance(ctx, interaction).await,
            GamePhase::PlayerTurns => self.handle_player_turn(ctx, interaction).await,
            _ => {
                self.send_ephemeral_response(
                    ctx,
                    interaction,
                    "The game is over or it's not time for actions.",
                )
                .await;
                GameUpdate::NoOp
            }
        }
    }

    fn render(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        if self.phase == GamePhase::WaitingForPlayers {
            self.render_lobby()
        } else {
            self.render_table()
        }
    }
}

// Private handler methods
impl BlackjackGame {
    async fn handle_lobby(
        &mut self,
        ctx: &Context,
        interaction: &ComponentInteraction,
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
                        hands: vec![Hand::new(self.base_bet)],
                        insurance: 0,
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

    async fn handle_insurance(
        &mut self,
        ctx: &Context,
        interaction: &ComponentInteraction,
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
                player.insurance = self.base_bet / 2;
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
                    let split_card = hand.cards.pop().unwrap(); // Safe due to can_split check
                    let mut new_hand = Hand::new(self.base_bet);
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
        let embed = CreateEmbed::new()
            .title("Blackjack Lobby")
            .description(format!(
                "A new game has been started with a base bet of **💰{}**!",
                self.base_bet
            ))
            .field("Players", players_list, false)
            .color(0xFFA500)
            .footer(CreateEmbedFooter::new("Lobby expires in 2 minutes."));
        let buttons = vec![
            CreateButton::new("bj_join")
                .label("Join")
                .style(ButtonStyle::Success),
            CreateButton::new("bj_start")
                .label("Start (Host)")
                .style(ButtonStyle::Primary),
        ];
        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    fn render_table(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let mut embed = CreateEmbed::new().title("Blackjack");
        let mut components = Vec::new();

        let dealer_display =
            if self.phase == GamePhase::PlayerTurns || self.phase == GamePhase::Insurance {
                if let Some(card) = self.dealer_hand.cards.first() {
                    format!(
                        "**Cards:** {}  **?**\n**Score:** `{}`",
                        card,
                        card.rank.value().0
                    )
                } else {
                    "Dealing...".to_string()
                }
            } else {
                self.dealer_hand.display()
            };
        embed = embed.field("Dealer's Hand", dealer_display, false);

        for (p_idx, player) in self.players.iter().enumerate() {
            for (h_idx, hand) in player.hands.iter().enumerate() {
                let turn_indicator = if self.phase == GamePhase::PlayerTurns
                    && p_idx == self.current_player_index
                    && h_idx == self.current_hand_index
                {
                    "▶️ "
                } else {
                    ""
                };
                let hand_indicator = if player.hands.len() > 1 {
                    format!(" (Hand {})", h_idx + 1)
                } else {
                    "".to_string()
                };
                let status_indicator = match hand.status {
                    HandStatus::Stood | HandStatus::Blackjack => " ✅",
                    HandStatus::Busted => " ❌",
                    HandStatus::Surrendered => " 🏳️",
                    HandStatus::Playing => "",
                };
                embed = embed.field(
                    format!(
                        "{}{} {}{}",
                        turn_indicator, player.user.name, hand_indicator, status_indicator
                    ),
                    hand.display(),
                    true,
                );
            }
        }

        if self.phase == GamePhase::GameOver {
            let (results_str, _) = self.calculate_payouts();
            embed = embed.description(results_str).color(0x00FF00);
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
