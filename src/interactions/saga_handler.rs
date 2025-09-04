//! Handles all component interactions for the `saga` command family.

use crate::commands::games::engine::Game;
use crate::saga::battle::game::BattleGame;
// (✓) FIXED: Import the specific structs needed, removing the unused `BattlePhase`.
use crate::constants::EQUIP_BONUS_CACHE_TTL_SECS;
use crate::saga::battle::state::{BattleSession, BattleUnit};
use crate::{AppState, commands, database, saga};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = &app_state.db;
    let custom_id_parts: Vec<&str> = component.data.custom_id.split('_').collect();

    match custom_id_parts.get(1) {
        // saga_play: lightweight alias button that just re-renders the main menu
        Some(&"play") => {
            component.defer(&ctx.http).await.ok();
            let saga_profile =
                match database::saga::update_and_get_saga_profile(db, component.user.id).await {
                    Ok(p) => p,
                    Err(_) => {
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
            let (embed, components) =
                commands::saga::ui::create_saga_menu(&saga_profile, has_party);
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
        }
        Some(&"map") => {
            component.defer_ephemeral(&ctx.http).await.ok();
            let saga_profile =
                match database::saga::update_and_get_saga_profile(db, component.user.id).await {
                    Ok(profile) => profile,
                    Err(_) => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not retrieve your game profile.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };
            let available_node_ids = saga::map::get_available_nodes(saga_profile.story_progress);
            let available_nodes =
                match database::world::get_map_nodes_by_ids(db, &available_node_ids).await {
                    Ok(nodes) => nodes,
                    Err(_) => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not retrieve world map data.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };
            let (embed, components) =
                commands::saga::ui::create_world_map_view(&available_nodes, &saga_profile);
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
                // Cached equipment bonuses (EQUIP_BONUS_CACHE_TTL_SECS)
                let bonuses = {
                    let cache = app_state.bonus_cache.read().await;
                    cache.get(&component.user.id.get()).cloned()
                };
                let bonuses = if let Some((ts, map)) = bonuses {
                    if ts.elapsed() < Duration::from_secs(EQUIP_BONUS_CACHE_TTL_SECS) {
                        map
                    } else {
                        std::collections::HashMap::new()
                    }
                } else {
                    std::collections::HashMap::new()
                };
                let bonuses = if bonuses.is_empty() {
                    let fresh = database::units::get_equipment_bonuses(db, component.user.id)
                        .await
                        .unwrap_or_default();
                    let mut cache = app_state.bonus_cache.write().await;
                    cache.insert(component.user.id.get(), (Instant::now(), fresh.clone()));
                    fresh
                } else {
                    bonuses
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
                    applied_equipment: false,
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
                let builder =
                    EditInteractionResponse::new().content("You don't have enough Action Points!");
                component.edit_response(&ctx.http, builder).await.ok();
            }
        }
        Some(&"tavern") => {
            component.defer_ephemeral(&ctx.http).await.ok();
            let profile =
                match database::economy::get_or_create_profile(db, component.user.id).await {
                    Ok(p) => p,
                    Err(_) => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not retrieve your profile.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };
            let recruits =
                database::units::get_units_by_ids(db, &commands::saga::tavern::TAVERN_RECRUITS)
                    .await
                    .unwrap_or_default();
            let (embed, components) =
                commands::saga::tavern::create_tavern_menu(&recruits, profile.balance);
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
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
            let mut builder = EditInteractionResponse::new().components(vec![]);
            match result {
                Ok(pet_name) => {
                    builder = builder.content(format!(
                        "You slide {} coins across the table. **{}** joins your army!",
                        commands::saga::tavern::HIRE_COST,
                        pet_name
                    ));
                }
                Err(e) => {
                    builder = builder.content(format!("Hiring failed: {}", e));
                }
            }
            component.edit_response(&ctx.http, builder).await.ok();
        }
        Some(&"team") => {
            component.defer_ephemeral(&ctx.http).await.ok();
            database::saga::update_and_get_saga_profile(db, component.user.id)
                .await
                .ok();
            let (embed, components) =
                commands::party::ui::create_party_view_with_bonds(&app_state, component.user.id)
                    .await;
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
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
