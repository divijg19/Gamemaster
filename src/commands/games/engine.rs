//! This module contains the core, generic game engine components.
//! It defines the `Game` trait that all games must implement, and the
//! `GameManager` which tracks and routes interactions for all active games.

use serenity::async_trait;
use serenity::builder::{CreateActionRow, CreateEmbed, EditMessage};
use serenity::model::application::ComponentInteraction;
use serenity::model::id::MessageId;
use serenity::prelude::Context;
use std::any::Any;
use std::collections::HashMap;

/// Represents the outcome of a player's interaction with a game.
/// This enum is returned by `handle_interaction` to tell the `GameManager` what to do next.
pub enum GameUpdate {
    /// The game state has changed and the message needs to be re-rendered.
    ReRender,
    /// The game is over and should be removed from the active games list.
    GameOver(String),
    /// No change occurred that requires a view update (e.g., an invalid action).
    NoOp,
}

/// The core `Game` trait. Every game you create (Blackjack, Poker, RPS, etc.) must implement this.
/// This allows the `GameManager` to handle any game in a generic, uniform way.
#[async_trait]
pub trait Game: Send + Sync {
    /// Allows for safe, dynamic downcasting from a `Box<dyn Game>` to a concrete game type.
    /// This is essential for game-specific logic, like timeouts.
    fn as_any(&self) -> &dyn Any;
    #[allow(dead_code)]
    fn as_any_mut(&mut self) -> &mut dyn Any;

    /// Handles a component interaction (e.g., a button press) from a player.
    async fn handle_interaction(
        &mut self,
        ctx: &Context,
        interaction: &mut ComponentInteraction,
    ) -> GameUpdate;

    /// Renders the current state of the game into its component parts for display.
    fn render(&self) -> (CreateEmbed, Vec<CreateActionRow>);
}

/// The GameManager is the central state machine responsible for all active game instances.
pub struct GameManager {
    active_games: HashMap<MessageId, Box<dyn Game>>,
}

impl GameManager {
    /// Creates a new, empty GameManager.
    pub fn new() -> Self {
        Self {
            active_games: HashMap::new(),
        }
    }

    /// Adds a new game instance to the manager, associated with its Discord message ID.
    pub fn start_game(&mut self, message_id: MessageId, game: Box<dyn Game>) {
        self.active_games.insert(message_id, game);
    }

    /// Gets a mutable reference to an active game, if one exists for the given message ID.
    pub fn get_game_mut(&mut self, message_id: &MessageId) -> Option<&mut Box<dyn Game>> {
        self.active_games.get_mut(message_id)
    }

    /// Removes a game from the manager, ending its lifecycle.
    pub fn remove_game(&mut self, message_id: &MessageId) {
        self.active_games.remove(message_id);
    }

    /// The main event router for all in-game interactions.
    /// It finds the correct game instance and delegates the interaction handling to it.
    pub async fn on_interaction(&mut self, ctx: &Context, interaction: &mut ComponentInteraction) {
        if let Some(game) = self.get_game_mut(&interaction.message.id) {
            match game.handle_interaction(ctx, interaction).await {
                GameUpdate::ReRender => {
                    let (embed, components) = game.render();
                    let builder = EditMessage::new().embed(embed).components(components);
                    if let Err(e) = interaction.message.edit(&ctx.http, builder).await {
                        println!("[GAME MANAGER] Error editing game message: {:?}", e);
                    }
                }
                GameUpdate::GameOver(final_message) => {
                    self.remove_game(&interaction.message.id);
                    println!("[GAME MANAGER] Game over: {}", final_message);
                    // In the future, you could edit the message one last time here to show the final result.
                }
                GameUpdate::NoOp => {}
            }
        }
    }
}
