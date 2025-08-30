//! Handles all component interactions that are managed by the generic `GameManager`.
//! This includes RPS, Blackjack, Poker, the Shop, and Battles.

use crate::AppState;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    // This logic is moved directly from the old handler.rs file.
    // It's the central point for all "game" sessions.
    let db = app_state.db.clone();
    let mut game_manager = app_state.game_manager.write().await;
    game_manager.on_interaction(ctx, component, &db).await;
}
