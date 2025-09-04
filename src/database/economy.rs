//! Contains all database functions related to the core player economy.
//! This includes profiles, balances, inventories, and work stats.

use super::models::{InventoryItem, Profile, ProgressionUpdate, WorkRewards};
use sqlx::PgPool;
use crate::commands::economy::core::item::Item;
use serenity::model::id::UserId;
use sqlx::types::chrono::Utc;
use sqlx::{Postgres, Transaction};

/// Retrieves a user's core profile from the database. Creates one if it doesn't exist.
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

/// Retrieves a user's entire inventory from the database.
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

/// Gets the quantity of a single item for a user within a transaction.
pub async fn get_inventory_item(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    item: Item,
) -> Result<Option<InventoryItem>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let item_id_i32 = item as i32;
    sqlx::query_as!(InventoryItem, "SELECT i.name, inv.quantity FROM inventories inv JOIN items i ON inv.item_id = i.item_id WHERE inv.user_id = $1 AND inv.item_id = $2 FOR UPDATE", user_id_i64, item_id_i32).fetch_optional(&mut **tx).await
}

/// Read-only fetch of a single inventory item (no row locking) used for UI displays.
pub async fn get_inventory_item_simple(
    pool: &PgPool,
    user_id: UserId,
    item: Item,
) -> Result<Option<InventoryItem>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let item_id_i32 = item as i32;
    sqlx::query_as!(InventoryItem, "SELECT i.name, inv.quantity FROM inventories inv JOIN items i ON inv.item_id = i.item_id WHERE inv.user_id = $1 AND inv.item_id = $2", user_id_i64, item_id_i32).fetch_optional(pool).await
}

/// Adds or removes from a user's balance within a transaction.
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

/// Atomically adds or removes a quantity of a specific item from a user's inventory within a transaction.
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

/// Atomically updates a user's profile stats in a single, efficient query within a transaction.
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
