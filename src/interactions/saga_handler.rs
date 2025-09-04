//! Handles all component interactions for the `saga` command family.

use crate::commands::games::engine::Game;
use crate::saga::battle::game::BattleGame;
// (✓) FIXED: Import the specific structs needed, removing the unused `BattlePhase`.
use crate::commands::saga::ui::back_refresh_row;
use crate::constants::EQUIP_BONUS_CACHE_TTL_SECS;
use crate::saga::battle::state::{BattleSession, BattleUnit};
use crate::services::{cache as cache_service, saga as saga_service};
use crate::ui::style::{error_embed, success_embed};
use crate::ui::{ContextBag, NavState};
use crate::{AppState, commands, database, saga};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;
use tracing::debug;
use tracing::instrument;

// Local cache helpers removed (centralized in services::saga).

struct SagaMenuState {
    has_party: bool,
    ap: i32,
    max_ap: i32,
    tp: i32,
    max_tp: i32,
}

struct SagaWorldMapState {
    nodes: Vec<crate::database::models::MapNode>,
    ap: i32,
    max_ap: i32,
    story_progress: i32,
}
#[async_trait::async_trait]
impl NavState for SagaWorldMapState {
    fn id(&self) -> &'static str {
        "saga_world_map"
    }
    async fn render(
        &self,
        ctx: &ContextBag,
    ) -> (
        serenity::builder::CreateEmbed,
        Vec<serenity::builder::CreateActionRow>,
    ) {
        let profile_stub = crate::database::models::SagaProfile {
            current_ap: self.ap,
            max_ap: self.max_ap,
            current_tp: 0,
            max_tp: 0,
            last_tp_update: chrono::Utc::now(),
            story_progress: self.story_progress,
        };
        // Touch ctx fields (db & user_id) to keep them "used" for now until future data-driven rendering.
        let _ = (&ctx.db, ctx.user_id);
        commands::saga::ui::create_world_map_view(&self.nodes, &profile_stub)
    }
}

struct SagaTavernState {
    recruits: Vec<crate::database::models::Unit>,
    balance: i64,
}
#[async_trait::async_trait]
impl NavState for SagaTavernState {
    fn id(&self) -> &'static str {
        "saga_tavern"
    }
    async fn render(
        &self,
        ctx: &ContextBag,
    ) -> (
        serenity::builder::CreateEmbed,
        Vec<serenity::builder::CreateActionRow>,
    ) {
        let _ = (&ctx.db, ctx.user_id);
        commands::saga::tavern::create_tavern_menu(&self.recruits, self.balance)
    }
}

struct SagaPartyState {
    embed: serenity::builder::CreateEmbed,
    components: Vec<serenity::builder::CreateActionRow>,
}
#[async_trait::async_trait]
impl NavState for SagaPartyState {
    fn id(&self) -> &'static str {
        "saga_party"
    }
    async fn render(
        &self,
        ctx: &ContextBag,
    ) -> (
        serenity::builder::CreateEmbed,
        Vec<serenity::builder::CreateActionRow>,
    ) {
        let _ = (&ctx.db, ctx.user_id);
        (self.embed.clone(), self.components.clone())
    }
}

#[async_trait::async_trait]
impl NavState for SagaMenuState {
    fn id(&self) -> &'static str {
        "saga_menu"
    }
    async fn render(
        &self,
        ctx: &ContextBag,
    ) -> (
        serenity::builder::CreateEmbed,
        Vec<serenity::builder::CreateActionRow>,
    ) {
        let _ = (&ctx.db, ctx.user_id);
        let (embed_final, components) = commands::saga::ui::create_saga_menu(
            &crate::database::models::SagaProfile {
                current_ap: self.ap,
                max_ap: self.max_ap,
                current_tp: self.tp,
                max_tp: self.max_tp,
                last_tp_update: chrono::Utc::now(),
                story_progress: 0,
            },
            self.has_party,
        );
        (embed_final, components)
    }
}

