//! Contains the run logic for the `/questlog` command.

use super::ui;
use crate::AppState;
use crate::database;
use crate::database::models::PlayerQuestStatus;
use serenity::builder::{
    CreateActionRow, CreateEmbed, CreateInteractionResponse, CreateInteractionResponseMessage,
    EditInteractionResponse,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::prelude::*;

/// Shared logic to fetch quest data and generate the response for the quest log.
pub async fn get_questlog_response(
    ctx: &Context,
    user_id: UserId,
    status: PlayerQuestStatus, // e.g., Active or Completed
) -> Result<(CreateEmbed, Vec<CreateActionRow>), String> {
    let db = {
        let data = ctx.data.read().await;
        data.get::<AppState>()
            .expect("Expected AppState in TypeMap.")
            .db
            .clone()
    };

    // Use our powerful new database function to get all the necessary data.
    match database::quests::get_player_quests_with_details(&db, user_id, status).await {
        Ok(quests) => Ok(ui::create_questlog_embed(&quests, status)),
        Err(e) => {
            println!(
                "[DB ERROR] Failed to get quest log for {}: {:?}",
                user_id, e
            );
            Err("❌ Could not retrieve your quest log. Please try again later.".to_string())
        }
    }
}

/// The entry point for the slash command `/questlog`.
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

    // By default, show the "Accepted" (active) quests.
    let builder =
        match get_questlog_response(ctx, interaction.user.id, PlayerQuestStatus::Accepted).await {
            Ok((embed, components)) => EditInteractionResponse::new()
                .embed(embed)
                .components(components),
            Err(content) => EditInteractionResponse::new().content(content),
        };

    interaction.edit_response(&ctx.http, builder).await.ok();
}

/// The entry point for the prefix command `!questlog`.
pub async fn run_prefix(ctx: &Context, msg: &Message, _args: Vec<&str>) {
    // By default, show the "Accepted" (active) quests.
    match get_questlog_response(ctx, msg.author.id, PlayerQuestStatus::Accepted).await {
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
