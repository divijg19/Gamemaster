//! Handles all component interactions for the `saga` command family.

use crate::commands::games::engine::Game;
use crate::saga::battle::game::BattleGame;
// (✓) FIXED: Import the specific structs needed, removing the unused `BattlePhase`.
use super::ids::*;
use super::util::{defer_component, edit_component, handle_global_nav, handle_saga_back_refresh};
use crate::commands::saga::ui::back_refresh_row;
use crate::constants::EQUIP_BONUS_CACHE_TTL_SECS;
use crate::saga::battle::state::{BattleSession, BattleUnit};
use crate::saga::view::{SagaView, push_and_render};
use crate::services::{cache as cache_service, saga as saga_service};
use crate::ui::style::error_embed;
// NavState no longer needed directly after SagaView migration
use crate::{AppState, commands, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;
use tracing::instrument;

// (Removed local edit helper; using util::edit_component for consistency.)

// Local cache helpers removed (centralized in services::saga).

#[instrument(level="debug", skip(ctx, component, app_state), fields(user_id = component.user.id.get(), cid = %component.data.custom_id))]
pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = &app_state.db;
    let raw_id = component.data.custom_id.as_str();
    let custom_id_parts: Vec<&str> = raw_id.split('_').collect();
    tracing::debug!(target="saga.interaction", user_id=%component.user.id, cid=%raw_id, "Saga component received");

    // Standard unified defer + global nav handling
    defer_component(ctx, component).await;
    if handle_global_nav(ctx, component, &app_state, "saga").await {
        return;
    }

    const MAX_NAV_DEPTH: usize = 15;

    // Centralized back / refresh.
    if handle_saga_back_refresh(ctx, component, &app_state).await {
        return;
    }

    match custom_id_parts.get(1) {
        Some(&"preview") if raw_id.starts_with("saga_preview_") => {
            // Preview a node's enemies & rewards without spending AP.
            let node_id = match raw_id.trim_start_matches("saga_preview_").parse::<i32>() {
                Ok(v) => v,
                Err(_) => {
                    edit_component(
                        ctx,
                        component,
                        "preview.bad_id",
                        EditInteractionResponse::new().content("Invalid node id for preview."),
                    )
                    .await;
                    return;
                }
            };
            match database::world::get_full_node_bundle(db, node_id).await {
                Ok((node, enemies, rewards)) => {
                    use serenity::builder::CreateEmbed;
                    let mut embed = CreateEmbed::new()
                        .title(format!("Node Preview: {}", node.name))
                        .description(
                            node.description
                                .clone()
                                .unwrap_or_else(|| "No description.".into()),
                        )
                        .field(
                            "Story Progress Req",
                            format!("`{}`", node.story_progress_required),
                            true,
                        )
                        .field(
                            "Base Rewards",
                            format!("💰 {} | XP {}", node.reward_coins, node.reward_unit_xp),
                            true,
                        )
                        .color(0x2F3136);
                    if !enemies.is_empty() {
                        let enemy_lines = enemies
                            .iter()
                            .map(|e| format!("- {} ({:?})", e.name, e.rarity))
                            .take(10)
                            .collect::<Vec<_>>()
                            .join("\n");
                        embed = embed.field("Enemies", enemy_lines, false);
                    }
                    if !rewards.is_empty() {
                        let reward_lines = rewards
                            .iter()
                            .take(10)
                            .map(|r| {
                                format!(
                                    "• Item {} x{} ({}%)",
                                    r.item_id,
                                    r.quantity,
                                    (r.drop_chance * 100.0) as i32
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n");
                        embed = embed.field("Possible Drops", reward_lines, false);
                    }
                    let mut components = Vec::new();
                    // Provide a Start Battle button (spends AP) and Back/Refresh row if applicable via existing util (depth >1).
                    components.push(serenity::builder::CreateActionRow::Buttons(vec![
                        crate::ui::buttons::Btn::primary(
                            &format!("saga_node_{}", node.node_id),
                            "⚔ Start Battle",
                        ),
                        crate::ui::buttons::Btn::secondary("nav_saga", "↩ Saga"),
                    ]));
                    components.push(crate::commands::saga::ui::global_nav_row("saga"));
                    edit_component(
                        ctx,
                        component,
                        "preview.render",
                        EditInteractionResponse::new()
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Err(_) => {
                    edit_component(
                        ctx,
                        component,
                        "preview.load_err",
                        EditInteractionResponse::new().content("Failed to load preview."),
                    )
                    .await;
                }
            }
            return;
        }
        // (Removed duplicate early tutorial handler; consolidated later in file around line ~630)
        // Global navigation buttons (handled here for saga context)
        Some(&"saga") if raw_id == NAV_SAGA => {
            // Re-render main saga root via SagaView abstraction.
            match push_and_render(SagaView::Root, &app_state, component.user.id, MAX_NAV_DEPTH)
                .await
            {
                Ok((embed, components)) => {
                    edit_component(
                        ctx,
                        component,
                        "nav_saga.ok",
                        EditInteractionResponse::new()
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Err(e) => {
                    edit_component(
                        ctx,
                        component,
                        "nav_saga.err",
                        EditInteractionResponse::new()
                            .content(format!("Error loading saga root: {e}")),
                    )
                    .await;
                }
            }
            return;
        }
        Some(&"party") if raw_id == NAV_PARTY => {
            let (embed, components) =
                commands::party::ui::create_party_view_with_bonds(&app_state, component.user.id)
                    .await;
            edit_component(
                ctx,
                component,
                "nav_party",
                EditInteractionResponse::new()
                    .embed(embed)
                    .components(components),
            )
            .await;
            return;
        }
        Some(&"train") if raw_id == NAV_TRAIN => {
            if let (Ok(units), Some(profile)) = (
                database::units::get_player_units(db, component.user.id).await,
                saga_service::get_saga_profile(&app_state, component.user.id, false).await,
            ) {
                let (embed, components) =
                    commands::train::ui::create_training_menu(&units, &profile);
                edit_component(
                    ctx,
                    component,
                    "nav_train",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            }
            return;
        }
        Some(&"map") => {
            // Guard: need party + 1 AP
            let saga_profile =
                match database::saga::update_and_get_saga_profile(db, component.user.id).await {
                    Ok(p) => p,
                    Err(e) => {
                        edit_component(
                            ctx,
                            component,
                            "map.profile_err",
                            EditInteractionResponse::new()
                                .content(format!("Failed to load profile: {e}")),
                        )
                        .await;
                        return;
                    }
                };
            let has_party = database::units::get_user_party(db, component.user.id)
                .await
                .map(|p| !p.is_empty())
                .unwrap_or(false);
            if saga_profile.current_ap < 1 || !has_party {
                edit_component(
                    ctx,
                    component,
                    "map.blocked",
                    EditInteractionResponse::new()
                        .content("You can't open the World Map right now (need party and 1+ AP)."),
                )
                .await;
                return;
            }
            if let Ok((embed, mut components)) =
                push_and_render(SagaView::Map, &app_state, component.user.id, MAX_NAV_DEPTH).await
            {
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
                edit_component(
                    ctx,
                    component,
                    "map.render",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            } else {
                edit_component(
                    ctx,
                    component,
                    "map.render_err",
                    EditInteractionResponse::new().content("Failed to render map."),
                )
                .await;
            }
        }
        Some(&"tavern") => {
            if let Ok((embed, mut components)) = push_and_render(
                SagaView::Tavern,
                &app_state,
                component.user.id,
                MAX_NAV_DEPTH,
            )
            .await
            {
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
                edit_component(
                    ctx,
                    component,
                    "tavern.render",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            } else {
                edit_component(
                    ctx,
                    component,
                    "tavern.render_err",
                    EditInteractionResponse::new().content("Failed to render tavern."),
                )
                .await;
            }
        }
        Some(&"recruit") => {
            if let Ok((embed, mut components)) = push_and_render(
                SagaView::Recruit,
                &app_state,
                component.user.id,
                MAX_NAV_DEPTH,
            )
            .await
            {
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
                edit_component(
                    ctx,
                    component,
                    "recruit.render",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            } else {
                edit_component(
                    ctx,
                    component,
                    "recruit.render_err",
                    EditInteractionResponse::new().content("Failed to render recruit view."),
                )
                .await;
            }
        }
        Some(&"team") => {
            if let Ok((embed, mut components)) = push_and_render(
                SagaView::Party,
                &app_state,
                component.user.id,
                MAX_NAV_DEPTH,
            )
            .await
            {
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
                edit_component(
                    ctx,
                    component,
                    "team.render",
                    EditInteractionResponse::new()
                        .embed(embed)
                        .components(components),
                )
                .await;
            } else {
                edit_component(
                    ctx,
                    component,
                    "team.render_err",
                    EditInteractionResponse::new().content("Failed to render party view."),
                )
                .await;
            }
        }
        Some(&suffix) if crate::interactions::ids::is_saga_node(raw_id) || suffix == "node" => {
            // Support both new full custom_id (saga_node_<id>) and legacy split form (saga_node_<id>) already parsed.
            let node_id = if crate::interactions::ids::is_saga_node(raw_id) {
                match raw_id
                    .trim_start_matches(crate::interactions::ids::SAGA_NODE_PREFIX)
                    .parse::<i32>()
                {
                    Ok(v) => v,
                    Err(_) => {
                        edit_component(
                            ctx,
                            component,
                            "node.bad_id",
                            EditInteractionResponse::new()
                                .content("Error: Invalid battle node ID format."),
                        )
                        .await;
                        return;
                    }
                }
            } else if let Some(id_str) = custom_id_parts.get(2) {
                if let Ok(id) = id_str.parse::<i32>() {
                    id
                } else {
                    edit_component(
                        ctx,
                        component,
                        "node.bad_id",
                        EditInteractionResponse::new()
                            .content("Error: Invalid battle node ID format."),
                    )
                    .await;
                    return;
                }
            } else {
                edit_component(
                    ctx,
                    component,
                    "node.missing_id",
                    EditInteractionResponse::new().content("Error: Missing battle node ID."),
                )
                .await;
                return;
            };

            // Ensure player has a valid party before spending AP
            let player_party_units =
                match database::units::get_user_party(db, component.user.id).await {
                    Ok(units) if !units.is_empty() => units,
                    _ => {
                        edit_component(
                            ctx,
                            component,
                            "node.no_party",
                            EditInteractionResponse::new()
                                .content("You cannot start a battle without an active party!"),
                        )
                        .await;
                        return;
                    }
                };

            // Spend AP last so failures above don't consume it
            if let Ok(true) = database::saga::spend_action_points(db, component.user.id, 1).await {
                let (node_data, enemies, _rewards) =
                    match database::world::get_full_node_bundle(db, node_id).await {
                        Ok(bundle) => bundle,
                        Err(_) => {
                            edit_component(
                                ctx,
                                component,
                                "node.bundle_err",
                                EditInteractionResponse::new()
                                    .content("Error: Could not load node data."),
                            )
                            .await;
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
                edit_component(
                    ctx,
                    component,
                    "node.no_ap",
                    EditInteractionResponse::new().embed(error_embed(
                        "Not Enough Action Points",
                        "You need more AP to start this battle. Come back after they recharge.",
                    )),
                )
                .await;
            }
        }
        Some(&"hire") => {
            let pet_id_to_hire = custom_id_parts[2].parse::<i32>().unwrap_or(0);
            let result = database::units::hire_unit(
                db,
                component.user.id,
                pet_id_to_hire,
                commands::saga::tavern::HIRE_COST,
            )
            .await;
            let builder = match result {
                Ok(pet_name) => {
                    // Fetch remaining balance for richer feedback (ignore failure quietly)
                    let balance_after =
                        database::economy::get_or_create_profile(db, component.user.id)
                            .await
                            .map(|p| p.balance)
                            .unwrap_or_default();
                    EditInteractionResponse::new().embed(
                        commands::saga::tavern::recruit_success_embed(&pet_name, balance_after),
                    )
                }
                Err(e) => EditInteractionResponse::new().embed(error_embed(
                    "Recruit Failed",
                    format!("Hiring failed: {}", e),
                )),
            };
            edit_component(ctx, component, "hire.result", builder).await;
        }
        Some(&"main") => {
            match push_and_render(SagaView::Root, &app_state, component.user.id, MAX_NAV_DEPTH)
                .await
            {
                Ok((embed, components)) => {
                    edit_component(
                        ctx,
                        component,
                        "main.render",
                        EditInteractionResponse::new()
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Err(e) => {
                    edit_component(
                        ctx,
                        component,
                        "main.err",
                        EditInteractionResponse::new()
                            .content(format!("Error: Could not refresh saga root ({e})")),
                    )
                    .await;
                }
            }
        }
        Some(&"tutorial") => {
            match custom_id_parts.get(2) {
                Some(&"hire") => {
                    // Give a free starter unit (unit_id 1 assumed) if player has none
                    let has_any = database::units::get_player_units(db, component.user.id)
                        .await
                        .map(|v| !v.is_empty())
                        .unwrap_or(false);
                    if has_any {
                        edit_component(
                            ctx,
                            component,
                            "tutorial.hire_skip",
                            EditInteractionResponse::new()
                                .content("You already have a unit. Tutorial reward skipped."),
                        )
                        .await;
                        return;
                    }
                    // Insert starter without cost
                    let user_id_i64 = component.user.id.get() as i64;
                    let starter_id = *app_state.starter_unit_id.read().await;
                    if let Err(e) = sqlx::query!("INSERT INTO player_units (user_id, unit_id, nickname, current_level, current_xp, current_attack, current_defense, current_health, is_in_party, rarity) SELECT $1, u.unit_id, u.name, 1, 0, u.base_attack, u.base_defense, u.base_health, TRUE, u.rarity FROM units u WHERE u.unit_id = $2 ON CONFLICT DO NOTHING", user_id_i64, starter_id).execute(db).await {
                        edit_component(ctx, component, "tutorial.hire_err", EditInteractionResponse::new().content(format!("Failed to grant starter unit: {}", e))).await;
                        return;
                    }
                    // Refresh saga profile and show main menu
                    let (embed, components) = match push_and_render(
                        SagaView::Root,
                        &app_state,
                        component.user.id,
                        MAX_NAV_DEPTH,
                    )
                    .await
                    {
                        Ok(ec) => ec,
                        Err(e) => {
                            edit_component(
                                ctx,
                                component,
                                "tutorial.refresh_err",
                                EditInteractionResponse::new()
                                    .content(format!("Failed to refresh saga root: {}", e)),
                            )
                            .await;
                            return;
                        }
                    };
                    edit_component(
                        ctx,
                        component,
                        "tutorial.hire_ok",
                        EditInteractionResponse::new()
                            .content("Starter unit recruited and added to your party!")
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                Some(&"skip") => {
                    // Just show main menu (will still be disabled map until a recruit happens)
                    let (embed, components) = match push_and_render(
                        SagaView::Root,
                        &app_state,
                        component.user.id,
                        MAX_NAV_DEPTH,
                    )
                    .await
                    {
                        Ok(ec) => ec,
                        Err(e) => {
                            edit_component(
                                ctx,
                                component,
                                "tutorial.skip_refresh_err",
                                EditInteractionResponse::new()
                                    .content(format!("Failed to refresh saga root: {}", e)),
                            )
                            .await;
                            return;
                        }
                    };
                    edit_component(
                        ctx,
                        component,
                        "tutorial.skip_ok",
                        EditInteractionResponse::new()
                            .content("Tutorial skipped.")
                            .embed(embed)
                            .components(components),
                    )
                    .await;
                }
                _ => {
                    edit_component(
                        ctx,
                        component,
                        "tutorial.unknown",
                        EditInteractionResponse::new().content("Unknown tutorial action."),
                    )
                    .await;
                }
            }
        }
        _ => {
            edit_component(
                ctx,
                component,
                "unknown",
                EditInteractionResponse::new().content("Unknown saga interaction."),
            )
            .await;
        }
    }
}