#[instrument(level="debug", skip(ctx, component, app_state), fields(user_id = component.user.id.get(), cid = %component.data.custom_id))]
pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = &app_state.db;
    let custom_id_parts: Vec<&str> = component.data.custom_id.split('_').collect();

    match custom_id_parts.get(1) {
        // (Removed duplicate early tutorial handler; consolidated later in file around line ~630)
        // Global navigation buttons (handled here for saga context)
        Some(&"saga") if component.data.custom_id == "nav_saga" => {
            // Re-render main saga menu (alias to saga_play)
            component.defer(&ctx.http).await.ok();
            let _ = database::economy::get_or_create_profile(db, component.user.id).await;
            if let Some(saga_profile) =
                saga_service::get_saga_profile(&app_state, component.user.id, false).await
            {
                let has_party = database::units::get_user_party(db, component.user.id)
                    .await
                    .map(|p| !p.is_empty())
                    .unwrap_or(false);
                let (embed, components) =
                    commands::saga::ui::create_saga_menu(&saga_profile, has_party);
                let builder = EditInteractionResponse::new()
                    .embed(embed)
                    .components(components);
                component.edit_response(&ctx.http, builder).await.ok();
            }
            return;
        }
        Some(&"party") if component.data.custom_id == "nav_party" => {
            component.defer(&ctx.http).await.ok();
            let (embed, components) =
                commands::party::ui::create_party_view_with_bonds(&app_state, component.user.id)
                    .await;
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
            return;
        }
        Some(&"train") if component.data.custom_id == "nav_train" => {
            component.defer(&ctx.http).await.ok();
            if let (Ok(units), Some(profile)) = (
                database::units::get_player_units(db, component.user.id).await,
                saga_service::get_saga_profile(&app_state, component.user.id, false).await,
            ) {
                let (embed, components) =
                    commands::train::ui::create_training_menu(&units, &profile);
                let builder = EditInteractionResponse::new()
                    .embed(embed)
                    .components(components);
                component.edit_response(&ctx.http, builder).await.ok();
            }
            return;
        }
        // saga_play: lightweight alias button that just re-renders the main menu
        Some(&"play") => {
            component.defer(&ctx.http).await.ok();
            // Ensure economy profile exists (FK requirement) then fetch saga profile
            let _ = database::economy::get_or_create_profile(db, component.user.id).await;
            let saga_profile =
                match saga_service::get_saga_profile(&app_state, component.user.id, false).await {
                    Some(p) => p,
                    None => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not load saga profile.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };
            let has_party = database::units::get_user_party(db, component.user.id)
                .await
                .map(|p| !p.is_empty())
                .unwrap_or(false);
            {
                let mut stacks = app_state.nav_stacks.write().await;
                let entry = stacks.entry(component.user.id.get()).or_default();
                if entry.stack.len() > 14 {
                    entry.stack.remove(0);
                }
                entry.push(Box::new(SagaMenuState {
                    has_party,
                    ap: saga_profile.current_ap,
                    max_ap: saga_profile.max_ap,
                    tp: saga_profile.current_tp,
                    max_tp: saga_profile.max_tp,
                }));
                debug!(target: "nav", user_id = component.user.id.get(), depth = entry.stack.len(), state = "saga_menu", action = "push");
            }
            let ctxbag = ContextBag::new(db.clone(), component.user.id);
            // Render top of stack
            if let Some(stack_top) = app_state
                .nav_stacks
                .read()
                .await
                .get(&component.user.id.get())
                .and_then(|s| s.stack.last())
            {
                let (embed, components) = stack_top.render(&ctxbag).await;
                let builder = EditInteractionResponse::new()
                    .embed(embed)
                    .components(components);
                component.edit_response(&ctx.http, builder).await.ok();
            }
        }
        Some(&"map") => {
            component.defer(&ctx.http).await.ok();
            if let Ok(saga_profile) =
                database::saga::update_and_get_saga_profile(db, component.user.id).await
            {
                let node_ids = saga::map::get_available_nodes(saga_profile.story_progress);
                if let Ok(nodes) = database::world::get_map_nodes_by_ids(db, &node_ids).await {
                    let state = SagaWorldMapState {
                        nodes,
                        ap: saga_profile.current_ap,
                        max_ap: saga_profile.max_ap,
                        story_progress: saga_profile.story_progress,
                    };
                    {
                        let mut stacks = app_state.nav_stacks.write().await;
                        stacks
                            .entry(component.user.id.get())
                            .or_default()
                            .push(Box::new(state));
                    }
                    let ctxbag = ContextBag::new(db.clone(), component.user.id);
                    if let Some(top) = app_state
                        .nav_stacks
                        .read()
                        .await
                        .get(&component.user.id.get())
                        .and_then(|s| s.stack.last())
                    {
                        let (embed, mut components) = top.render(&ctxbag).await;
                        let depth = app_state
                            .nav_stacks
                            .read()
                            .await
                            .get(&component.user.id.get())
                            .map(|s| s.stack.len())
                            .unwrap_or(1);
                        if let Some(row) = back_refresh_row(depth) {
                            components.push(row);
                        }
                        let builder = EditInteractionResponse::new()
                            .embed(embed)
                            .components(components);
                        component.edit_response(&ctx.http, builder).await.ok();
                    }
                }
            }
        }
        Some(&"tavern") => {
            component.defer(&ctx.http).await.ok();
            // Ensure economy profile for balance
            let profile =
                match database::economy::get_or_create_profile(db, component.user.id).await {
                    Ok(p) => p,
                    Err(_) => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not load your profile.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };
            let recruits =
                database::units::get_units_by_ids(db, &commands::saga::tavern::TAVERN_RECRUITS)
                    .await
                    .unwrap_or_default();
            let state = SagaTavernState {
                recruits,
                balance: profile.balance,
            };
            {
                let mut stacks = app_state.nav_stacks.write().await;
                stacks
                    .entry(component.user.id.get())
                    .or_default()
                    .push(Box::new(state));
            }
            let ctxbag = ContextBag::new(db.clone(), component.user.id);
            if let Some(top) = app_state
                .nav_stacks
                .read()
                .await
                .get(&component.user.id.get())
                .and_then(|s| s.stack.last())
            {
                let (embed, mut components) = top.render(&ctxbag).await;
                let depth = app_state
                    .nav_stacks
                    .read()
                    .await
                    .get(&component.user.id.get())
                    .map(|s| s.stack.len())
                    .unwrap_or(1);
                if let Some(row) = back_refresh_row(depth) {
                    components.push(row);
                }
                let builder = EditInteractionResponse::new()
                    .embed(embed)
                    .components(components);
                component.edit_response(&ctx.http, builder).await.ok();
            }
        }
        Some(&"team") => {
            component.defer(&ctx.http).await.ok();
            let (embed, components) =
                commands::party::ui::create_party_view_with_bonds(&app_state, component.user.id)
                    .await;
            let state = SagaPartyState {
                embed: embed.clone(),
                components: components.clone(),
            };
            {
                let mut stacks = app_state.nav_stacks.write().await;
                stacks
                    .entry(component.user.id.get())
                    .or_default()
                    .push(Box::new(state));
            }
            let depth = app_state
                .nav_stacks
                .read()
                .await
                .get(&component.user.id.get())
                .map(|s| s.stack.len())
                .unwrap_or(1);
            let mut components = components;
            if let Some(row) = back_refresh_row(depth) {
                components.push(row);
            }
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
        }
        Some(&"node") => {
            component.defer(&ctx.http).await.ok();
            // Parse node id first
            let node_id = if let Some(id_str) = custom_id_parts.get(2) {
                if let Ok(id) = id_str.parse::<i32>() {
                    id
                } else {
                    let builder = EditInteractionResponse::new()
                        .content("Error: Invalid battle node ID format.");
                    component.edit_response(&ctx.http, builder).await.ok();
                    return;
                }
            } else {
                let builder =
                    EditInteractionResponse::new().content("Error: Missing battle node ID.");
                component.edit_response(&ctx.http, builder).await.ok();
                return;
            };

            // Ensure player has a valid party before spending AP
            let player_party_units =
                match database::units::get_user_party(db, component.user.id).await {
                    Ok(units) if !units.is_empty() => units,
                    _ => {
                        let builder = EditInteractionResponse::new()
                            .content("You cannot start a battle without an active party!");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };

            // Spend AP last so failures above don't consume it
            if let Ok(true) = database::saga::spend_action_points(db, component.user.id, 1).await {
                let (node_data, enemies, _rewards) =
                    match database::world::get_full_node_bundle(db, node_id).await {
                        Ok(bundle) => bundle,
                        Err(_) => {
                            let builder = EditInteractionResponse::new()
                                .content("Error: Could not load node data.");
                            component.edit_response(&ctx.http, builder).await.ok();
                            return;
                        }
                    };
                // Cached equipment bonuses via generic TTL helper
                let bonuses = if let Some(map) = cache_service::get_with_ttl(
                    &app_state.bonus_cache,
                    &component.user.id.get(),
                    Duration::from_secs(EQUIP_BONUS_CACHE_TTL_SECS),
                )
                .await
                {
                    map
                } else {
                    let fresh = database::units::get_equipment_bonuses(db, component.user.id)
                        .await
                        .unwrap_or_default();
                    cache_service::insert(
                        &app_state.bonus_cache,
                        component.user.id.get(),
                        fresh.clone(),
                    )
                    .await;
                    fresh
                };
                let mut synergy_log: Vec<String> = Vec::new();
                let player_units: Vec<BattleUnit> = player_party_units
                    .iter()
                    .map(|u| {
                        if let Some(b) = bonuses.get(&u.player_unit_id) {
                            if b.0 > 0 || b.1 > 0 || b.2 > 0 {
                                synergy_log.push(format!(
                                    "🔗 {} gains +{} Atk / +{} Def / +{} HP from bonded unit(s).",
                                    u.nickname.as_deref().unwrap_or(&u.name),
                                    b.0,
                                    b.1,
                                    b.2
                                ));
                            }
                            BattleUnit::from_player_unit_with_bonus(u, *b)
                        } else {
                            BattleUnit::from_player_unit(u)
                        }
                    })
                    .collect();
                let enemy_units: Vec<BattleUnit> =
                    enemies.iter().map(BattleUnit::from_unit).collect();
                let mut session = BattleSession::new(player_units, enemy_units);
                session.log.extend(synergy_log);
                let can_afford_recruit = database::units::can_afford_recruit(db, component.user.id)
                    .await
                    .unwrap_or(false);
                let battle_game = BattleGame {
                    session,
                    party_members: player_party_units,
                    node_id,
                    node_name: node_data.name,
                    can_afford_recruit,
                    player_quest_id: None,
                    claimed: false,
                };
                let (content, embed, components) = battle_game.render();
                let builder = EditInteractionResponse::new()
                    .content(content)
                    .embed(embed)
                    .components(components);
                if let Ok(msg) = component.edit_response(&ctx.http, builder).await {
                    app_state
                        .game_manager
                        .write()
                        .await
                        .start_game(msg.id, Box::new(battle_game));
                }
            } else {
                let builder = EditInteractionResponse::new().embed(error_embed(
                    "Not Enough Action Points",
                    "You need more AP to start this battle. Come back after they recharge.",
                ));
                component.edit_response(&ctx.http, builder).await.ok();
            }
        }
        Some(&"hire") => {
            component.defer_ephemeral(&ctx.http).await.ok();
            let pet_id_to_hire = custom_id_parts[2].parse::<i32>().unwrap_or(0);
            let result = database::units::hire_unit(
                db,
                component.user.id,
                pet_id_to_hire,
                commands::saga::tavern::HIRE_COST,
            )
            .await;
            let builder = match result {
                Ok(pet_name) => EditInteractionResponse::new().embed(
                    success_embed(
                        "Recruit Hired",
                        format!("**{}** joins your forces!", pet_name),
                    )
                    .field(
                        "Cost",
                        format!("{} coins", commands::saga::tavern::HIRE_COST),
                        true,
                    ),
                ),
                Err(e) => EditInteractionResponse::new().embed(error_embed(
                    "Recruit Failed",
                    format!("Hiring failed: {}", e),
                )),
            };
            component.edit_response(&ctx.http, builder).await.ok();
        }
        Some(&"back") => {
            component.defer(&ctx.http).await.ok();
            {
                let mut stacks = app_state.nav_stacks.write().await;
                if let Some(s) = stacks.get_mut(&component.user.id.get())
                    && let Some(old) = s.pop()
                {
                    debug!(target: "nav", user_id = component.user.id.get(), state = old.id(), action = "pop", depth = s.stack.len());
                }
            }
            // Re-render new top (or fallback main saga menu)
            let has_party = database::units::get_user_party(db, component.user.id)
                .await
                .map(|p| !p.is_empty())
                .unwrap_or(false);
            let saga_profile =
                saga_service::get_saga_profile(&app_state, component.user.id, false).await;
            if let Some(nav) = app_state
                .nav_stacks
                .read()
                .await
                .get(&component.user.id.get())
                .and_then(|s| s.stack.last())
            {
                let ctxbag = ContextBag::new(db.clone(), component.user.id);
                let (embed, components) = nav.render(&ctxbag).await;
                let builder = EditInteractionResponse::new()
                    .embed(embed)
                    .components(components);
                component.edit_response(&ctx.http, builder).await.ok();
            } else if let Some(profile) = saga_profile {
                let (embed, components) = commands::saga::ui::create_saga_menu(&profile, has_party);
                let builder = EditInteractionResponse::new()
                    .embed(embed)
                    .components(components);
                component.edit_response(&ctx.http, builder).await.ok();
            }
        }
        Some(&"refresh") => {
            component.defer(&ctx.http).await.ok();
            // Force refresh: bypass cache by always fetching then writing (refresh semantics)
            let _ = saga_service::get_saga_profile(&app_state, component.user.id, true).await;
            let ctxbag = ContextBag::new(db.clone(), component.user.id);
            if let Some(nav) = app_state
                .nav_stacks
                .read()
                .await
                .get(&component.user.id.get())
                .and_then(|s| s.stack.last())
            {
                let (embed, mut components) = nav.render(&ctxbag).await;
                let depth = app_state
                    .nav_stacks
                    .read()
                    .await
                    .get(&component.user.id.get())
                    .map(|s| s.stack.len())
                    .unwrap_or(1);
                if let Some(row) = back_refresh_row(depth) {
                    components.push(row);
                }
                let builder = EditInteractionResponse::new()
                    .embed(embed)
                    .components(components);
                component.edit_response(&ctx.http, builder).await.ok();
            }
        }
        Some(&"main") => {
            // Return to the main saga menu
            component.defer(&ctx.http).await.ok();
            let saga_profile =
                match database::saga::update_and_get_saga_profile(db, component.user.id).await {
                    Ok(p) => p,
                    Err(_) => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not refresh saga menu.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };
            let has_party = database::units::get_user_party(db, component.user.id)
                .await
                .map(|p| !p.is_empty())
                .unwrap_or(false);
            let (embed, components) =
                commands::saga::ui::create_saga_menu(&saga_profile, has_party);
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
        }
        Some(&"tutorial") => {
            component.defer_ephemeral(&ctx.http).await.ok();
            match custom_id_parts.get(2) {
                Some(&"hire") => {
                    // Give a free starter unit (unit_id 1 assumed) if player has none
                    let has_any = database::units::get_player_units(db, component.user.id)
                        .await
                        .map(|v| !v.is_empty())
                        .unwrap_or(false);
                    if has_any {
                        let builder = EditInteractionResponse::new()
                            .content("You already have a unit. Tutorial reward skipped.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                    // Insert starter without cost
                    let user_id_i64 = component.user.id.get() as i64;
                    let starter_id = *app_state.starter_unit_id.read().await;
                    if let Err(e) = sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_level, current_xp, current_attack, current_defense, current_health, is_in_party, rarity) SELECT $1, u.unit_id, u.name, 1, 0, u.base_attack, u.base_defense, u.base_health, TRUE, u.rarity FROM units u WHERE u.unit_id = $2 ON CONFLICT DO NOTHING", user_id_i64, starter_id).execute(db).await {
                        let builder = EditInteractionResponse::new().content(format!("Failed to grant starter unit: {}", e));
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                    // Refresh saga profile and show main menu
                    let saga_profile =
                        match database::saga::update_and_get_saga_profile(db, component.user.id)
                            .await
                        {
                            Ok(p) => p,
                            Err(e) => {
                                let builder = EditInteractionResponse::new()
                                    .content(format!("Failed to refresh saga profile: {}", e));
                                component.edit_response(&ctx.http, builder).await.ok();
                                return;
                            }
                        };
                    let (embed, components) =
                        commands::saga::ui::create_saga_menu(&saga_profile, true);
                    let builder = EditInteractionResponse::new()
                        .content("Starter unit recruited and added to your party!")
                        .embed(embed)
                        .components(components);
                    component.edit_response(&ctx.http, builder).await.ok();
                }
                Some(&"skip") => {
                    // Just show main menu (will still be disabled map until a recruit happens)
                    let saga_profile =
                        match database::saga::update_and_get_saga_profile(db, component.user.id)
                            .await
                        {
                            Ok(p) => p,
                            Err(e) => {
                                let builder = EditInteractionResponse::new()
                                    .content(format!("Failed to refresh saga profile: {}", e));
                                component.edit_response(&ctx.http, builder).await.ok();
                                return;
                            }
                        };
                    let has_party = database::units::get_user_party(db, component.user.id)
                        .await
                        .map(|p| !p.is_empty())
                        .unwrap_or(false);
                    let (embed, components) =
                        commands::saga::ui::create_saga_menu(&saga_profile, has_party);
                    let builder = EditInteractionResponse::new()
                        .content("Tutorial skipped.")
                        .embed(embed)
                        .components(components);
                    component.edit_response(&ctx.http, builder).await.ok();
                }
                _ => {
                    let builder =
                        EditInteractionResponse::new().content("Unknown tutorial action.");
                    component.edit_response(&ctx.http, builder).await.ok();
                }
            }
        }
        _ => {
            // Gracefully ignore but surface minimal feedback once (ephemeral)
            component.defer_ephemeral(&ctx.http).await.ok();
            let builder = EditInteractionResponse::new().content("Unknown saga interaction.");
            component.edit_response(&ctx.http, builder).await.ok();
        }
    }
}
