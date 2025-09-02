//! This module implements the `ping` command in both prefix and slash command formats.
//! It is used to check the bot's heartbeat latency to the Discord gateway.

use crate::ShardManagerContainer;
// (✓) FIXED: Import CreateCommand for the register function.
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

// (✓) NEW: Add a register function for the slash command.
pub fn register() -> CreateCommand {
    CreateCommand::new("ping").description("Checks the bot's latency.")
}

/// A helper function to get the bot's current shard latency.
/// This logic is shared between both the prefix and slash command handlers.
async fn get_latency_string(ctx: &Context) -> String {
    // Get the shard manager from the global context data.
    let data = ctx.data.read().await;
    let shard_manager = match data.get::<ShardManagerContainer>() {
        Some(manager) => manager,
        None => return "Error: Could not retrieve shard manager.".to_string(),
    };

    // Get the runner for the current shard.
    let runners = shard_manager.runners.lock().await;
    let runner = match runners.get(&ctx.shard_id) {
        Some(runner) => runner,
        None => return "Error: Could not find runner for this shard.".to_string(),
    };

    // Calculate latency and format it into a user-friendly string.
    let latency = runner.latency.map_or_else(
        || "N/A".to_string(),
        |latency| format!("{:.2} ms", latency.as_millis()),
    );

    format!("Pong! Heartbeat Latency: `{}`", latency)
}

/// The entry point for the prefix command `!ping`.
pub async fn run_prefix(ctx: &Context, msg: &Message) {
    let response_content = get_latency_string(ctx).await;

    if let Err(why) = msg.channel_id.say(&ctx.http, response_content).await {
        println!("Error sending ping prefix response: {:?}", why);
    }
}

/// The entry point for the slash command `/ping`.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let response_content = get_latency_string(ctx).await;

    // For slash commands, we build a specific response structure.
    let response_builder = CreateInteractionResponseMessage::new().content(response_content);
    let interaction_response = CreateInteractionResponse::Message(response_builder);

    if let Err(why) = interaction
        .create_response(&ctx.http, interaction_response)
        .await
    {
        println!("Error sending ping slash response: {:?}", why);
    }
}
