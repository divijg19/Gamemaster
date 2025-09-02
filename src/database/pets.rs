//! Contains all database functions related to pets.
//! This includes hiring, taming, training, and managing party status.

use super::economy::{add_balance, add_to_inventory, get_inventory_item};
use super::models::{Pet, PlayerPet, Profile};
use crate::commands::economy::core::item::Item;
use crate::saga;
use crate::saga::leveling::LevelUpResult;
use serenity::model::id::UserId;
use sqlx::types::chrono::Utc;
use sqlx::{PgPool, Postgres, Transaction};
use std::str::FromStr;

/// Fetches all pets owned by a player, joining with the master pet table to get species names.
pub async fn get_player_pets(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<PlayerPet>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(PlayerPet, "SELECT pp.*, p.name FROM player_pets pp JOIN pets p ON pp.pet_id = p.pet_id WHERE pp.user_id = $1 ORDER BY pp.is_in_party DESC, pp.current_level DESC", user_id_i64).fetch_all(pool).await
}

/// Fetches only the pets that are currently in the player's active party.
pub async fn get_user_party(pool: &PgPool, user_id: UserId) -> Result<Vec<PlayerPet>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(PlayerPet, "SELECT pp.*, p.name FROM player_pets pp JOIN pets p ON pp.pet_id = p.pet_id WHERE pp.user_id = $1 AND pp.is_in_party = TRUE ORDER BY pp.player_pet_id", user_id_i64).fetch_all(pool).await
}

/// Fetches the master data for a list of pets by their IDs.
pub async fn get_pets_by_ids(pool: &PgPool, pet_ids: &[i32]) -> Result<Vec<Pet>, sqlx::Error> {
    sqlx::query_as!(Pet, "SELECT * FROM pets WHERE pet_id = ANY($1)", pet_ids)
        .fetch_all(pool)
        .await
}

pub async fn can_afford_tame(pool: &PgPool, user_id: UserId) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let lure_item_id = Item::TamingLure as i32;
    let count = sqlx::query_scalar!(
        "SELECT quantity FROM inventories WHERE user_id = $1 AND item_id = $2",
        user_id_i64,
        lure_item_id
    )
    .fetch_optional(pool)
    .await?
    .unwrap_or(0);
    Ok(count >= 1)
}

pub async fn hire_mercenary(
    pool: &PgPool,
    user_id: UserId,
    pet_id: i32,
    cost: i64,
) -> Result<String, String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let profile = sqlx::query_as!(Profile, "SELECT balance, last_work, work_streak, fishing_xp, fishing_level, mining_xp, mining_level, coding_xp, coding_level FROM profiles WHERE user_id = $1 FOR UPDATE", user_id_i64).fetch_one(&mut *tx).await.map_err(|_| "Could not find your profile.".to_string())?;
    if profile.balance < cost {
        tx.rollback().await.ok();
        return Err("You don't have enough coins.".to_string());
    }
    let army_size: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM player_pets WHERE user_id = $1",
        user_id_i64
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);
    if army_size >= 10 {
        tx.rollback().await.ok();
        return Err("Your army is full (10/10).".to_string());
    }
    let pet_to_hire = sqlx::query_as!(Pet, "SELECT * FROM pets WHERE pet_id = $1", pet_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "This mercenary is no longer available.".to_string())?;
    // (✓) FIXED: Pass `&mut tx` to your helper function.
    add_balance(&mut tx, user_id, -cost)
        .await
        .map_err(|_| "Failed to process payment.".to_string())?;
    sqlx::query!("INSERT INTO player_pets (user_id, pet_id, nickname, current_attack, current_defense, current_health) VALUES ($1, $2, $3, $4, $5, $6)", user_id_i64, pet_id, &pet_to_hire.name, pet_to_hire.base_attack, pet_to_hire.base_defense, pet_to_hire.base_health).execute(&mut *tx).await.map_err(|_| "Failed to add mercenary to your army.".to_string())?;
    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;
    Ok(pet_to_hire.name)
}

