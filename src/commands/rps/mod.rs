use std::collections::HashMap;
use std::sync::Arc;

use serenity::model::application::ComponentInteraction;
use serenity::model::id::MessageId;
use serenity::prelude::*;
use tokio::sync::RwLock;

mod interactions;
mod run;
pub mod state;

use crate::commands::rps::state::GameState;
use interactions::*;
pub use run::run;

// DEFINITIVE FIX: This function now correctly accepts an owned Arc.
pub async fn handle_interaction(
    ctx: &Context,
    interaction: &mut ComponentInteraction,
    active_games: Arc<RwLock<HashMap<MessageId, GameState>>>,
) {
    let custom_id = interaction.data.custom_id.clone();
    let custom_id_parts: Vec<&str> = custom_id.split('_').collect();
    let action = custom_id_parts.get(1).unwrap_or(&"");

    // DEFINITIVE FIX: Pass a reference down to the handlers to satisfy the type checker.
    match *action {
        "accept" => handle_accept(ctx, interaction, &custom_id_parts, &active_games).await,
        "decline" => handle_decline(ctx, interaction, &custom_id_parts, &active_games).await,
        "prompt" => handle_prompt(ctx, interaction, &active_games).await,
        "move" => handle_move(ctx, interaction, &custom_id_parts, &active_games).await,
        _ => {}
    }
}
