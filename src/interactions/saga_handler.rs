//! Handles all component interactions for the `saga` command family.

use crate::commands::games::engine::Game;
use crate::saga::battle::game::BattleGame;
use crate::{AppState, commands, database};
use serenity::builder::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;

pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    let db = app_state.db.clone();
    let custom_id_parts: Vec<&str> = component.data.custom_id.split('_').collect();

    match custom_id_parts.get(1) {
        Some(&"map") => {
            component.defer(&ctx.http).await.ok();
            let spend_result =
                database::profile::spend_action_points(&db, component.user.id, 1).await;
            if let Ok(true) = spend_result {
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
                                .content("You cannot explore the map without an active party!"),
                        )
                        .await
                        .ok();
                    return;
                }

                // The ID for "Wild Slime" is 4, based on our migrations.
                let enemy_pet = database::profile::get_pets_by_ids(&db, &[4])
                    .await
                    .unwrap()
                    .remove(0);

                let session = crate::saga::battle::state::BattleSession {
                    player_party: player_party_pets.iter().map(Into::into).collect(),
                    enemy_party: vec![(&enemy_pet).into()],
                    current_turn: crate::saga::battle::state::BattleParty::Player,
                    log: vec!["A Wild Slime appears!".to_string()],
                };

                // (✓) MODIFIED: The BattleGame is now constructed with the full pet data,
                // enabling it to apply rewards and XP upon victory.
                let battle_game = BattleGame {
                    session,
                    party_members: player_party_pets,
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
