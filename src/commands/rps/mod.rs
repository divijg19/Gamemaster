// 1. Declare the existence of our internal modules.
mod interactions;
mod run;
pub mod state;

// 2. Import necessary types for handling the game state.
use std::collections::HashMap;
use std::sync::Arc;

use serenity::model::application::ComponentInteraction;
use serenity::model::id::MessageId;
use serenity::prelude::*;
use tokio::sync::RwLock;

// 3. Import the public functions from our modules.
use interactions::*;
pub use run::run;

use crate::commands::rps::state::GameState;

// This is the main router for the RPS command's interactions.
// It now accepts the active_games state.
pub async fn handle_interaction(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    active_games: &Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    let custom_id = interaction.data.custom_id.clone();
    let custom_id_parts: Vec<&str> = custom_id.split('_').collect();
    let action = custom_id_parts.get(1).unwrap_or(&"");

    // The active_games state is now passed down to the specific handlers.
    match *action {
        "accept" => handle_accept(ctx, interaction, &custom_id_parts, active_games).await,
        "decline" => handle_decline(ctx, interaction, &custom_id_parts, active_games).await,
        "prompt" => handle_prompt(ctx, interaction, active_games).await,
        "move" => handle_move(ctx, interaction, &custom_id_parts, active_games).await,
        _ => {}
    }
}
