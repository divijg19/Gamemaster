//! Handles the command logic for `/shop`.

use super::game::ShopGame;
use super::state::ShopSession;
use crate::AppState;
use crate::commands::games::engine::{Game, GameManager};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub fn register() -> CreateCommand {
    CreateCommand::new("shop").description("Buy and sell items.")
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
        return;
    }

    let session = ShopSession {
        user_id: interaction.user.id.get(),
        current_category: crate::commands::economy::core::item::ItemCategory::Resource,
        current_page: 0,
    };
    let shop_game = ShopGame { session };

    let (content, embed, components) = shop_game.render();
    let builder = serenity::builder::EditInteractionResponse::new()
        .content(content)
        .embed(embed)
        .components(components);

    if let Ok(game_msg) = interaction.edit_response(&ctx.http, builder).await {
        game_manager_lock
            .write()
            .await
            .start_game(game_msg.id, Box::new(shop_game));
        spawn_session_timeout_handler(ctx.clone(), game_manager_lock, game_msg);
    }
}

/// (✓) ADDED: A prefix command handler for the shop.
pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let game_manager_lock = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .game_manager
            .clone()
    };

    let session = ShopSession {
        user_id: msg.author.id.get(),
        current_category: crate::commands::economy::core::item::ItemCategory::Resource,
        current_page: 0,
    };
    let shop_game = ShopGame { session };

    let (content, embed, components) = shop_game.render();
    let builder = CreateMessage::new()
        .content(content)
        .embed(embed)
        .components(components)
        .reference_message(msg);

    if let Ok(game_msg) = msg.channel_id.send_message(&ctx.http, builder).await {
        game_manager_lock
            .write()
            .await
            .start_game(game_msg.id, Box::new(shop_game));
        spawn_session_timeout_handler(ctx.clone(), game_manager_lock, game_msg);
    }
}

fn spawn_session_timeout_handler(
    ctx: Context,
    game_manager: Arc<RwLock<GameManager>>,
    mut game_msg: Message,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(120)).await;
        let mut manager = game_manager.write().await;

        if manager.get_game_mut(&game_msg.id).is_some() {
            let embed = serenity::builder::CreateEmbed::new()
                .title("Shop Session Expired")
                .description("Your shop session has timed out due to inactivity.")
                .color(0xFF0000); // Red
            let builder = EditMessage::new().embed(embed).components(vec![]);
            game_msg.edit(&ctx.http, builder).await.ok();
            manager.remove_game(&game_msg.id);
        }
    });
}
