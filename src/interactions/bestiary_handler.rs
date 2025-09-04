//! Handles Bestiary component interactions (refresh button, pagination future-ready).
use crate::database;
use crate::{
    AppState,
    commands::bestiary::ui::{BestiaryEntry, create_bestiary_embed},
};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
// Reduced tracing verbosity; only emit debug when refreshing.
#[tracing::instrument(level="debug", skip(ctx, component, _app_state), fields(user_id = component.user.id.get()))]
pub async fn handle(
    ctx: &Context,
    component: &mut ComponentInteraction,
    _app_state: Arc<AppState>,
) {
    if component.data.custom_id != "bestiary_refresh" {
        return;
    }
    component.defer_ephemeral(&ctx.http).await.ok();
    let Some(state) = AppState::from_ctx(ctx).await else {
        return;
    };
    let _db = &state.db;
    // Rebuild entries with research + (future) defeat counts
    // Prefer cached path; fall back to direct DB path if cache empty so the non-cached function stays exercised.
    let mut entries = gather_entries_enriched_cached(&state, component.user.id).await;
    if entries.is_empty() {
        entries = gather_entries_enriched(&state.db, component.user.id).await;
    }
    let (embed, components) = create_bestiary_embed(&entries);
    component
        .edit_response(
            &ctx.http,
            EditInteractionResponse::new()
                .embed(embed)
                .components(components),
        )
        .await
        .ok();
}

async fn gather_entries_enriched(
    db: &sqlx::PgPool,
    user: serenity::model::id::UserId,
) -> Vec<BestiaryEntry> {
    use crate::database::models::UnitKind;
    if let Ok(units) = database::units::get_all_units(db).await {
        let mut out = Vec::new();
        for u in units
            .into_iter()
            .filter(|u| matches!(u.kind, UnitKind::Pet))
        {
            let research_item =
                crate::commands::economy::core::item::Item::research_item_for_unit(&u.name);
            let research_owned = if let Some(item) = research_item {
                database::economy::get_inventory_item_simple(db, user, item)
                    .await
                    .ok()
                    .flatten()
                    .map(|i| i.quantity)
                    .unwrap_or(0)
            } else {
                0
            };
            // Future: track defeats vs this pet for flavor (not currently stored) so defeated stays 0.
            out.push(BestiaryEntry {
                unit: u,
                defeated: 0,
                research_owned: research_owned as i64,
            });
        }
        out
    } else {
        Vec::new()
    }
}

// Cached variant leverages research progress cache for consistency/perf
async fn gather_entries_enriched_cached(
    app_state: &AppState,
    user: serenity::model::id::UserId,
) -> Vec<BestiaryEntry> {
    use crate::database::models::UnitKind;
    let db = &app_state.db;
    if let Ok(units) = database::units::get_all_units(db).await {
        let mut out = Vec::new();
        // build research owned map from cached list (unit_id -> count)
        let mut research_counts = std::collections::HashMap::new();
        if let Ok(rows) = database::units::list_research_progress_cached(app_state, user).await {
            for (uid, cnt) in rows {
                research_counts.insert(uid, cnt);
            }
        }
        for u in units
            .into_iter()
            .filter(|u| matches!(u.kind, UnitKind::Pet))
        {
            let research_owned = research_counts.get(&u.unit_id).cloned().unwrap_or(0) as i64;
            out.push(BestiaryEntry {
                unit: u,
                defeated: 0,
                research_owned,
            });
        }
        out
    } else {
        Vec::new()
    }
}
