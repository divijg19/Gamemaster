//! This module contains the `run` functions for the Poker command.

use super::state::PokerGame;
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

pub fn register() -> CreateCommand {
    CreateCommand::new("poker")
        .description("Start a multiplayer game of Five Card Poker against the house.")
        .add_option(
            CreateCommandOption::new(
                CommandOptionType::Integer,
                "ante",
                "The mandatory ante bet to join the table.",
            )
            .required(true)
            .min_int_value(1),
        )
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    let response = CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new());
    if interaction
        .create_response(&ctx.http, response)
        .await
        .is_err()
    {
        println!("[POKER] Failed to defer slash command response.");
        return;
    }

    let ante = interaction
        .data
        .options
        .iter()
        .find_map(|opt| {
            if opt.name == "ante" {
                opt.value.as_i64()
            } else {
                None
            }
        })
        .unwrap_or(0); // This is safe because the option is required.

    let poker_game = PokerGame::new(Arc::new(interaction.user.clone()), ante);
    let (content, embed, components) = poker_game.render();
    let builder = serenity::builder::EditInteractionResponse::new()
        .content(content)
        .embed(embed)
        .components(components);

    if let Ok(game_msg) = interaction.edit_response(&ctx.http, builder).await {
        game_manager_lock
            .write()
            .await
            .start_game(game_msg.id, Box::new(poker_game));
        spawn_lobby_timeout_handler(ctx.clone(), game_manager_lock, game_msg);
    }
}

pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    let ante = match args.first().and_then(|arg| arg.parse::<i64>().ok()) {
        Some(bet) if bet > 0 => bet,
        _ => {
            msg.reply(
                ctx,
                "You must specify a valid ante amount to start a poker game.",
            )
            .await
            .ok();
            return;
        }
    };

    let poker_game = PokerGame::new(Arc::new(msg.author.clone()), ante);
    let (content, embed, components) = poker_game.render();
    let builder = CreateMessage::new()
        .content(content)
        .embed(embed)
        .components(components)
        .reference_message(msg);

    if let Ok(game_msg) = msg.channel_id.send_message(&ctx.http, builder).await {
        game_manager_lock
            .write()
            .await
            .start_game(game_msg.id, Box::new(poker_game));
        spawn_lobby_timeout_handler(ctx.clone(), game_manager_lock, game_msg);
    }
}

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
            && let Some(poker_game) = game_box.as_any().downcast_ref::<PokerGame>()
            && poker_game.is_in_lobby()
        {
            let embed = serenity::builder::CreateEmbed::new()
                .title("Poker Lobby Expired")
                .description("The game was not started by the host in time.")
                .color(0xFF0000); // Red
            let builder = EditMessage::new()
                .content("**Poker Lobby Expired**")
                .embed(embed)
                .components(vec![]);
            game_msg.edit(&ctx.http, builder).await.ok();
            manager.remove_game(&game_msg.id);
        }
    });
}
