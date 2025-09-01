//! Handles all component interactions for the `saga` command family.

use crate::commands::games::engine::Game;
use crate::saga::battle::game::BattleGame;
use crate::saga::battle::state::BattlePhase;
use crate::{AppState, commands, database, saga};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = app_state.db.clone();
    let custom_id_parts: Vec<&str> = component.data.custom_id.split('_').collect();

    match custom_id_parts.get(1) {
        // Player clicked the "World Map" button in the main /saga menu.
        Some(&"map") => {
            component.defer_ephemeral(&ctx.http).await.ok();

            let saga_profile =
                match database::update_and_get_saga_profile(&db, component.user.id).await {
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
                match database::get_map_nodes_by_ids(&db, &available_node_ids).await {
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
        // Player clicked a specific battle node button on the world map.
        Some(&"node") => {
            component.defer(&ctx.http).await.ok();

            if let Ok(true) = database::spend_action_points(&db, component.user.id, 1).await {
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

                let player_party_pets =
                    match database::get_player_pets(&db, component.user.id).await {
                        Ok(pets) => pets
                            .into_iter()
                            .filter(|p| p.is_in_party)
                            .collect::<Vec<_>>(),
                        Err(_) => {
                            let builder = EditInteractionResponse::new()
                                .content("Error: Could not retrieve your pet party.");
                            component.edit_response(&ctx.http, builder).await.ok();
                            return;
                        }
                    };

                if player_party_pets.is_empty() {
                    let builder = EditInteractionResponse::new()
                        .content("You cannot start a battle without an active party!");
                    component.edit_response(&ctx.http, builder).await.ok();
                    return;
                }

                let enemies = match database::get_enemies_for_node(&db, node_id).await {
                    Ok(enemies) => enemies,
                    Err(_) => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not load enemies for this node.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };

                let node_data = match database::get_map_nodes_by_ids(&db, &[node_id]).await {
                    Ok(mut nodes) if !nodes.is_empty() => nodes.remove(0),
                    _ => {
                        let builder = EditInteractionResponse::new()
                            .content("Error: Could not find data for the selected battle node.");
                        component.edit_response(&ctx.http, builder).await.ok();
                        return;
                    }
                };

                let session = crate::saga::battle::state::BattleSession {
                    player_party: player_party_pets.iter().map(Into::into).collect(),
                    enemy_party: enemies.iter().map(Into::into).collect(),
                    phase: BattlePhase::PlayerTurn,
                    log: vec![node_data.description.clone().unwrap_or_else(|| {
                        format!("You encounter enemies at the **{}**!", node_data.name)
                    })],
                };

                // (✓) MODIFIED: `can_afford_tame` is now fetched before creating the game instance.
                let can_afford_tame = database::can_afford_tame(&db, component.user.id)
                    .await
                    .unwrap_or(false);

                // (✓) FIXED: The `BattleGame` is now initialized with the required `can_afford_tame` field.
                let battle_game = BattleGame {
                    session,
                    party_members: player_party_pets,
                    node_id,
                    node_name: node_data.name,
                    can_afford_tame,
                };

                // (✓) FIXED: The call to `render` now correctly takes no arguments, matching the `Game` trait.
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
            let profile = match database::get_or_create_profile(&db, component.user.id).await {
                Ok(p) => p,
                Err(_) => {
                    let builder = EditInteractionResponse::new()
                        .content("Error: Could not retrieve your profile.");
                    component.edit_response(&ctx.http, builder).await.ok();
                    return;
                }
            };
            let recruits = database::get_pets_by_ids(&db, &commands::saga::tavern::TAVERN_RECRUITS)
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
            let result = database::hire_mercenary(
                &db,
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
            database::update_and_get_saga_profile(&db, component.user.id)
                .await
                .ok();
            let pets = database::get_player_pets(&db, component.user.id)
                .await
                .unwrap_or_default();
            let (embed, components) = commands::party::ui::create_party_view(&pets);
            let builder = EditInteractionResponse::new()
                .embed(embed)
                .components(components);
            component.edit_response(&ctx.http, builder).await.ok();
        }
        _ => {}
    }
}
