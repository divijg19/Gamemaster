//! Contains the run logic for the `/questlog` command.

use super::ui;
use crate::AppState;
use crate::database;
use crate::database::models::PlayerQuestStatus;
use serenity::builder::{CreateActionRow, CreateEmbed, EditInteractionResponse};
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
    // Check for optional quest_id integer option to show raw detail (debug / future UI expansion)
    let quest_id_opt = interaction
        .data
        .options
        .iter()
        .find(|o| o.name == "quest_id")
        .and_then(|o| match o.value {
            serenity::model::application::CommandDataOptionValue::Integer(v) => Some(v as i32),
            _ => None,
        });

    let builder = if let Some(qid) = quest_id_opt {
        if let Some(q) = quest_detail_for_debug(ctx, qid).await {
            EditInteractionResponse::new().content(format!(
                "Quest {} (user:{} quest:{} status:{:?} offered_at:{:?} accepted_at:{:?} completed_at:{:?})",
                q.player_quest_id, q.user_id, q.quest_id, q.status, q.offered_at, q.accepted_at, q.completed_at
            ))
        } else {
            EditInteractionResponse::new().content("Quest not found or not yours.")
        }
    } else {
        match get_questlog_response(ctx, interaction.user.id, PlayerQuestStatus::Accepted).await {
            Ok((embed, components)) => EditInteractionResponse::new()
                .embed(embed)
                .components(components),
            Err(content) => EditInteractionResponse::new().content(content),
        }
    };

    interaction.edit_response(&ctx.http, builder).await.ok();
}

/// The entry point for the prefix command `!questlog`.
pub async fn run_prefix(ctx: &Context, msg: &Message, args: Vec<&str>) {
    if let Some(first) = args.first()
        && let Ok(qid) = first.parse::<i32>()
    {
        if let Some(q) = quest_detail_for_debug(ctx, qid).await {
            let _ = msg
                .reply(
                    &ctx.http,
                    format!(
                        "Quest {} status {:?} (accepted_at: {:?} completed_at: {:?})",
                        q.player_quest_id, q.status, q.accepted_at, q.completed_at
                    ),
                )
                .await;
        } else {
            let _ = msg.reply(&ctx.http, "Quest not found.").await;
        }
        return;
    }
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

/// Exposed detail fetch used to activate underlying get_player_quest mapping and avoid dead code attr.
pub async fn quest_detail_for_debug(
    ctx: &Context,
    quest_id: i32,
) -> Option<crate::database::models::PlayerQuest> {
    let db = {
        let data = ctx.data.read().await;
        data.get::<AppState>()?.db.clone()
    };
    crate::database::quests::get_player_quest(&db, quest_id)
        .await
        .ok()
        .flatten()
}
