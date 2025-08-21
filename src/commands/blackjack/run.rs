//! This module contains the `run` functions for the Blackjack command,
//! handling both prefix and slash command invocations to start a new game.

use super::game::BlackjackGame;
use crate::AppState;
use crate::commands::games::Game; // (✓) The `Game` trait is needed for the `.render()` method.
use serenity::builder::{
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;
// (✓) CORRECTED: Removed the unused `GameManager` and synchronization imports.
// They are not needed here because we access the manager through `AppState`.

/// Entry point for the `/blackjack` slash command.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    // 1. Create a new instance of the Blackjack game.
    let blackjack_game = BlackjackGame::new();

    // 2. Render its initial state to get the embed and buttons.
    let (embed, components) = blackjack_game.render();

    // 3. Send the initial game message as the response to the interaction.
    let builder = CreateInteractionResponseMessage::new()
        .embed(embed)
        .components(components);
    let response = CreateInteractionResponse::Message(builder);
    if let Err(e) = interaction.create_response(&ctx.http, response).await {
        println!("[BJ] Error sending slash command response: {:?}", e);
        return;
    }

    // 4. Get the message we just sent so we have its ID.
    let game_msg = match interaction.get_response(&ctx.http).await {
        Ok(msg) => msg,
        Err(e) => {
            println!("[BJ] Error getting interaction response message: {:?}", e);
            return;
        }
    };

    // 5. Register the new game with the GameManager, linking it to the message ID.
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

    // 1. Create a new instance of the Blackjack game.
    let blackjack_game = BlackjackGame::new();

    // 2. Render its initial state.
    let (embed, components) = blackjack_game.render();

    // 3. Send the initial game message as a reply.
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
