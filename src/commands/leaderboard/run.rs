//! Implements the run logic for the `/leaderboard` command.

use super::ui::{create_leaderboard_buttons, create_leaderboard_embed};
// (✓) FIXED: Corrected the path to the LeaderboardType enum.
use crate::saga::leaderboard::LeaderboardType;
use crate::{AppState, database};
use serenity::builder::{
    CreateCommand, CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage,
    EditInteractionResponse,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::*;

const LEADERBOARD_LIMIT: i64 = 10;

pub fn register() -> CreateCommand {
    CreateCommand::new("leaderboard").description("View the server leaderboards.")
}

pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(CreateInteractionResponseMessage::new()),
        )
        .await
        .ok();

    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    let board_type = LeaderboardType::Gamemaster;
    let entries = database::leaderboard::get_gamemaster_leaderboard(&pool, LEADERBOARD_LIMIT)
        .await
        .unwrap_or_default();

    let embed = create_leaderboard_embed(ctx, &entries, board_type).await;
    let components = vec![create_leaderboard_buttons(board_type)];

    let builder = EditInteractionResponse::new()
        .embed(embed)
        .components(components);

    interaction.edit_response(&ctx.http, builder).await.ok();
}

pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    let Some(app_state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let pool = app_state.db.clone();

    let board_type = LeaderboardType::Gamemaster;
    let entries = database::leaderboard::get_gamemaster_leaderboard(&pool, LEADERBOARD_LIMIT)
        .await
        .unwrap_or_default();

    let embed = create_leaderboard_embed(ctx, &entries, board_type).await;
    let components = vec![create_leaderboard_buttons(board_type)];

    let builder = CreateMessage::new()
        .embed(embed)
        .components(components)
        .reference_message(msg);
    msg.channel_id.send_message(&ctx.http, builder).await.ok();
}
