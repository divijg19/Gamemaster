//! Handles all component interactions for the `saga` command family.

use crate::commands::games::engine::Game;
use crate::saga::battle::game::BattleGame;
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

            // Get the player's up-to-date profile to check their story progress.
            let saga_profile = match database::profile::update_and_get_saga_profile(
                &db,
                component.user.id,
            )
            .await
            {
                Ok(profile) => profile,
                Err(_) => {
                    let builder = EditInteractionResponse::new()
                        .content("Error: Could not retrieve your game profile.");
                    component.edit_response(&ctx.http, builder).await.ok();
                    return;
                }
            };

            // Get the list of node IDs the player can access based on their progress.
            let available_node_ids = saga::map::get_available_nodes(saga_profile.story_progress);
            let available_nodes = database::profile::get_map_nodes_by_ids(&db, &available_node_ids)
                .await
                .unwrap_or_default();

            // Render the world map view with buttons for each available node.
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

            // Spend 1 AP to start the battle.
            let spend_result =
                database::profile::spend_action_points(&db, component.user.id, 1).await;
            if let Ok(true) = spend_result {
                let node_id = custom_id_parts[2].parse::<i32>().unwrap();

                // Fetch the player's active party.
                let player_party_pets = database::profile::get_player_pets(&db, component.user.id)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|p| p.is_in_party)
                    .collect::<Vec<_>>();

                if player_party_pets.is_empty() {
                    component
                        .edit_response(
                            &ctx.http,
                            EditInteractionResponse::new()
                                .content("You cannot start a battle without an active party!"),
                        )
                        .await
                        .ok();
                    return;
                }

                // Fetch the specific enemies for this node from the database.
                let enemies = database::profile::get_enemies_for_node(&db, node_id)
                    .await
                    .unwrap_or_default();
                let battle_enemies: Vec<_> = enemies.iter().map(Into::into).collect();

                // Fetch the node's data for its description.
                let node_data = database::profile::get_map_nodes_by_ids(&db, &[node_id])
                    .await
                    .unwrap()
                    .remove(0);

                let session = crate::saga::battle::state::BattleSession {
                    player_party: player_party_pets.iter().map(Into::into).collect(),
                    enemy_party: battle_enemies,
                    current_turn: crate::saga::battle::state::BattleParty::Player,
                    log: vec![node_data.description.unwrap_or_else(|| {
                        format!("You encounter enemies at the **{}**!", node_data.name)
                    })],
                };

                // Construct the BattleGame with all necessary data for rewards and progression.
                let battle_game = BattleGame {
                    session,
                    party_members: player_party_pets,
                    node_id,
                };

                let (_content, embed, components) = battle_game.render();
                let builder = EditInteractionResponse::new()
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
                component
                    .edit_response(
                        &ctx.http,
                        EditInteractionResponse::new()
                            .content("You don't have enough Action Points!"),
                    )
                    .await
                    .ok();
            }
        }
        Some(&"tavern") => {
            component.defer_ephemeral(&ctx.http).await.ok();
            let profile = database::profile::get_or_create_profile(&db, component.user.id)
                .await
                .unwrap();
            let recruits =
                database::profile::get_pets_by_ids(&db, &commands::saga::tavern::TAVERN_RECRUITS)
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
            let pet_id_to_hire = custom_id_parts[2].parse::<i32>().unwrap();
            let result = database::profile::hire_mercenary(
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
            database::profile::update_and_get_saga_profile(&db, component.user.id)
                .await
                .ok();
            let pets = database::profile::get_player_pets(&db, component.user.id)
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
