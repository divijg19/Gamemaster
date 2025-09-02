//! Contains the run logic for the `/quests` command.

use super::ui;
use crate::{AppState, database};
// (✓) IMPROVED: Explicitly import UserId for clarity and robustness.
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::prelude::*;

/// Shared logic to fetch quest data and generate the response.
async fn get_quests_response(
    ctx: &Context,
    user_id: UserId,
) -> Result<(CreateEmbed, Vec<CreateActionRow>), String> {
    let db = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .db
            .clone()
    };

    match database::quests::get_or_refresh_quest_board(&db, user_id).await {
        Ok(quests) => Ok(ui::create_quest_board_embed(&quests)),
        Err(e) => {
            println!(
                "[DB ERROR] Failed to get quest board for {}: {:?}",
                user_id, e
            );
            Err("❌ Could not retrieve the quest board. Please try again later.".to_string())
        }
    }
}

/// The entry point for the slash command `/quests`.
pub async fn run_slash(ctx: &Context, interaction: &CommandInteraction) {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Defer(
                CreateInteractionResponseMessage::new().ephemeral(true),
            ),
        )
        .await
        .ok();

    let builder = match get_quests_response(ctx, interaction.user.id).await {
        Ok((embed, components)) => EditInteractionResponse::new()
            .embed(embed)
            .components(components),
        Err(content) => EditInteractionResponse::new().content(content),
    };

    interaction.edit_response(&ctx.http, builder).await.ok();
}

/// The entry point for the prefix command `!quests`.
pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    match get_quests_response(ctx, msg.author.id).await {
        Ok((embed, components)) => {
            msg.channel_id
                .send_message(
                    &ctx.http,
                    serenity::all::CreateMessage::new()
                        .embed(embed)
                        .components(components)
                        .reference_message(msg),
                )
                .await
                .ok();
        }
        Err(content) => {
            msg.channel_id.say(&ctx.http, content).await.ok();
        }
    };
}
