//! This is the main controller for the Blackjack game. It implements the `Game` trait
//! and holds the core, non-async game logic.

use super::state::{BlackjackGame, GamePhase, Hand, HandStatus, Player};
use crate::commands::games::card::Rank;
use crate::commands::games::deck::Deck;
use crate::commands::games::engine::{Game, GamePayout, GameUpdate};
use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::ComponentInteraction;
use serenity::model::user::User;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

// This is the main entry point from the game engine. It delegates all work
// to the appropriate handlers and renderers.
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
        db: &PgPool,
    ) -> GameUpdate {
        // Handle player turn timeout before processing any interaction.
        if self.phase == GamePhase::PlayerTurns {
            let elapsed = self.last_action_time.elapsed();
            if elapsed > Duration::from_secs(60) {
                let player = &mut self.players[self.current_player_index];
                player.hands[self.current_hand_index].status = HandStatus::Stood;
                player.has_passed_turn = false;
                self.advance_turn();
                self.inactivity_warned = false; // reset for next player
            } else if elapsed > Duration::from_secs(40) && !self.inactivity_warned {
                self.inactivity_warned = true; // single warning in-channel log (embed log style not implemented)
            }
        }

        match self.phase {
            GamePhase::WaitingForPlayers => self.handle_lobby(ctx, interaction, db).await,
            GamePhase::Betting => self.handle_betting(ctx, interaction, db).await,
            GamePhase::Insurance => self.handle_insurance(ctx, interaction, db).await,
            GamePhase::PlayerTurns => self.handle_player_turn(ctx, interaction, db).await,
            GamePhase::GameOver => self.handle_game_over(ctx, interaction).await,
            GamePhase::DealerTurn => {
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
            "A new Blackjack lobby has been opened!".to_string()
        } else {
            let round_text = if self.min_bet > 0 {
                format!("`[ROUND {}]` ", self.round)
            } else {
                "".to_string()
            };
            let player_mentions = self
                .players
                .iter()
                .map(|p| {
                    if self.phase == GamePhase::GameOver && self.min_bet > 0 {
                        let total_winnings = self
                            .calculate_payouts()
                            .1
                            .iter()
                            .find(|pay| pay.user_id == p.user.id)
                            .map_or(0, |pay| pay.amount);
                        if total_winnings < 0 {
                            format!("~~<@{}>~~", p.user.id)
                        } else {
                            format!("<@{}>", p.user.id)
                        }
                    } else {
                        format!("<@{}>", p.user.id)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}{}", round_text, player_mentions)
        };

        let (embed, components) = match self.phase {
            GamePhase::WaitingForPlayers => self.render_lobby(),
            GamePhase::Betting => self.render_betting(),
            _ => self.render_table(),
        };
        (content, embed, components)
    }
}

// This block contains the core, non-async game logic and state manipulation.
// These are the "rules" of the game.
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
                has_passed_turn: false,
            }],
            dealer_hand: Hand::new(0),
            deck: Deck::new(),
            phase: GamePhase::WaitingForPlayers,
            min_bet,
            pot: 0,
            round: 1,
            ready_players: HashSet::new(),
            current_player_index: 0,
            current_hand_index: 0,
            last_action_time: Instant::now(),
            inactivity_warned: false,
        }
    }

    pub fn is_in_lobby(&self) -> bool {
        self.phase == GamePhase::WaitingForPlayers
    }

    pub(super) fn start_game(&mut self) {
        self.phase = if self.min_bet == 0 {
            GamePhase::PlayerTurns
        } else {
            GamePhase::Betting
        };
        if self.phase == GamePhase::PlayerTurns {
            self.deal_new_round();
        }
        self.last_action_time = Instant::now();
    }

    pub(super) fn deal_new_round(&mut self) {
        self.deck = Deck::new();
        self.deck.shuffle(); // Don't forget to shuffle!

        // (✓) FIXED: Use the public method `cards_remaining` instead of the private field.
        if self.deck.cards_remaining() < self.players.len() * 4 + 4 {
            self.phase = GamePhase::GameOver;
            // In a real scenario, you might want a specific message for this.
            return;
        }

        self.dealer_hand = Hand::new(0);
        self.pot = 0;
        self.current_player_index = 0;
        self.current_hand_index = 0;

        for player in self.players.iter_mut() {
            let bet = if self.min_bet == 0 {
                0
            } else {
                player.current_bet
            };
            player.hands = vec![Hand::new(bet)];
            player.insurance = 0;
            player.insurance_decision_made = false;
            player.has_passed_turn = false;
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

    pub(super) fn reset_for_next_round(&mut self) {
        // NOTE: Actual DB balance checks occur in handlers; here we only drop players who set their bet to 0 (can't afford min bet last round)
        if self.min_bet > 0 {
            self.players.retain(|p| p.current_bet >= self.min_bet);
        }
        self.ready_players.clear();
        self.pot = 0;
        self.round += 1;
        for player in self.players.iter_mut() {
            player.current_bet = self.min_bet;
            player.has_passed_turn = false;
        }
        self.phase = GamePhase::Betting;
        self.last_action_time = Instant::now();
    }

    // (✓) FIXED: Rewrote the turn-finding logic to be simpler and correct.
    pub(super) fn find_next_hand(&mut self) -> bool {
        let (start_p_idx, start_h_idx) = (self.current_player_index, self.current_hand_index);

        // Check remaining hands for the current player first.
        for h_idx in (start_h_idx + 1)..self.players[start_p_idx].hands.len() {
            if self.players[start_p_idx].hands[h_idx].status == HandStatus::Playing {
                self.current_hand_index = h_idx;
                return true;
            }
        }

        // Check subsequent players, cycling through the list once.
        for i in 1..=self.players.len() {
            let p_idx = (start_p_idx + i) % self.players.len();
            if !self.players[p_idx].has_passed_turn {
                for h_idx in 0..self.players[p_idx].hands.len() {
                    if self.players[p_idx].hands[h_idx].status == HandStatus::Playing {
                        self.current_player_index = p_idx;
                        self.current_hand_index = h_idx;
                        return true;
                    }
                }
            }
        }

        // If no one is left on the first pass, check for players who passed their turn.
        if self.players.iter().any(|p| p.has_passed_turn) {
            for i in 1..=self.players.len() {
                let p_idx = (start_p_idx + i) % self.players.len();
                if self.players[p_idx].has_passed_turn {
                    self.players[p_idx].has_passed_turn = false; // Reset pass flag for their turn.
                    for h_idx in 0..self.players[p_idx].hands.len() {
                        if self.players[p_idx].hands[h_idx].status == HandStatus::Playing {
                            self.current_player_index = p_idx;
                            self.current_hand_index = h_idx;
                            return true;
                        }
                    }
                }
            }
        }

        false
    }

    pub(super) fn advance_turn(&mut self) {
        self.last_action_time = Instant::now();
        if !self.find_next_hand() {
            self.play_dealer_turn();
        }
    }

    pub(super) fn play_dealer_turn(&mut self) {
        self.phase = GamePhase::DealerTurn;
        while self.dealer_hand.score() < 17 {
            if let Some(card) = self.deck.deal_one() {
                self.dealer_hand.add_card(card);
            } else {
                break;
            }
        }
        self.phase = GamePhase::GameOver;
        self.last_action_time = Instant::now();
    }

    pub(super) fn calculate_payouts(&self) -> (String, Vec<GamePayout>) {
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

    pub(super) async fn send_ephemeral_response(
        &self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        content: &str,
    ) {
        let builder = CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true);
        let response = CreateInteractionResponse::Message(builder);
        interaction.create_response(&ctx.http, response).await.ok();
    }
}
