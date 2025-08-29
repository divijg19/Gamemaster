//! This module contains all functions for interacting with the `profiles` and `inventories` tables.
//! It is the single source of truth for creating, retrieving, and updating user-specific data.

use crate::commands::economy::core::item::Item;
use crate::saga;
use serenity::model::id::UserId;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};

// --- Data Structures ---

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Profile {
    pub balance: i64,
    pub last_work: Option<DateTime<Utc>>,
    pub work_streak: i32,
    pub fishing_xp: i64,
    pub fishing_level: i32,
    pub mining_xp: i64,
    pub mining_level: i32,
    pub coding_xp: i64,
    pub coding_level: i32,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct InventoryItem {
    pub name: String,
    pub quantity: i64,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct SagaProfile {
    pub current_ap: i32,
    pub max_ap: i32,
    pub current_tp: i32,
    pub max_tp: i32,
    pub last_tp_update: DateTime<Utc>,
    pub story_progress: i32,
}

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Pet {
    pub pet_id: i32,
    pub name: String,
    pub description: Option<String>,
    pub base_attack: i32,
    pub base_defense: i32,
    pub base_health: i32,
}

#[allow(dead_code)]
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct PlayerPet {
    pub player_pet_id: i32,
    pub user_id: i64,
    pub pet_id: i32,
    pub nickname: Option<String>,
    pub current_level: i32,
    pub current_xp: i32,
    pub current_attack: i32,
    pub current_defense: i32,
    pub current_health: i32,
    pub is_in_party: bool,
    pub is_training: bool,
    pub training_stat: Option<String>,
    pub training_ends_at: Option<DateTime<Utc>>,
    pub name: String,
}

#[derive(Debug, Default)]
pub struct WorkRewards {
    pub coins: i64,
    pub xp: i64,
    pub items: Vec<(Item, i64)>,
}

pub struct ProgressionUpdate {
    pub job_name: String,
    pub new_level: i32,
    pub new_xp: i64,
}

// --- Database Functions ---

pub async fn get_or_create_profile<'e, E>(
    executor: E,
    user_id: UserId,
) -> Result<Profile, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(
        Profile,
        "WITH ins AS (INSERT INTO profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING) SELECT balance, last_work, work_streak, fishing_xp, fishing_level, mining_xp, mining_level, coding_xp, coding_level FROM profiles WHERE user_id = $1",
        user_id_i64
    ).fetch_one(executor).await
}

// (✓) REMOVED: This function is now redundant and has been removed to resolve the "unused function" warning.
// `update_and_get_saga_profile` is the correct function to use everywhere.

pub async fn update_and_get_saga_profile(
    pool: &PgPool,
    user_id: UserId,
) -> Result<SagaProfile, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let now = Utc::now();
    let completed_pets = sqlx::query_as!(PlayerPet, "SELECT pp.*, p.name FROM player_pets pp JOIN pets p ON pp.pet_id = p.pet_id WHERE pp.user_id = $1 AND pp.is_training = TRUE AND pp.training_ends_at <= $2", user_id_i64, now).fetch_all(pool).await?;
    if !completed_pets.is_empty() {
        let mut tx = pool.begin().await?;
        for pet in completed_pets {
            let (stat_column, stat_gain) = match pet.training_stat.as_deref() {
                Some("attack") => ("current_attack", 1),
                Some("defense") => ("current_defense", 1),
                _ => continue,
            };
            let query_str = format!(
                "UPDATE player_pets SET is_training = FALSE, training_stat = NULL, training_ends_at = NULL, {} = {} + $1 WHERE player_pet_id = $2",
                stat_column, stat_column
            );
            sqlx::query(&query_str)
                .bind(stat_gain)
                .bind(pet.player_pet_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
    }
    let mut tx = pool.begin().await?;
    let initial_profile = sqlx::query_as!(SagaProfile, "WITH ins AS (INSERT INTO player_saga_profile (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING) SELECT current_ap, max_ap, current_tp, max_tp, last_tp_update, story_progress FROM player_saga_profile WHERE user_id = $1 FOR UPDATE", user_id_i64).fetch_one(&mut *tx).await?;
    let (calculated_tp, needs_tp_update) = saga::core::calculate_tp_recharge(&initial_profile);
    let needs_ap_reset = now.date_naive() != initial_profile.last_tp_update.date_naive();
    let calculated_ap = if needs_ap_reset {
        initial_profile.max_ap
    } else {
        initial_profile.current_ap
    };
    if needs_tp_update || needs_ap_reset {
        let updated_profile = sqlx::query_as!(SagaProfile, "UPDATE player_saga_profile SET current_tp = $1, current_ap = $2, last_tp_update = $3 WHERE user_id = $4 RETURNING current_ap, max_ap, current_tp, max_tp, last_tp_update, story_progress", calculated_tp, calculated_ap, now, user_id_i64).fetch_one(&mut *tx).await?;
        tx.commit().await?;
        Ok(updated_profile)
    } else {
        tx.commit().await?;
        Ok(initial_profile)
    }
}

pub async fn get_inventory<'e, E>(
    executor: E,
    user_id: UserId,
) -> Result<Vec<InventoryItem>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(InventoryItem, "SELECT i.name, inv.quantity FROM inventories inv JOIN items i ON inv.item_id = i.item_id WHERE inv.user_id = $1 AND inv.quantity > 0 ORDER BY i.name", user_id_i64).fetch_all(executor).await
}

