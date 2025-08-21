//! This module contains the core implementation of the RPS game,
//! adhering to the generic `Game` trait.

use super::state::{GameState, Move, RoundOutcome};
use crate::commands::games::{Game, GameUpdate};
use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::id::UserId;
use serenity::prelude::Context;
use std::any::Any; // (✓) ADDED: Import `Any` for downcasting.

/// This struct holds the state of an active RPS game and implements the `Game` trait.
pub struct RpsGame {
    pub state: GameState,
}

#[async_trait]
impl Game for RpsGame {
    // (✓) ADDED: Implement the required methods for downcasting.
    // This allows the timeout logic to safely identify this game type.
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Handles all button presses for an RPS game.
    async fn handle_interaction(
        &mut self,
        _ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        let custom_id_parts: Vec<&str> = interaction.data.custom_id.split('_').collect();
        let action = custom_id_parts.get(1).unwrap_or(&"");

        match *action {
            "accept" => {
                let p2_id: UserId = custom_id_parts
                    .get(3)
                    .unwrap_or(&"")
                    .parse()
                    .unwrap_or(0)
                    .into();
                if interaction.user.id != p2_id {
                    return GameUpdate::NoOp;
                }
                self.state.accepted = true;
                GameUpdate::ReRender
            }
            "decline" => {
                let p2_id: UserId = custom_id_parts
                    .get(3)
                    .unwrap_or(&"")
                    .parse()
                    .unwrap_or(0)
                    .into();
                if interaction.user.id != p2_id {
                    return GameUpdate::NoOp;
                }
                GameUpdate::GameOver("Game was declined by the opponent.".to_string())
            }
            "move" => {
                if interaction.user.id != self.state.player1.id
                    && interaction.user.id != self.state.player2.id
                {
                    return GameUpdate::NoOp;
                }
                let player_move = match *custom_id_parts.get(2).unwrap_or(&"") {
                    "rock" => Move::Rock,
                    "paper" => Move::Paper,
                    "scissors" => Move::Scissors,
                    _ => return GameUpdate::NoOp,
                };

                if interaction.user.id == self.state.player1.id {
                    self.state.p1_move = Some(player_move);
                } else {
                    self.state.p2_move = Some(player_move);
                }

                if self.state.p1_move.is_some() && self.state.p2_move.is_some() {
                    self.state.process_round();
                }

                if self.state.is_over() {
                    GameUpdate::GameOver("The winner has been decided!".to_string())
                } else {
                    GameUpdate::ReRender
                }
            }
            _ => GameUpdate::NoOp,
        }
    }

    /// Renders the current state of the game into an embed and action rows.
    fn render(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        if !self.state.accepted {
            return self.render_challenge();
        }
        self.render_active_game()
    }
}

impl RpsGame {
    // (✓) ADDED: A public function to create the specific timeout embed.
    pub fn render_timeout_message(state: &GameState) -> (CreateEmbed, Vec<CreateActionRow>) {
        let embed = CreateEmbed::new()
            .title(format!("Rock Paper Scissors | {}", state.format))
            .color(0xFF0000) // ERROR_COLOR
            .field(state.player1.name.clone(), "Status: —", true)
            .field(
                format!("`{}` vs `{}`", state.scores.p1, state.scores.p2),
                "\u{200B}",
                true,
            )
            .field(state.player2.name.clone(), "Status: Did not respond", true)
            .field("\u{200B}", "The challenge was not accepted in time.", false);

        let disabled_buttons = vec![
            CreateButton::new("disabled_accept")
                .label("Accept")
                .style(ButtonStyle::Success)
                .disabled(true),
            CreateButton::new("disabled_decline")
                .label("Decline")
                .style(ButtonStyle::Danger)
                .disabled(true),
        ];

        (embed, vec![CreateActionRow::Buttons(disabled_buttons)])
    }

