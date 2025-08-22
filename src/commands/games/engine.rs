//! This module contains the core, generic game engine components.
//! It defines the `Game` trait that all games must implement, and the
//! `GameManager` which tracks and routes interactions for all active games.

use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateEmbed, EditMessage};
use serenity::model::application::ComponentInteraction;
use serenity::model::id::{MessageId, UserId};
use serenity::prelude::Context;
use sqlx::PgPool;
use std::any::Any;
use std::collections::HashMap;

pub enum GameUpdate {
    ReRender,
    GameOver {
        message: String,
        winner: Option<UserId>,
        loser: Option<UserId>,
        bet: i64,
    },
    NoOp,
}

#[async_trait]
pub trait Game: Send + Sync {
    fn as_any(&self) -> &dyn Any;
    #[allow(dead_code)]
    fn as_any_mut(&mut self) -> &mut dyn Any;
    async fn handle_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate;
    fn render(&self) -> (CreateEmbed, Vec<CreateActionRow>);
}

pub struct GameManager {
    active_games: HashMap<MessageId, Box<dyn Game>>,
}

impl GameManager {
    pub fn new() -> Self {
        Self {
            active_games: HashMap::new(),
        }
    }

    pub fn start_game(&mut self, message_id: MessageId, game: Box<dyn Game>) {
        self.active_games.insert(message_id, game);
    }

    pub fn get_game_mut(&mut self, message_id: &MessageId) -> Option<&mut Box<dyn Game>> {
        self.active_games.get_mut(message_id)
    }

    pub fn remove_game(&mut self, message_id: &MessageId) {
        self.active_games.remove(message_id);
    }

    pub async fn on_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
        db: &PgPool,
    ) {
        if let Some(game) = self.get_game_mut(&interaction.message.id) {
            match game.handle_interaction(ctx, interaction).await {
                GameUpdate::ReRender => {
                    let (embed, components) = game.render();
                    let builder = EditMessage::new().embed(embed).components(components);
                    if let Err(e) = interaction.message.edit(&ctx.http, builder).await {
                        println!("[GAME MANAGER] Error editing game message: {:?}", e);
                    }
                }
                GameUpdate::GameOver {
                    message,
                    winner,
                    loser,
                    bet,
                } => {
                    println!("[GAME MANAGER] Game over: {}", message);

                    // (✓) FINAL FIX: Reverted to a two-query transaction.
                    // This is the most robust way to handle this when the macro fails on complex queries.
                    // The logic is simple for the macro to verify, and atomicity is guaranteed by the transaction.
                    if bet > 0
                        && let (Some(winner_id), Some(loser_id)) = (winner, loser)
                    {
                        let loser_db_id = loser_id.get() as i64;
                        let winner_db_id = winner_id.get() as i64;

                        let mut tx = match db.begin().await {
                            Ok(tx) => tx,
                            Err(e) => {
                                println!("[DB] Failed to begin transaction: {:?}", e);
                                return;
                            }
                        };

                        // Query 1: Subtract from loser
                        if let Err(e) = sqlx::query!(
                            "UPDATE profiles SET balance = balance - $1 WHERE user_id = $2",
                            bet,
                            loser_db_id
                        )
                        .execute(&mut *tx)
                        .await
                        {
                            println!("[DB] Failed to subtract balance for loser: {:?}", e);
                            tx.rollback().await.ok(); // Attempt to roll back
                            return;
                        }

                        // Query 2: Add to winner
                        if let Err(e) = sqlx::query!(
                            "UPDATE profiles SET balance = balance + $1 WHERE user_id = $2",
                            bet,
                            winner_db_id
                        )
                        .execute(&mut *tx)
                        .await
                        {
                            println!("[DB] Failed to add balance for winner: {:?}", e);
                            tx.rollback().await.ok(); // Attempt to roll back
                            return;
                        }

                        // If both queries succeeded, commit the transaction.
                        if let Err(e) = tx.commit().await {
                            println!("[DB] Failed to commit transaction: {:?}", e);
                        } else {
                            println!(
                                "[DB] Transferred {} from {} to {}",
                                bet, loser_id, winner_id
                            );
                        }
                    }

                    let (embed, _) = game.render();
                    let builder = EditMessage::new().embed(embed).components(vec![]);
                    if let Err(e) = interaction.message.edit(&ctx.http, builder).await {
                        println!("[GAME MANAGER] Error editing final message: {:?}", e);
                    }
                    self.remove_game(&interaction.message.id);
                }
                GameUpdate::NoOp => {}
            }
        }
    }
}
