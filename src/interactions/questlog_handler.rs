//! Handles all component interactions for the `/questlog` command family.

use crate::AppState;
use crate::commands::questlog::run::get_questlog_response;
use crate::database::models::PlayerQuestStatus;
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

/// The main entry point for questlog-related component interactions.
pub async fn handle(
    ctx: &Context,
    component: &mut ComponentInteraction,
    _app_state: Arc<AppState>, // _app_state is kept for signature consistency with other handlers
) {
    // The custom_id is expected to be "questlog_view_{Status}"
    let custom_id = &component.data.custom_id;
    let view_status_str = match custom_id.strip_prefix("questlog_view_") {
        Some(s) => s,
        None => return, // Not a questlog button, ignore it.
    };

    // Parse the status string from the button ID into our enum.
    let status_to_view = match view_status_str {
        "Accepted" => PlayerQuestStatus::Accepted,
        "Completed" => PlayerQuestStatus::Completed,
        // This case should not be reachable with correctly generated buttons.
        _ => {
            let builder =
                EditInteractionResponse::new().content("❌ Error: Invalid view requested.");
            component.edit_response(&ctx.http, builder).await.ok();
            return;
        }
    };

    // Defer the interaction to show a loading state.
    component.defer_ephemeral(&ctx.http).await.ok();

    // We can reuse the exact same response logic from the main command.
    // This is a great example of well-architected, reusable code.
    let builder = match get_questlog_response(ctx, component.user.id, status_to_view).await {
        Ok((embed, components)) => EditInteractionResponse::new()
            .embed(embed)
            .components(components),
        Err(content) => EditInteractionResponse::new().content(content),
    };

    // Edit the original message with the new view (Active or Completed).
    component.edit_response(&ctx.http, builder).await.ok();
}