    /// Renders the initial challenge screen with "Accept" and "Decline" buttons.
    fn render_challenge(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let embed = CreateEmbed::new()
            .title(format!("Rock Paper Scissors | {}", self.state.format))
            .color(0xFFA500) // PENDING_COLOR
            .field(self.state.player1.name.clone(), "Status: 🕰️ Waiting", true)
            .field("`0` vs `0`", "\u{200B}", true)
            .field(self.state.player2.name.clone(), "Status: 🕰️ Waiting", true)
            .field("\u{200B}", "A challenge has been issued!", false)
            .footer(CreateEmbedFooter::new(format!(
                "{}, you have 30 seconds to respond.",
                self.state.player2.name
            )));

        let buttons = vec![
            CreateButton::new(format!(
                "rps_accept_{}_{}",
                self.state.player1.id, self.state.player2.id
            ))
            .label("Accept")
            .style(ButtonStyle::Success),
            CreateButton::new(format!(
                "rps_decline_{}_{}",
                self.state.player1.id, self.state.player2.id
            ))
            .label("Decline")
            .style(ButtonStyle::Danger),
        ];

        (embed, vec![CreateActionRow::Buttons(buttons)])
    }

    /// Renders the main game screen with player stats, game log, and move buttons.
    fn render_active_game(&self) -> (CreateEmbed, Vec<CreateActionRow>) {
        let (p1_status, p2_status) = self.get_player_statuses();
        let log_content = self.get_log_content();
        let footer_text = self.get_footer_text();

        let embed = CreateEmbed::new()
            .title(format!("Rock Paper Scissors | {}", self.state.format))
            .color(if self.state.is_over() {
                0x00FF00
            } else {
                0x5865F2
            })
            .field(
                self.state.player1.name.clone(),
                format!("Status: {}", p1_status),
                true,
            )
            .field(
                format!("`{}` vs `{}`", self.state.scores.p1, self.state.scores.p2),
                "\u{200B}",
                true,
            )
            .field(
                self.state.player2.name.clone(),
                format!("Status: {}", p2_status),
                true,
            )
            .field("\u{200B}", log_content, false)
            .footer(CreateEmbedFooter::new(footer_text));

        let components = if self.state.is_over() {
            vec![]
        } else {
            let buttons = vec![
                CreateButton::new("rps_move_rock")
                    .label("Rock")
                    .emoji('✊')
                    .style(ButtonStyle::Secondary),
                CreateButton::new("rps_move_paper")
                    .label("Paper")
                    .emoji('✋')
                    .style(ButtonStyle::Secondary),
                CreateButton::new("rps_move_scissors")
                    .label("Scissors")
                    .emoji('✌')
                    .style(ButtonStyle::Secondary),
            ];
            vec![CreateActionRow::Buttons(buttons)]
        };

        (embed, components)
    }

    fn get_player_statuses(&self) -> (String, String) {
        if self.state.is_over() {
            ("Game Over".to_string(), "Game Over".to_string())
        } else {
            let p1 = if self.state.p1_move.is_some() {
                "✅ Move Locked"
            } else {
                "🕰️ Waiting"
            };
            let p2 = if self.state.p2_move.is_some() {
                "✅ Move Locked"
            } else {
                "🕰️ Waiting"
            };
            (p1.to_string(), p2.to_string())
        }
    }

    fn get_log_content(&self) -> String {
        if self.state.history.is_empty() {
            "The duel has begun! Make your move.".to_string()
        } else {
            self.state
                .history
                .iter()
                .enumerate()
                .map(|(i, record)| {
                    let outcome_text = match &record.outcome {
                        RoundOutcome::Tie => "Draw!".to_string(),
                        RoundOutcome::Winner(id) => format!("<@{}> won!", id),
                    };
                    format!(
                        "`{}.` {} vs {} — {}",
                        i + 1,
                        record.p1_move.to_emoji(),
                        record.p2_move.to_emoji(),
                        outcome_text
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }

    fn get_footer_text(&self) -> String {
        if self.state.is_over() {
            let winner = if self.state.scores.p1 > self.state.scores.p2 {
                &self.state.player1
            } else {
                &self.state.player2
            };
            format!("{} is the winner!", winner.name)
        } else {
            match (self.state.p1_move, self.state.p2_move) {
                (None, None) => "Waiting for both players...".to_string(),
                (Some(_), None) => format!("Waiting for {}...", self.state.player2.name),
                (None, Some(_)) => format!("Waiting for {}...", self.state.player1.name),
                (Some(_), Some(_)) => "Processing round...".to_string(),
            }
        }
    }
}
