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

/// (✓) ADDED: Represents a single player's win or loss.
/// This is flexible enough for both 1v1 and multiplayer games.
#[derive(Debug, Clone)]
pub struct GamePayout {
    pub user_id: UserId,
    pub amount: i64, // Positive for win, negative for loss, zero for push/tie.
}

/// (✓) MODIFIED: The GameOver variant is now unified to handle all game types.
/// It contains a list of payouts, making it suitable for everything from RPS to multi-hand Blackjack.
pub enum GameUpdate {
    ReRender,
    GameOver {
        message: String,
        payouts: Vec<GamePayout>,
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
                // (✓) MODIFIED: This single arm now handles game over for RPS, Blackjack, etc.
                GameUpdate::GameOver { message, payouts } => {
                    println!("[GAME MANAGER] Game over: {}", message);

                    // (✓) OPTIMAL: All database updates are performed in a single, atomic transaction.
                    if !payouts.is_empty() {
                        let mut tx = match db.begin().await {
                            Ok(tx) => tx,
                            Err(e) => {
                                println!("[DB] Failed to begin transaction: {:?}", e);
                                // Don't return yet, still need to update the message.
                                return;
                            }
                        };

                        for payout in &payouts {
                            // Skip users who broke even to avoid unnecessary DB calls.
                            if payout.amount == 0 {
                                continue;
                            }

                            if let Err(e) = sqlx::query!(
                                "UPDATE profiles SET balance = balance + $1 WHERE user_id = $2",
                                payout.amount,
                                payout.user_id.get() as i64
                            )
                            .execute(&mut *tx)
                            .await
                            {
                                println!(
                                    "[DB] Failed to process payout for {}: {:?}. Rolling back.",
                                    payout.user_id, e
                                );
                                tx.rollback().await.ok();
                                // Don't return, as we must update the Discord message.
                                return;
                            }
                        }

                        if let Err(e) = tx.commit().await {
                            println!("[DB] Failed to commit transaction: {:?}", e);
                        } else {
                            println!("[DB] Successfully processed {} payouts.", payouts.len());
                        }
                    }

                    // Render the final game state and remove all buttons.
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
