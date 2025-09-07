//! Handles interactive contract drafting / acceptance via component interactions.
use crate::{AppState, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use super::util::{defer_component, handle_global_nav, edit_component};
use tracing::instrument;

#[instrument(level="info", skip(ctx, component, app_state), fields(user_id = component.user.id.get(), custom_id = %component.data.custom_id))]
pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let cid = component.data.custom_id.as_str();
    if !(cid.starts_with("contracts_")) {
        return;
    }
    defer_component(ctx, component).await;
    if handle_global_nav(ctx, component, &app_state, "saga").await { return; }
    let Some(state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let _db = &state.db; // kept for direct draft/accept operations (underscore to silence unused warning)
    // Draft select
    if cid == "contracts_refresh" {
        refresh(ctx, component, &app_state).await;
        return;
    }
    if cid == "contracts_draft_select" {
        if let Some(id) = parse_first_select(component) {
            draft(ctx, component, &app_state, id).await;
        }
        return;
    }
    if cid == "contracts_accept_select" {
        if let Some(id) = parse_first_select(component) {
            accept(ctx, component, &app_state, id).await;
        }
        return;
    }
    if let Some(rest) = cid.strip_prefix("contracts_page_") {
        if let Ok(page) = rest.parse::<usize>() {
            page_nav(ctx, component, &app_state, page).await;
        }
        return;
    }
}

fn parse_first_select(component: &ComponentInteraction) -> Option<i32> {
    if let serenity::model::application::ComponentInteractionDataKind::StringSelect { values } =
        &component.data.kind
    {
        values.first()?.parse().ok()
    } else {
        None
    }
}

async fn refresh(ctx: &Context, component: &mut ComponentInteraction, app_state: &AppState) {
    // Touch get_encounter for the first unit on page to keep helper active (lightweight optional)
    if let Ok(rows) =
        database::human::list_contract_status_cached(app_state, component.user.id).await
        && let Some((unit, _d, _r, _dr, _rec, _last)) = rows.first()
    {
        let _ = database::human::get_encounter(&app_state.db, component.user.id, unit.unit_id)
            .await
            .ok();
    }
    let (_desc, embed, comps) = load_embed(app_state, component.user.id, 0).await;
    edit_component(ctx, component, "contracts.refresh", EditInteractionResponse::new().embed(embed).components(comps)).await;
}
async fn page_nav(
    ctx: &Context,
    component: &mut ComponentInteraction,
    app_state: &AppState,
    page: usize,
) {
    if let Ok(rows) =
        database::human::list_contract_status_cached(app_state, component.user.id).await
    {
        let drafted =
            crate::database::human::list_drafted_contracts(&app_state.db, component.user.id)
                .await
                .unwrap_or_default();
        let legacy =
            crate::database::human::list_legacy_open_offers(&app_state.db, component.user.id)
                .await
                .unwrap_or_default();
        let view =
            crate::commands::contracts::run::build_contracts_embed(&rows, &drafted, &legacy, page);
    edit_component(ctx, component, "contracts.page", EditInteractionResponse::new().embed(view.embed).components(view.components)).await;
    }
}
async fn draft(
    ctx: &Context,
    component: &mut ComponentInteraction,
    app_state: &AppState,
    unit_id: i32,
) {
    let feedback =
        match database::human::draft_contract(&app_state.db, component.user.id, unit_id).await {
            Ok(_) => format!("Drafted contract for {}", unit_id),
            Err(e) => format!("Draft failed: {}", e),
        };
    app_state.invalidate_user_caches(component.user.id).await;
    let (existing_desc, base_embed, comps) = load_embed(app_state, component.user.id, 0).await;
    let combined_desc = if !existing_desc.is_empty() {
        format!("{}\n\n{}", feedback, existing_desc)
    } else {
        feedback.clone()
    };
    let new_embed = base_embed.description(combined_desc);
    edit_component(ctx, component, "contracts.draft", EditInteractionResponse::new().embed(new_embed).components(comps)).await;
}
async fn accept(
    ctx: &Context,
    component: &mut ComponentInteraction,
    app_state: &AppState,
    unit_id: i32,
) {
    let feedback =
        match database::human::accept_drafted_contract(&app_state.db, component.user.id, unit_id)
            .await
        {
            Ok(name) => format!("Accepted: {}", name),
            Err(e) => format!("Accept failed: {}", e),
        };
    app_state.invalidate_user_caches(component.user.id).await;
    let (existing_desc, base_embed, comps) = load_embed(app_state, component.user.id, 0).await;
    let combined_desc = if !existing_desc.is_empty() {
        format!("{}\n\n{}", feedback, existing_desc)
    } else {
        feedback.clone()
    };
    let new_embed = base_embed.description(combined_desc);
    edit_component(ctx, component, "contracts.accept", EditInteractionResponse::new().embed(new_embed).components(comps)).await;
}

async fn load_embed(
    app_state: &AppState,
    user: serenity::model::id::UserId,
    page: usize,
) -> (
    String,
    serenity::builder::CreateEmbed,
    Vec<serenity::builder::CreateActionRow>,
) {
    if let Ok(rows) = database::human::list_contract_status_cached(app_state, user).await {
        let drafted = crate::database::human::list_drafted_contracts(&app_state.db, user)
            .await
            .unwrap_or_default();
        let legacy = crate::database::human::list_legacy_open_offers(&app_state.db, user)
            .await
            .unwrap_or_default();
        let view =
            crate::commands::contracts::run::build_contracts_embed(&rows, &drafted, &legacy, page);
        (view.description.clone(), view.embed, view.components)
    } else {
        let embed = serenity::builder::CreateEmbed::new()
            .title("Contracts")
            .description("Error loading.");
        ("Error loading.".into(), embed, vec![])
    }
}