pub async fn get_inventory_item(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    item: Item,
) -> Result<Option<InventoryItem>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let item_id_i32 = item as i32;
    sqlx::query_as!(InventoryItem, "SELECT i.name, inv.quantity FROM inventories inv JOIN items i ON inv.item_id = i.item_id WHERE inv.user_id = $1 AND inv.item_id = $2 FOR UPDATE", user_id_i64, item_id_i32).fetch_optional(&mut **tx).await
}

pub async fn add_balance(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    amount: i64,
) -> Result<(), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query!(
        "UPDATE profiles SET balance = balance + $1 WHERE user_id = $2",
        amount,
        user_id_i64
    )
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn add_to_inventory(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    item: Item,
    quantity: i64,
) -> Result<(), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let item_id_i32 = item as i32;
    sqlx::query!("INSERT INTO inventories (user_id, item_id, quantity) VALUES ($1, $2, $3) ON CONFLICT (user_id, item_id) DO UPDATE SET quantity = inventories.quantity + $3", user_id_i64, item_id_i32, quantity).execute(&mut **tx).await?;
    Ok(())
}

pub async fn update_work_stats(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    rewards: &WorkRewards,
    new_streak: i32,
    progression: Option<ProgressionUpdate>,
) -> Result<(), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let mut set_clauses: Vec<String> = vec![
        "balance = balance + $1".to_string(),
        "last_work = $2".to_string(),
        "work_streak = $3".to_string(),
    ];
    if let Some(prog) = &progression {
        set_clauses.push(format!("{}_xp = $4", prog.job_name));
        set_clauses.push(format!("{}_level = $5", prog.job_name));
    }
    let query_str = format!(
        "UPDATE profiles SET {} WHERE user_id = ${}",
        set_clauses.join(", "),
        set_clauses.len() + 1
    );
    let mut query = sqlx::query(&query_str)
        .bind(rewards.coins)
        .bind(Utc::now())
        .bind(new_streak);
    if let Some(prog) = progression {
        query = query.bind(prog.new_xp).bind(prog.new_level);
    }
    query.bind(user_id_i64).execute(&mut **tx).await?;
    Ok(())
}

pub async fn spend_action_points(
    pool: &PgPool,
    user_id: UserId,
    amount: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows_affected = sqlx::query!("UPDATE player_saga_profile SET current_ap = current_ap - $1 WHERE user_id = $2 AND current_ap >= $1", amount, user_id_i64).execute(pool).await?.rows_affected();
    Ok(rows_affected > 0)
}

pub async fn get_player_pets(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<PlayerPet>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(PlayerPet, "SELECT pp.*, p.name FROM player_pets pp JOIN pets p ON pp.pet_id = p.pet_id WHERE pp.user_id = $1 ORDER BY pp.is_in_party DESC, pp.current_level DESC", user_id_i64).fetch_all(pool).await
}

pub async fn spend_training_points(
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

pub async fn get_pets_by_ids(pool: &PgPool, pet_ids: &[i32]) -> Result<Vec<Pet>, sqlx::Error> {
    sqlx::query_as!(Pet, "SELECT * FROM pets WHERE pet_id = ANY($1)", pet_ids)
        .fetch_all(pool)
        .await
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
    add_balance(&mut tx, user_id, -cost)
        .await
        .map_err(|_| "Failed to process payment.".to_string())?;
    sqlx::query!("INSERT INTO player_pets (user_id, pet_id, nickname, current_attack, current_defense, current_health) VALUES ($1, $2, $3, $4, $5, $6)", user_id_i64, pet_id, &pet_to_hire.name, pet_to_hire.base_attack, pet_to_hire.base_defense, pet_to_hire.base_health).execute(&mut *tx).await.map_err(|_| "Failed to add mercenary to your army.".to_string())?;
    tx.commit()
        .await
        .map_err(|_| "Failed to finalize the transaction.".to_string())?;
    Ok(pet_to_hire.name)
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
