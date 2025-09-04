//! This module contains the `run` functions for the Blackjack command.

use super::state::BlackjackGame;
use crate::AppState;
use crate::commands::games::engine::{Game, GameManager};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage, EditMessage,
};
use serenity::model::application::{CommandInteraction, CommandOptionType};
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::{instrument, warn};

/// Registers the `/blackjack` slash command with an optional bet.
pub fn register() -> CreateCommand {
    CreateCommand::new("blackjack")
        .description("Start a multiplayer game of Blackjack.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "bet",
                "Optional: The minimum bet for the table. Leave blank for a friendly game.",
            )
            .required(false)
            .min_int_value(1),
        )
}

/// Entry point for the `/blackjack` slash command.
#[instrument(level = "info", skip(ctx, interaction), fields(user_id = interaction.user.id.get()))]
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        warn!(command = "blackjack_slash", "missing_app_state");
        return;
    };
    let game_manager_lock = app_state.game_manager.clone();

    let response = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
    if interaction
        .create_response(&ctx.http, response)
        .await
        .is_err()
    {
        println!("[BJ] Failed to defer slash command response.");
        return;
    }

    // Bet is optional and defaults to 0 for a friendly game.
    let bet = interaction
        .data
        .options
        .iter()
        .find_map(|opt| {
            if opt.name == "bet" {
                opt.value.as_i64()
            } else {
                None
            }
        })
        .unwrap_or(0);

    let blackjack_game = BlackjackGame::new(Arc::new(interaction.user.clone()), bet);
    let (content, embed, components) = blackjack_game.render();

    let builder = serenity::builder::EditInteractionResponse::new()
        .content(content)
        .embed(embed)
        .components(components);

    if let Ok(game_msg) = interaction.edit_response(&ctx.http, builder).await {
        game_manager_lock
            .write()
            .await
            .start_game(game_msg.id, Box::new(blackjack_game));
        spawn_lobby_timeout_handler(ctx.clone(), game_manager_lock, game_msg);
    }
}

/// Entry point for the `!blackjack` prefix command.
#[instrument(level = "info", skip(ctx, msg, args), fields(user_id = msg.author.id.get()))]
pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        warn!(command = "blackjack_prefix", "missing_app_state");
        return;
    };
    let game_manager_lock = app_state.game_manager.clone();

    // Bet is optional for prefix commands as well.
    let bet = args
        .iter()
        .find_map(|arg| arg.parse::<i64>().ok())
        .unwrap_or(0);

    let blackjack_game = BlackjackGame::new(Arc::new(msg.author.clone()), bet);
    let (content, embed, components) = blackjack_game.render();

    let builder = CreateMessage::new()
        .content(content)
        .embed(embed)
        .components(components)
        .reference_message(msg);

    if let Ok(game_msg) = msg.channel_id.send_message(&ctx.http, builder).await {
        game_manager_lock
            .write()
            .await
            .start_game(game_msg.id, Box::new(blackjack_game));
        spawn_lobby_timeout_handler(ctx.clone(), game_manager_lock, game_msg);
    }
}

/// Spawns a task to handle the 2-minute lobby timeout, cleaning up inactive games.
fn spawn_lobby_timeout_handler(
    ctx: Context,
    game_manager: Arc<RwLock<GameManager>>,
    mut game_msg: Message,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;
        let mut manager = game_manager.write().await;

        // (✓) FIXED: Collapsed the nested `if` statements into a single, more readable block.
        if let Some(game_box) = manager.get_game_mut(&game_msg.id)
            && let Some(bj_game) = game_box.as_any().downcast_ref::<BlackjackGame>()
            && bj_game.is_in_lobby()
        {
            let embed = serenity::builder::CreateEmbed::new()
                .title("Blackjack Lobby Expired")
                .description("The game was not started by the host in time.")
                .color(0xFF0000); // Red

            let builder = EditMessage::new()
                .content("**Blackjack Lobby Expired**")
                .embed(embed)
                .components(vec![]);

            game_msg.edit(&ctx.http, builder).await.ok();
            manager.remove_game(&game_msg.id);
            println!(
                "[BJ] Lobby for game {} timed out and was removed.",
                game_msg.id
            );
        }
    });
}
