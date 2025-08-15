// 1. Declare the existence of our internal modules.
mod interactions;
mod run;
pub mod state;

// 2. Import the public functions.
// CORRECTED: `use run::*` is removed as it's not needed.
use interactions::*;

// 3. Re-export the public functions so `handler.rs` can use them.
pub use run::run;

// This is the main router for the RPS command.
pub async fn handle_interaction(
    ctx: &serenity::prelude::Context,
    interaction: &mut serenity::model::application::ComponentInteraction,
) {
    let custom_id = interaction.data.custom_id.clone();
    let custom_id_parts: Vec<&str> = custom_id.split('_').collect();
    let action = custom_id_parts.get(1).unwrap_or(&"");

    match *action {
        "accept" => handle_accept(ctx, interaction, &custom_id_parts).await,
        "decline" => handle_decline(ctx, interaction, &custom_id_parts).await,
        "prompt" => handle_prompt(ctx, interaction).await,
        "move" => handle_move(ctx, interaction, &custom_id_parts).await,
        _ => {}
    }
}
