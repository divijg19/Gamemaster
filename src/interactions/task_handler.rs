//! Handles all component interactions for the `/tasks` command family.

use crate::commands::economy::core::item::Item;
use crate::commands::tasks::ui;
use crate::{AppState, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use super::util::{defer_component, handle_global_nav, edit_component};

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = &app_state.db;
    defer_component(ctx, component).await;
    if handle_global_nav(ctx, component, &app_state, "saga").await { return; }

    // The custom_id is expected to be "task_claim_{player_task_id}"
    let custom_id = &component.data.custom_id;
    let player_task_id_str = match custom_id.strip_prefix("task_claim_") {
        Some(id) => id,
        None => return, // Not a task claim button, ignore it.
    };

    let player_task_id: i32 = match player_task_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            let builder =
                EditInteractionResponse::new().content("❌ Error: Invalid task ID in button.");
            component.edit_response(&ctx.http, builder).await.ok();
            return;
        }
    };

    // Attempt to claim the reward from the database.
    let result = database::tasks::claim_task_reward(db, component.user.id, player_task_id).await;

    // (✓) FIXED: Directly assign `response_message` from the match block to resolve the unused assignment warning.
    let response_message = match result {
        Ok((coins, item_id, quantity)) => {
            let mut rewards = Vec::new();
            if coins > 0 {
                rewards.push(format!("💰 **{}** coins", coins));
            }
            if let (Some(id), Some(qty)) = (item_id, quantity)
                && let Some(item) = Item::from_i32(id)
            {
                rewards.push(format!(
                    "{} **{}x** {}",
                    item.emoji(),
                    qty,
                    item.display_name()
                ));
            }
            // Add a default message if for some reason there are no rewards.
            if rewards.is_empty() {
                "🎉 **Quest Complete!**".to_string()
            } else {
                format!(
                    "🎉 **Reward Claimed!**\nYou received: {}.",
                    rewards.join(", ")
                )
            }
        }
        Err(e) => {
            format!("⚠️ **Claim Failed:** {}", e)
        }
    };

    // After claiming, always re-fetch the tasks to show the updated UI.
    // This removes the button for the just-claimed task.
    let mut builder = EditInteractionResponse::new();
    match database::tasks::get_or_assign_player_tasks(db, component.user.id).await {
        Ok(tasks) => {
            let (embed, components) = ui::create_tasks_embed(&tasks);
            // Prepend the result message to the top of the interaction response.
            builder = builder
                .content(response_message)
                .embed(embed)
                .components(components);
        }
        Err(_) => {
            // If re-fetching fails, just show the claim result message.
            builder = builder.content(format!(
                "{}\n\nCould not refresh task list. Please run `/tasks` again.",
                response_message
            ));
        }
    };

    edit_component(ctx, component, "tasks.claim", builder).await;
}
