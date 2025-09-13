//! Handles research refresh component.
use super::util::{defer_component, handle_global_nav};
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
    defer_component(ctx, component).await;
    if handle_global_nav(ctx, component, &_app_state, "saga").await {
        return;
    }
    if component.data.custom_id != "research_refresh" {
        return;
    }
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
