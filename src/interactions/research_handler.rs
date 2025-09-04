//! Handles research refresh component.
use crate::AppState;
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use tracing::instrument;

#[instrument(level = "info", skip(ctx, component, _app_state))]
pub async fn handle(
    ctx: &Context,
    component: &mut ComponentInteraction,
    _app_state: Arc<AppState>,
) {
    if component.data.custom_id != "research_refresh" {
        return;
    }
    component.defer_ephemeral(&ctx.http).await.ok();
    let Some(state) = AppState::from_ctx(ctx).await else {
        return;
    };
    // rebuild research view using cached variant
    let embed = crate::commands::research::run::build_view_cached(&state, component.user.id).await;
    component
        .edit_response(&ctx.http, EditInteractionResponse::new().embed(embed))
        .await
        .ok();
}
