//! Handles all `ComponentInteraction` events for the Poker game.

use super::state::{GamePhase, Player, PlayerStatus, PokerGame}; // (✓) FIXED: Imported PokerGame, not BlackjackGame.
use crate::commands::games::GameUpdate; // (✓) Using re-export from commands::games
use serenity::builder::{CreateInteractionResponse, CreateInteractionResponseMessage};
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use sqlx::PgPool;
use std::sync::Arc;

async fn get_balance(db: &PgPool, user_id: serenity::model::id::UserId) -> i64 {
    match sqlx::query!(
        "SELECT balance FROM profiles WHERE user_id = $1",
        user_id.get() as i64
    )
    .fetch_optional(db)
    .await
    {
        Ok(Some(record)) => record.balance,
        _ => 0,
    }
}

async fn send_ephemeral_response(ctx: &Context, interaction: &ComponentInteraction, content: &str) {
    let builder = CreateInteractionResponseMessage::new()
        .content(content)
        .ephemeral(true);
    let response = CreateInteractionResponse::Message(builder);
    interaction.create_response(&ctx.http, response).await.ok();
}

impl PokerGame {
    pub(super) async fn handle_lobby(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        match interaction.data.custom_id.as_str() {
            "poker_join" => {
                if self.players.len() >= 5 {
                    send_ephemeral_response(ctx, interaction, "Sorry, this poker table is full.")
                        .await;
                    return GameUpdate::NoOp;
                }
                if !self
                    .players
                    .iter()
                    .any(|p| p.user.id == interaction.user.id)
                {
                    let balance = get_balance(db, interaction.user.id).await;
                    if balance < self.min_bet {
                        send_ephemeral_response(
                            ctx,
                            interaction,
                            &format!(
                                "You cannot afford the ante of **💰{}** to join.",
                                self.min_bet
                            ),
                        )
                        .await;
                        return GameUpdate::NoOp;
                    }
                    self.players.push(Player {
                        user: Arc::new(interaction.user.clone()),
                        hand: Vec::new(),
                        hand_rank: None,
                        ante_bet: 0,
                        play_bet: 0,
                        status: PlayerStatus::Waiting,
                    });
                    interaction.defer(&ctx.http).await.ok();
                    GameUpdate::ReRender
                } else {
                    send_ephemeral_response(ctx, interaction, "You have already joined.").await;
                    GameUpdate::NoOp
                }
            }
            "poker_start" => {
                if interaction.user.id.get() == self.host_id {
                    self.start_game();
                    interaction.defer(&ctx.http).await.ok();
                    GameUpdate::ReRender
                } else {
                    send_ephemeral_response(ctx, interaction, "Only the host can start the game.")
                        .await;
                    GameUpdate::NoOp
                }
            }
            "poker_cancel" => {
                if interaction.user.id.get() == self.host_id {
                    interaction.defer(&ctx.http).await.ok();
                    self.phase = GamePhase::GameOver;
                    GameUpdate::GameOver {
                        message: "Game cancelled by host.".to_string(),
                        payouts: vec![],
                    }
                } else {
                    send_ephemeral_response(ctx, interaction, "Only the host can cancel the game.")
                        .await;
                    GameUpdate::NoOp
                }
            }
            _ => GameUpdate::NoOp,
        }
    }

    pub(super) async fn handle_ante_phase(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        let player = match self
            .players
            .iter_mut()
            .find(|p| p.user.id == interaction.user.id)
        {
            Some(p) => p,
            None => return GameUpdate::NoOp,
        };
        if self.ready_players.contains(&interaction.user.id) {
            send_ephemeral_response(ctx, interaction, "You have already placed your ante.").await;
            return GameUpdate::NoOp;
        }

        let balance = get_balance(db, interaction.user.id).await;
        if balance < self.min_bet {
            send_ephemeral_response(
                ctx,
                interaction,
                &format!(
                    "You can no longer afford the ante of **💰{}**.",
                    self.min_bet
                ),
            )
            .await;
            return GameUpdate::NoOp;
        }

        player.ante_bet = self.min_bet;
        self.pot += self.min_bet;
        self.ready_players.insert(interaction.user.id);

        interaction.defer(&ctx.http).await.ok();

        if self.ready_players.len() == self.players.len() {
            self.deal_new_round();
        }

        GameUpdate::ReRender
    }

    pub(super) async fn handle_player_turn(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) -> GameUpdate {
        if interaction.user.id != self.players[self.current_player_index].user.id {
            send_ephemeral_response(ctx, interaction, "It's not your turn.").await;
            return GameUpdate::NoOp;
        }

        let player = &mut self.players[self.current_player_index];
        match interaction.data.custom_id.as_str() {
            "poker_bet" => {
                let play_bet_amount = player.ante_bet * 2;
                let balance = get_balance(db, player.user.id).await;
                if balance < player.ante_bet + play_bet_amount {
                    send_ephemeral_response(ctx, interaction, "You cannot afford the play bet.")
                        .await;
                    return GameUpdate::NoOp;
                }
                player.play_bet = play_bet_amount;
                self.pot += play_bet_amount;
                player.status = PlayerStatus::Playing;
            }
            "poker_fold" => {
                player.status = PlayerStatus::Folded;
            }
            _ => return GameUpdate::NoOp,
        }

        interaction.defer(&ctx.http).await.ok();
        self.advance_turn();

        if self.phase == GamePhase::GameOver {
            let (message, payouts) = self.calculate_payouts();
            GameUpdate::GameOver { message, payouts }
        } else {
            GameUpdate::ReRender
        }
    }

    pub(super) async fn handle_game_over(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate {
        if interaction.data.custom_id == "poker_next_round"
            && interaction.user.id.get() == self.host_id
        {
            interaction.defer(&ctx.http).await.ok();
            self.reset_for_next_round();
            GameUpdate::ReRender
        } else {
            send_ephemeral_response(ctx, interaction, "Only the host can start the next round.")
                .await;
            GameUpdate::NoOp
        }
    }
}
