//! Handles all component interactions for the `/quests` command family.

use crate::commands::games::engine::GameManager;
use crate::database;
use crate::saga::battle::game::BattleGame;
use crate::saga::battle::state::{BattleSession, BattleUnit};
use crate::{AppState, interactions};
use serenity::all::EditInteractionResponse;
use serenity::model::application::ComponentInteraction;
use serenity::prelude::Context;
use std::sync::Arc;
use tokio::sync::RwLock;

/// The main entry point for quest-related component interactions.
pub async fn handle(ctx: &Context, component: &mut ComponentInteraction, app_state: Arc<AppState>) {
    // (✓) FIXED: Clone the string slice to avoid borrow checker errors (E0502).
    if let Some(player_quest_id_str) = component.data.custom_id.strip_prefix("quest_accept_") {
        let quest_id_str = player_quest_id_str.to_string();
        handle_accept_quest(ctx, component, &quest_id_str, app_state).await;
    }
}

/// Handles the logic for when a player clicks an "Accept" button on the quest board.
async fn handle_accept_quest(
    ctx: &Context,
    component: &mut ComponentInteraction,
    player_quest_id_str: &str,
    app_state: Arc<AppState>,
) {
    let player_quest_id: i32 = match player_quest_id_str.parse() {
        Ok(id) => id,
        Err(_) => {
            let builder = EditInteractionResponse::new().content("❌ Error: Invalid quest ID.");
            component.edit_response(&ctx.http, builder).await.ok();
            return;
        }
    };

    component.defer(&ctx.http).await.ok();

    match database::quests::accept_quest(&app_state.db, component.user.id, player_quest_id).await {
        Ok(accepted_quest) => match accepted_quest.quest_type {
            database::models::QuestType::Battle => {
                start_quest_battle(
                    ctx,
                    component,
                    accepted_quest,
                    app_state.game_manager.clone(),
                )
                .await;
            }
            database::models::QuestType::Riddle => {
                let builder = EditInteractionResponse::new()
                    .content("Riddle quests are not yet implemented.")
                    .components(vec![]);
                component.edit_response(&ctx.http, builder).await.ok();
            }
        },
        Err(e) => {
            let builder = EditInteractionResponse::new()
                .content(format!("❌ Could not accept quest: {}", e))
                .components(vec![]);
            component.edit_response(&ctx.http, builder).await.ok();
        }
    }
}

/// Constructs and launches a battle game session for a battle quest.
async fn start_quest_battle(
    ctx: &Context,
    component: &mut ComponentInteraction,
    quest: database::quests::AcceptedQuest,
    // (✓) FIXED: Use the correct GameManager type: Arc<RwLock<GameManager>> to fix E0308.
    game_manager: Arc<RwLock<GameManager>>,
) {
    let db = &{ ctx.data.read().await.get::<AppState>().unwrap().db.clone() };

    let player_party_db = match database::pets::get_user_party(db, component.user.id).await {
        Ok(party) if !party.is_empty() => party,
        _ => {
            let builder = EditInteractionResponse::new()
                .content("You must have at least one pet in your party to accept a battle quest.");
            component.edit_response(&ctx.http, builder).await.ok();
            return;
        }
    };

    let enemy_ids: Vec<i32> = quest
        .objective_key
        .split(',')
        .filter_map(|s| s.parse().ok())
        .collect();
    if enemy_ids.is_empty() {
        let builder = EditInteractionResponse::new()
            .content("Quest error: Could not determine enemies for this battle.");
        component.edit_response(&ctx.http, builder).await.ok();
        return;
    }

    let enemy_pets_db = match database::pets::get_pets_by_ids(db, &enemy_ids).await {
        Ok(pets) => pets,
        Err(_) => {
            let builder =
                EditInteractionResponse::new().content("Quest error: Could not load enemy data.");
            component.edit_response(&ctx.http, builder).await.ok();
            return;
        }
    };

    let player_units: Vec<BattleUnit> = player_party_db
        .iter()
        .map(BattleUnit::from_player_pet)
        .collect();
    let enemy_units: Vec<BattleUnit> = enemy_pets_db.iter().map(BattleUnit::from_pet).collect();
    let session = BattleSession::new(player_units, enemy_units);

    let battle_game = BattleGame {
        session,
        party_members: player_party_db,
        node_id: 0,
        node_name: "Quest Battle".to_string(),
        can_afford_recruit: false,
        player_quest_id: Some(quest.player_quest_id),
    };

    interactions::game_handler::start_new_game(
        ctx,
        component,
        game_manager,
        Box::new(battle_game),
        "📜 Quest Accepted! A battle begins!",
    )
    .await;
}
