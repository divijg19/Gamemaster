//! This module contains the `run` functions for the Blackjack command.

use super::game::BlackjackGame;
use crate::AppState;
use crate::commands::games::engine::{Game, GameManager};
// (✓) MODIFIED: Removed unused `CreateEmbed` import for cleanliness.
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

/// Registers the `/blackjack` slash command with a mandatory bet.
pub fn register() -> CreateCommand {
    CreateCommand::new("blackjack")
        .description("Start a multiplayer game of Blackjack.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "bet",
                "The amount each player will bet.",
            )
            .required(true)
            .min_int_value(1),
        )
}

/// Entry point for the `/blackjack` slash command.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    // Defer the response immediately to prevent "interaction failed" errors.
    let response = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
    if let Err(e) = interaction.create_response(&ctx.http, response).await {
        println!("[BJ] Failed to defer slash command response: {:?}", e);
        return;
    }

    // Safely get the required 'bet' option.
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
        .unwrap_or(0); // Should always exist due to `required(true)`.

    // This check is a fallback; Discord's `min_int_value` should prevent this.
    if bet <= 0 {
        let builder = serenity::builder::EditInteractionResponse::new()
            .content("You must provide a valid bet greater than 0.");
        interaction.edit_response(&ctx.http, builder).await.ok();
        return;
    }

    // TODO: Add database logic here to check if the host can afford the bet.

    // Create and render the initial game lobby.
    let blackjack_game = BlackjackGame::new(Arc::new(interaction.user.clone()), bet);
    let (embed, components) = blackjack_game.render();

    // Edit the original "thinking..." response to show the lobby.
    let builder = serenity::builder::EditInteractionResponse::new()
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
pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    let bet = args
        .iter()
        .find_map(|arg| arg.parse::<i64>().ok())
        .unwrap_or(0);
    if bet <= 0 {
        msg.reply(ctx, "You must specify a valid bet amount.")
            .await
            .ok();
        return;
    }

    // TODO: Add database logic here to check if the host can afford the bet.

    let blackjack_game = BlackjackGame::new(Arc::new(msg.author.clone()), bet);
    let (embed, components) = blackjack_game.render();
    let builder = CreateMessage::new()
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

        if let Some(game_box) = manager.get_game_mut(&game_msg.id) {
            // Downcast to the specific BlackjackGame type to access its state.
            if let Some(bj_game) = game_box.as_any().downcast_ref::<BlackjackGame>() {
                // Only remove the game if it's still in the lobby phase.
                if bj_game.is_in_lobby() {
                    let embed = serenity::builder::CreateEmbed::new()
                        .title("Blackjack Lobby Expired")
                        .description("The game was not started by the host in time.")
                        .color(0xFF0000); // Red
                    let builder = EditMessage::new().embed(embed).components(vec![]);

                    game_msg.edit(&ctx.http, builder).await.ok();
                    manager.remove_game(&game_msg.id);
                    println!(
                        "[BJ] Lobby for game {} timed out and was removed.",
                        game_msg.id
                    );
                }
            }
        }
    });
}
