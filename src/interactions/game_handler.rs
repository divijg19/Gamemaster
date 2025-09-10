//! Handles all component interactions that are managed by the generic `GameManager`.
//! This includes RPS, Blackjack, Poker, the Shop, and Battles.

use crate::AppState;
use crate::commands::games::{Game, GameManager};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = &app_state.db;
    let mut game_manager = app_state.game_manager.write().await;
    game_manager.on_interaction(ctx, component, db).await;
}

// (✓) NEW: Add the missing `start_new_game` function required by quest_handler.
/// A shared helper function to start a new game session and send the initial response.
pub async fn start_new_game(
    ctx: &Context,
    component: &mut ComponentInteraction,
    game_manager: Arc<RwLock<GameManager>>,
    game: Box<dyn Game + Send + Sync>,
    initial_content: &str,
) {
    let (content, embed, components) = game.render();
    let final_content = format!("{}\n{}", initial_content, content);
    let builder = EditInteractionResponse::new()
        .content(final_content)
        .embed(embed)
        .components(components);

    if let Ok(msg) = component.edit_response(&ctx.http, builder).await {
        game_manager.write().await.start_game(msg.id, game);
    }
}
