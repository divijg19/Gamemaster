//! Unified SagaView enum centralizing rendering for saga-related panels.
use crate::util;
use crate::{AppState, commands, database};
use serenity::builder::{CreateActionRow, CreateEmbed};
use serenity::model::id::UserId;
use std::sync::Arc;

#[derive(Clone)]
pub enum SagaView {
    Root,
    Map,
    Tavern,
    Recruit,
    Party,
}

impl SagaView {
    pub async fn render(
        &self,
        state: &AppState,
        user: UserId,
    ) -> anyhow::Result<(CreateEmbed, Vec<CreateActionRow>)> {
        match self {
            SagaView::Root => {
                let profile = database::saga::update_and_get_saga_profile(&state.db, user).await?;
                let units = database::units::get_player_units(&state.db, user)
                    .await
                    .unwrap_or_default();
                let has_party = units.iter().any(|u| u.is_in_party);
                let out = if !has_party && profile.story_progress == 0 {
                    commands::saga::ui::create_first_time_tutorial()
                } else {
                    commands::saga::ui::create_saga_menu(&profile, has_party)
                };
                Ok(out)
            }
            SagaView::Map => {
                let profile = database::saga::update_and_get_saga_profile(&state.db, user).await?;
                let node_ids = crate::saga::map::get_available_nodes(profile.story_progress);
                let nodes = database::world::get_map_nodes_by_ids(&state.db, &node_ids)
                    .await
                    .unwrap_or_default();
                Ok(commands::saga::ui::create_world_map_view(&nodes, &profile))
            }
            SagaView::Tavern | SagaView::Recruit => {
                let profile = database::economy::get_or_create_profile(&state.db, user).await?;
                let recruits = database::units::get_units_by_ids(
                    &state.db,
                    &commands::saga::tavern::TAVERN_RECRUITS,
                )
                .await
                .unwrap_or_default();
                Ok(commands::saga::tavern::create_tavern_menu(
                    &recruits,
                    profile.balance,
                ))
            }
            SagaView::Party => {
                Ok(commands::party::ui::create_party_view_with_bonds(state, user).await)
            }
        }
    }
}

/// Push a new SagaView onto the navigation stack with a max depth cap and return rendered output.
pub async fn push_and_render(
    view: SagaView,
    state: &Arc<AppState>,
    user: UserId,
    max_depth: usize,
) -> anyhow::Result<(CreateEmbed, Vec<CreateActionRow>)> {
    let (embed, components) = view.render(state, user).await?;
    if !matches!(view, SagaView::Root) {
        let mut stacks = state.nav_stacks.write().await;
        stacks.entry(user.get()).or_default().push_capped(
            Box::new(SagaNavState::new(view, embed.clone(), components.clone())),
            max_depth,
        );
    }
    Ok((embed, components))
}

pub struct SagaNavState {
    marker: &'static str,
    view: SagaView,
    embed: CreateEmbed,
    components: Vec<CreateActionRow>,
    /// Cached hash of last rendered view to allow stale detection.
    fingerprint: u64,
}

impl SagaNavState {
    fn new(view: SagaView, embed: CreateEmbed, components: Vec<CreateActionRow>) -> Self {
        let fingerprint = util::hash_embed(&embed, &components);
        Self {
            marker: view_marker(&view),
            view,
            embed,
            components,
            fingerprint,
        }
    }
    /// Re-render the underlying view if data changed (simple heuristic: story progress / balance changes).
    pub async fn refresh_if_stale(&mut self, app: &AppState, user: UserId) {
        // Only attempt for dynamic views (skip static Recruit which aliases Tavern).
        let should_check = !matches!(self.view, SagaView::Root);
        if !should_check {
            return;
        }
        if let Ok((embed, comps)) = self.view.render(app, user).await {
            let new_fp = util::hash_embed(&embed, &comps);
            if new_fp != self.fingerprint {
                self.embed = embed;
                self.components = comps;
                self.fingerprint = new_fp;
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::ui::NavState for SagaNavState {
    fn id(&self) -> &'static str {
        self.marker
    }
    async fn render(&self, _ctx: &crate::ui::ContextBag) -> (CreateEmbed, Vec<CreateActionRow>) {
        (self.embed.clone(), self.components.clone())
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

fn view_marker(v: &SagaView) -> &'static str {
    match v {
        SagaView::Root => "saga_root",
        SagaView::Map => "saga_map_view",
        SagaView::Tavern => "saga_tavern_view",
        SagaView::Recruit => "saga_recruit_view",
        SagaView::Party => "saga_party_view",
    }
}
