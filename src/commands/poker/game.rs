//! This is the main controller for the Poker game. It implements the `Game` trait
//! and holds the core, non-async game logic.

use super::hand_eval::evaluate_hand;
use super::state::{GamePhase, HandRank, Player, PlayerStatus, PokerGame};
use crate::commands::games::card::Rank;
use crate::commands::games::deck::Deck;
use crate::commands::games::{Game, GamePayout, GameUpdate};
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
impl Game for PokerGame {
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
        // Handle anti-stalling timeouts before processing any user clicks.
        let now = Instant::now();
        match self.phase {
            GamePhase::Ante
                if now.duration_since(self.last_action_time) > Duration::from_secs(60) =>
            {
                return GameUpdate::GameOver {
                    message: "Game cancelled due to inactivity during ante phase.".to_string(),
                    payouts: vec![],
                };
            }
            GamePhase::PlayerTurns
                if now.duration_since(self.last_action_time) > Duration::from_secs(60) =>
            {
                self.players[self.current_player_index].status = PlayerStatus::Folded;
                self.advance_turn();
            }
            GamePhase::GameOver
                if now.duration_since(self.last_action_time) > Duration::from_secs(60) =>
            {
                return GameUpdate::GameOver {
                    message: "Game ended due to host inactivity.".to_string(),
                    payouts: self.calculate_payouts().1,
                };
            }
            _ => {}
        }

        // Delegate the actual interaction handling to the appropriate function.
        match self.phase {
            GamePhase::WaitingForPlayers => self.handle_lobby(ctx, interaction, db).await,
            GamePhase::Ante => self.handle_ante_phase(ctx, interaction, db).await,
            GamePhase::PlayerTurns => self.handle_player_turn(ctx, interaction, db).await,
            GamePhase::GameOver => self.handle_game_over(ctx, interaction).await,
            // (✓) FIXED: Added the missing match arm for DealerTurn.
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
        let content = if self.phase != GamePhase::WaitingForPlayers {
            let round_text = format!("[ROUND {}] ", self.round);
            let player_mentions = self
                .players
                .iter()
                .map(|p| {
                    if self.phase == GamePhase::GameOver {
                        let total_winnings = self
                            .calculate_payouts()
                            .1
                            .iter()
                            .find(|pay| pay.user_id == p.user.id)
                            .map_or(0, |pay| pay.amount);
                        if total_winnings < 0 || p.status == PlayerStatus::Folded {
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
        } else {
            "A new Poker lobby has been opened!".to_string()
        };

        let (embed, components) = match self.phase {
            GamePhase::WaitingForPlayers => self.render_lobby(),
            GamePhase::Ante => self.render_ante_phase(),
            _ => self.render_table(),
        };
        (content, embed, components)
    }
}

// This block contains the core, non-async game logic and state manipulation.
// (✓) FIXED: All methods are now in a single, consolidated impl block.
impl PokerGame {
    pub fn new(host: Arc<User>, min_bet: i64) -> Self {
        Self {
            host_id: host.id.get(),
            players: vec![Player {
                user: host,
                hand: Vec::new(),
                hand_rank: None,
                ante_bet: 0,
                play_bet: 0,
                status: PlayerStatus::Waiting,
            }],
            dealer_hand: Vec::new(),
            dealer_rank: None,
            deck: Deck::new(),
            phase: GamePhase::WaitingForPlayers,
            min_bet,
            pot: 0,
            round: 1,
            ready_players: HashSet::new(),
            current_player_index: 0,
            last_action_time: Instant::now(),
        }
    }

    // (✓) ADDED: Helper function used by the timeout handler in run.rs.
    pub fn is_in_lobby(&self) -> bool {
        self.phase == GamePhase::WaitingForPlayers
    }

    pub fn start_game(&mut self) {
        self.phase = GamePhase::Ante;
        self.last_action_time = Instant::now();
    }

    pub fn deal_new_round(&mut self) {
        self.deck = Deck::new();
        self.deck.shuffle();

        if self.deck.cards_remaining() < self.players.len() * 5 + 5 {
            self.phase = GamePhase::GameOver;
            return;
        }

        self.current_player_index = 0;
        self.dealer_hand = self.deck.deal(5).unwrap_or_default();
        self.dealer_rank = Some(evaluate_hand(&self.dealer_hand));

        for player in self.players.iter_mut() {
            player.status = PlayerStatus::Playing;
            player.hand = self.deck.deal(5).unwrap_or_default();
            player.hand_rank = Some(evaluate_hand(&player.hand));
        }

        self.phase = GamePhase::PlayerTurns;
        self.last_action_time = Instant::now();
    }

    pub fn reset_for_next_round(&mut self) {
        self.ready_players.clear();
        self.pot = 0;
        self.round += 1;
        self.phase = GamePhase::Ante;
        self.last_action_time = Instant::now();
    }

    pub fn advance_turn(&mut self) {
        self.last_action_time = Instant::now();
        if let Some(next_player_index) = ((self.current_player_index + 1)..self.players.len())
            .find(|&i| self.players[i].status == PlayerStatus::Playing)
        {
            self.current_player_index = next_player_index;
        } else {
            self.play_dealer_turn();
        }
    }

    pub fn play_dealer_turn(&mut self) {
        self.phase = GamePhase::GameOver;
        self.last_action_time = Instant::now();
    }

    pub fn calculate_payouts(&self) -> (String, Vec<GamePayout>) {
        let dealer_rank = self.dealer_rank.unwrap_or(HandRank::HighCard(0));
        let dealer_qualifies = dealer_rank >= HandRank::HighCard(Rank::King as u8);
        let mut overall_results = Vec::new();
        let mut payouts = HashMap::new();

        for player in &self.players {
            let mut total_winnings = 0;
            let result_str = match player.status {
                PlayerStatus::Folded => {
                    total_winnings -= player.ante_bet;
                    "Folded.".to_string()
                }
                PlayerStatus::Playing => {
                    let player_rank = player.hand_rank.unwrap_or(HandRank::HighCard(0));
                    if !dealer_qualifies {
                        total_winnings += player.ante_bet;
                        "**Wins!** Dealer did not qualify (Ante pays 1:1).".to_string()
                    } else if player_rank > dealer_rank {
                        total_winnings += player.ante_bet;
                        total_winnings += player.play_bet;
                        format!("**Wins!** Beats dealer's {:?}.", dealer_rank)
                    } else if player_rank == dealer_rank {
                        "Push.".to_string()
                    } else {
                        total_winnings -= player.ante_bet;
                        total_winnings -= player.play_bet;
                        format!("Loses to dealer's {:?}.", dealer_rank)
                    }
                }
                _ => "Status Error".to_string(),
            };
            payouts.insert(player.user.id, total_winnings);
            overall_results.push(format!("**<@{}>**: {}", player.user.id, result_str));
        }
        let final_payouts = payouts
            .into_iter()
            .map(|(user_id, amount)| GamePayout { user_id, amount })
            .collect();
        (overall_results.join("\n"), final_payouts)
    }

    // (✓) FIXED: This helper function is now correctly part of the main impl block.
    async fn send_ephemeral_response(
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
