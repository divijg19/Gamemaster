//! This module contains the core implementation of the RPS game,
//! adhering to the generic `Game` trait.

use super::state::{GameState, Move, RoundOutcome};
use crate::commands::games::engine::{Game, GamePayout, GameUpdate};
use serenity::async_trait;
use serenity::builder::{
    CreateActionRow, CreateButton, CreateEmbed, CreateEmbedFooter, CreateInteractionResponse,
    CreateInteractionResponseMessage,
};
use serenity::model::application::{ButtonStyle, ComponentInteraction};
use serenity::model::id::UserId;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::any::Any;

/// This struct holds the state of an active RPS game and implements the `Game` trait.
pub struct RpsGame {
    pub state: GameState,
}

#[async_trait]
impl Game for RpsGame {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    /// Signature updated to match the new Game trait. The `_db` parameter is ignored.
    async fn handle_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        _db: &PgPool,
    ) -> GameUpdate {
        let custom_id_parts: Vec<&str> = interaction.data.custom_id.split('_').collect();
        let action = custom_id_parts.get(1).unwrap_or(&"");

        match *action {
            "accept" => self.handle_accept(ctx, interaction).await,
            "decline" => self.handle_decline(ctx, interaction).await,
            "move" => self.handle_move(ctx, interaction).await,
            _ => GameUpdate::NoOp,
        }
    }

    /// Checks for the declined state first to show the final message.
    fn render(&self) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        if self.state.declined {
            self.render_declined()
        } else if !self.state.accepted {
            self.render_challenge()
        } else {
            self.render_active_game()
        }
    }
}

impl RpsGame {
    /// Sends a direct, ephemeral response to an interaction for user errors.
    async fn send_ephemeral_response(
        &self,
        ctx: &Context,
        interaction: &ComponentInteraction,
        content: &str,
    ) {
        let response = CreateInteractionResponseMessage::new()
            .content(content)
            .ephemeral(true);
        let builder = CreateInteractionResponse::Message(response);
        if let Err(e) = interaction.create_response(&ctx.http, builder).await {
            println!("[RPS] Error sending ephemeral response: {:?}", e);
        }
    }

    /// Handles the "accept" button press.
    async fn handle_accept(
        &mut self,
        ctx: &Context,
        interaction: &ComponentInteraction,
    ) -> GameUpdate {
        let p2_id: UserId = interaction
            .data
            .custom_id
            .split('_')
            .nth(3)
            .unwrap_or("0")
            .parse()
            .unwrap_or(0)
            .into();
        if interaction.user.id != p2_id {
            self.send_ephemeral_response(ctx, interaction, "This is not your challenge to accept.")
                .await;
            return GameUpdate::NoOp;
        }

        interaction.defer(&ctx.http).await.ok();
        self.state.accepted = true;
        GameUpdate::ReRender
    }

    /// Handles the "decline" button press.
    async fn handle_decline(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        let p2_id: UserId = interaction
            .data
            .custom_id
            .split('_')
            .nth(3)
            .unwrap_or("0")
            .parse()
            .unwrap_or(0)
            .into();
        if interaction.user.id != p2_id {
            self.send_ephemeral_response(
                ctx,
                interaction,
                "This is not your challenge to decline.",
            )
            .await;
            return GameUpdate::NoOp;
        }

        interaction.defer(&ctx.http).await.ok();
        self.state.declined = true;

        GameUpdate::GameOver {
            message: format!("{} declined the challenge.", self.state.player2.name),
            payouts: vec![
                GamePayout {
                    user_id: self.state.player1.id,
                    amount: 0,
                },
                GamePayout {
                    user_id: self.state.player2.id,
                    amount: 0,
                },
            ],
        }
    }

    /// Handles a player's move.
    async fn handle_move(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        if interaction.user.id != self.state.player1.id
            && interaction.user.id != self.state.player2.id
        {
            self.send_ephemeral_response(ctx, interaction, "You are not a player in this game.")
                .await;
            return GameUpdate::NoOp;
        }
        let player_move = match interaction.data.custom_id.split('_').nth(2) {
            Some("rock") => Move::Rock,
            Some("paper") => Move::Paper,
            Some("scissors") => Move::Scissors,
            _ => return GameUpdate::NoOp,
        };
        let is_player1 = interaction.user.id == self.state.player1.id;
        if (is_player1 && self.state.p1_move.is_some())
            || (!is_player1 && self.state.p2_move.is_some())
        {
            self.send_ephemeral_response(ctx, interaction, "You have already locked in your move.")
                .await;
            return GameUpdate::NoOp;
        }

        interaction.defer(&ctx.http).await.ok();
        if is_player1 {
            self.state.p1_move = Some(player_move);
        } else {
            self.state.p2_move = Some(player_move);
        }
        if self.state.p1_move.is_some() && self.state.p2_move.is_some() {
            self.state.process_round();
        }

        if self.state.is_over() {
            let (winner_id, loser_id) = if self.state.scores.p1 > self.state.scores.p2 {
                (self.state.player1.id, self.state.player2.id)
            } else {
                (self.state.player2.id, self.state.player1.id)
            };
            GameUpdate::GameOver {
                message: "The winner has been decided!".to_string(),
                payouts: vec![
                    GamePayout {
                        user_id: winner_id,
                        amount: self.state.bet,
                    },
                    GamePayout {
                        user_id: loser_id,
                        amount: -self.state.bet,
                    },
                ],
            }
        } else {
            GameUpdate::ReRender
        }
    }

