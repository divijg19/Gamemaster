//! This module contains the `run` functions for the Blackjack command.
//! Its only job is to create a new game lobby and register it with the GameManager.

use super::game::BlackjackGame;
use crate::AppState;
use crate::commands::games::Game;
use serenity::builder::{
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::sync::Arc;

/// Entry point for the `/blackjack` slash command.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    // 1. Create a new Blackjack game instance with the command author as the host.
    let blackjack_game = BlackjackGame::new(Arc::new(interaction.user.clone()));

    // 2. Render the initial state (the "waiting for players" lobby).
    let (embed, components) = blackjack_game.render();

    // 3. Send the initial game message.
    let builder = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components);
    let response = CreateInteractionResponse::Message(builder);
    if let Err(e) = interaction.create_response(&ctx.http, response).await {
        println!("[BJ] Error sending slash command response: {:?}", e);
        return;
    }
    let game_msg = match interaction.get_response(&ctx.http).await {
        Ok(msg) => msg,
        Err(e) => {
            println!("[BJ] Error getting interaction response message: {:?}", e);
            return;
        }
    };

    // 4. Register the new game with the GameManager.
    game_manager_lock
        .write()
        .await
        .start_game(game_msg.id, Box::new(blackjack_game));
}

/// Entry point for the `!blackjack` prefix command.
pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    // 1. Create a new Blackjack game instance with the command author as the host.
    let blackjack_game = BlackjackGame::new(Arc::new(msg.author.clone()));

    // 2. Render the initial state (the lobby).
    let (embed, components) = blackjack_game.render();

    // 3. Send the initial game message.
    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    let game_msg = match msg.channel_id.send_message(&ctx.http, builder).await {
        Ok(m) => m,
        Err(e) => {
            println!("[BJ] Error sending initial game message: {:?}", e);
            return;
        }
    };

    // 4. Register the new game with the GameManager.
    game_manager_lock
        .write()
        .await
        .start_game(game_msg.id, Box::new(blackjack_game));
}