pub async fn attempt_tame_pet(
    pool: &PgPool,
    user_id: UserId,
    pet_id_to_tame: i32,
) -> Result<String, String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let pet_master = sqlx::query_as!(Pet, "SELECT * FROM pets WHERE pet_id = $1", pet_id_to_tame)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| "Creature data not found.".to_string())?;
    if !pet_master.is_tameable {
        tx.rollback().await.ok();
        return Err("This creature cannot be tamed.".to_string());
    }
    let army_size: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM player_pets WHERE user_id = $1",
        user_id_i64
    )
    .fetch_one(&mut *tx)
    .await
    .unwrap_or(Some(0))
    .unwrap_or(0);
    if army_size >= 10 {
        tx.rollback().await.ok();
        return Err(
            "Your army is full! You must dismiss a pet before taming a new one.".to_string(),
        );
    }
    let research_data_item_name = format!("{} Research Data", pet_master.name);
    let research_data_item = Item::from_str(&research_data_item_name)
        .map_err(|_| "Could not identify the required research data.".to_string())?;
    let required_items = [(Item::TamingLure, 1), (research_data_item, 10)];
    for (item, required_quantity) in required_items {
        // (✓) FIXED: Pass `&mut tx` to your helper function.
        let has_item = get_inventory_item(&mut tx, user_id, item)
            .await
            .map_err(|_| "Could not check your inventory.".to_string())?;
        if has_item.is_none() || has_item.unwrap().quantity < required_quantity {
            tx.rollback().await.ok();
            return Err(format!(
                "You don't have enough materials! You need {} {}.",
                required_quantity,
                item.display_name()
            ));
        }
    }
    for (item, required_quantity) in required_items {
        // (✓) FIXED: Pass `&mut tx` to your helper function.
        add_to_inventory(&mut tx, user_id, item, -required_quantity)
            .await
            .map_err(|_| "Failed to consume taming items.".to_string())?;
    }
    sqlx::query!("INSERT INTO player_pets (user_id, pet_id, nickname, current_attack, current_defense, current_health) VALUES ($1, $2, $3, $4, $5, $6)", user_id_i64, pet_id_to_tame, &pet_master.name, pet_master.base_attack, pet_master.base_defense, pet_master.base_health).execute(&mut *tx).await.map_err(|_| "Failed to add the tamed pet to your army.".to_string())?;
    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;
    Ok(pet_master.name)
}

async fn spend_training_points(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    amount: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows_affected = sqlx::query!("UPDATE player_saga_profile SET current_tp = current_tp - $1 WHERE user_id = $2 AND current_tp >= $1", amount, user_id_i64).execute(&mut **tx).await?.rows_affected();
    Ok(rows_affected > 0)
}

pub async fn start_training(
    pool: &PgPool,
    user_id: UserId,
    player_pet_id: i32,
    stat_to_train: &str,
    duration_hours: i64,
    tp_cost: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await?;
    if !spend_training_points(&mut tx, user_id, tp_cost).await? {
        tx.rollback().await?;
        return Ok(false);
    }
    let training_ends = Utc::now() + chrono::Duration::hours(duration_hours);
    let rows_affected = sqlx::query!("UPDATE player_pets SET is_training = TRUE, training_stat = $1, training_ends_at = $2 WHERE player_pet_id = $3 AND user_id = $4", stat_to_train, training_ends, player_pet_id, user_id_i64).execute(&mut *tx).await?.rows_affected();
    if rows_affected > 0 {
        tx.commit().await?;
        Ok(true)
    } else {
        tx.rollback().await?;
        Ok(false)
    }
}

pub async fn set_pet_party_status(
    pool: &PgPool,
    user_id: UserId,
    player_pet_id: i32,
    is_in_party: bool,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await?;
    if is_in_party {
        let party_size: i64 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM player_pets WHERE user_id = $1 AND is_in_party = TRUE",
            user_id_i64
        )
        .fetch_one(&mut *tx)
        .await?
        .unwrap_or(0);
        if party_size >= 5 {
            tx.rollback().await?;
            return Ok(false);
        }
    }
    let rows_affected = sqlx::query!(
        "UPDATE player_pets SET is_in_party = $1 WHERE player_pet_id = $2 AND user_id = $3",
        is_in_party,
        player_pet_id,
        user_id_i64
    )
    .execute(&mut *tx)
    .await?
    .rows_affected();
    if rows_affected > 0 {
        tx.commit().await?;
        Ok(true)
    } else {
        tx.rollback().await?;
        Ok(false)
    }
}

pub async fn apply_battle_rewards(
    pool: &PgPool,
    user_id: UserId,
    coins: i64,
    loot: &[(Item, i64)],
    pets_in_battle: &[PlayerPet],
    xp_per_pet: i32,
) -> Result<Vec<LevelUpResult>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    if coins > 0 {
        add_balance(&mut tx, user_id, coins).await?;
    }
    for (item, quantity) in loot {
        add_to_inventory(&mut tx, user_id, *item, *quantity).await?;
    }
    let mut level_up_results = Vec::new();
    for pet in pets_in_battle {
        let level_result = saga::leveling::handle_pet_leveling(pet, xp_per_pet);
        if level_result.did_level_up {
            sqlx::query!("UPDATE player_pets SET current_level = $1, current_xp = $2, current_attack = current_attack + $3, current_defense = current_defense + $4, current_health = current_health + $5 WHERE player_pet_id = $6", level_result.new_level, level_result.new_xp, level_result.stat_gains.0, level_result.stat_gains.1, level_result.stat_gains.2, pet.player_pet_id).execute(&mut *tx).await?;
        } else {
            sqlx::query!(
                "UPDATE player_pets SET current_xp = $1 WHERE player_pet_id = $2",
                level_result.new_xp,
                pet.player_pet_id
            )
            .execute(&mut *tx)
            .await?;
        }
        level_up_results.push(level_result);
    }
    tx.commit().await?;
    Ok(level_up_results)
}

pub async fn dismiss_pet(
    pool: &PgPool,
    user_id: UserId,
    player_pet_id: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows_affected = sqlx::query!(
        "DELETE FROM player_pets WHERE player_pet_id = $1 AND user_id = $2",
        player_pet_id,
        user_id_i64
    )
    .execute(pool)
    .await?
    .rows_affected();
    Ok(rows_affected > 0)
}