    // --- Rendering Functions ---

    /// (✓) ADDED: A new helper function to build the dynamic header content.
    fn render_header_content(&self) -> String {
        let p1_id = self.state.player1.id;
        let p2_id = self.state.player2.id;

        if self.state.is_over() {
            let (winner_id, loser_id) = if self.state.scores.p1 > self.state.scores.p2 {
                (p1_id, p2_id)
            } else {
                (p2_id, p1_id)
            };
            // Format the final result with the loser struck through.
            format!(
                "`[ROUND {}]` <@{}> vs ~~<@{}>~~",
                self.state.round - 1, // Show the final round number
                winner_id,
                loser_id
            )
        } else {
            // Dynamically format based on whether each player has made a move.
            let p1_mention = if self.state.p1_move.is_some() {
                format!("~~<@{}>~~", p1_id)
            } else {
                format!("<@{}>", p1_id)
            };
            let p2_mention = if self.state.p2_move.is_some() {
                format!("~~<@{}>~~", p2_id)
            } else {
                format!("<@{}>", p2_id)
            };
            format!(
                "`[ROUND {}]` {} vs {}",
                self.state.round, p1_mention, p2_mention
            )
        }
    }

    fn render_declined(&self) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        let content = format!(
            "<@{}> vs <@{}> - Challenge Declined",
            self.state.player1.id, self.state.player2.id
        );
        let embed = CreateEmbed::new()
            .title(format!("Rock Paper Scissors | {}", self.state.format))
            .color(0xFF0000)
            .description(format!(
                "**<@{}> declined the challenge.**",
                self.state.player2.id
            ));
        (content, embed, vec![])
    }

    pub fn render_timeout_message(
        state: &GameState,
    ) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        let content = format!(
            "<@{}> vs ~~<@{}>~~ - Timed Out",
            state.player1.id, state.player2.id
        );
        let mut embed = CreateEmbed::new()
            .title(format!("Rock Paper Scissors | {}", state.format))
            .color(0xFF0000)
            .field(state.player1.name.clone(), "Status: 👑", true)
            .field(
                format!("`{}` vs `{}`", state.scores.p1, state.scores.p2),
                "\u{200B}",
                true,
            )
            .field(state.player2.name.clone(), "Status: ⛓️‍💥 Timed Out", true)
            .field("\u{200B}", "The challenge was not accepted in time.", false);
        if state.bet > 0 {
            embed = embed.field("Bet Amount (Returned)", format!("💰 {}", state.bet), false);
        }
        let disabled_buttons = vec![
            CreateButton::new("rps_disabled_accept")
                .label("Accept")
                .style(ButtonStyle::Success)
                .disabled(true),
            CreateButton::new("rps_disabled_decline")
                .label("Decline")
                .style(ButtonStyle::Danger)
                .disabled(true),
        ];
        (
            content,
            embed,
            vec![CreateActionRow::Buttons(disabled_buttons)],
        )
    }

    fn render_challenge(&self) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        let content = format!(
            "<@{}> vs <@{}>",
            self.state.player1.id, self.state.player2.id
        );
        let mut embed = CreateEmbed::new()
            .title(format!("Rock Paper Scissors | {}", self.state.format))
            .color(0xFFA500)
            .field(self.state.player1.name.clone(), "Status: 🕰️ Waiting", true)
            .field("`0` vs `0`", "\u{200B}", true)
            .field(self.state.player2.name.clone(), "Status: 🕰️ Waiting", true)
            .footer(CreateEmbedFooter::new(format!(
                "{}, you have 30 seconds to respond.",
                self.state.player2.name
            )));
        let challenge_text = if self.state.bet > 0 {
            format!("A challenge has been issued for **💰 {}**!", self.state.bet)
        } else {
            "A challenge has been issued!".to_string()
        };
        embed = embed.field("\u{200B}", challenge_text, false);
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
        (content, embed, vec![CreateActionRow::Buttons(buttons)])
    }

    fn render_active_game(&self) -> (String, CreateEmbed, Vec<CreateActionRow>) {
        // (✓) MODIFIED: Call the new helper function to get the dynamic content string.
        let content = self.render_header_content();
        let (p1_status, p2_status) = self.get_player_statuses();
        let log_content = self.get_log_content();
        let footer_text = self.get_footer_text();
        let bet_display = if self.state.bet > 0 {
            format!("💰 **{}**", self.state.bet)
        } else {
            "\u{200B}".to_string()
        };
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
                bet_display,
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
            vec![CreateActionRow::Buttons(vec![
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
            ])]
        };
        (content, embed, components)
    }

    fn get_player_statuses(&self) -> (String, String) {
        if self.state.is_over() {
            if self.state.scores.p1 > self.state.scores.p2 {
                ("👑".to_string(), "⛓️‍💥".to_string())
            } else {
                ("⛓️‍💥".to_string(), "👑".to_string())
            }
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
            if self.state.bet > 0 {
                format!(
                    "{} is the winner and gets 💰 {}!",
                    winner.name, self.state.bet
                )
            } else {
                format!("{} is the winner!", winner.name)
            }
        } else {
            match (self.state.p1_move, self.state.p2_move) {
                (None, None) => "Waiting for both players...".to_string(),
                (Some(_), None) => format!("Waiting for {}...", self.state.player2.name),
                (None, Some(_)) => format!("Waiting for {}...", self.state.player1.name),
                (Some(_), Some(_)) => "Processing round... 💭".to_string(),
            }
        }
    }
}
