//! This module contains all functions for interacting with the `profiles` and `inventories` tables.
//! It is the single source of truth for creating, retrieving, and updating user-specific data.

use crate::commands::economy::core::item::Item;
use serenity::model::id::UserId;
use sqlx::types::chrono::{DateTime, Utc};
use sqlx::{Postgres, Transaction};

/// Represents a user's core economic profile, excluding their inventory.
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

/// Represents a single item in a user's inventory.
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct InventoryItem {
    pub name: String,
    pub quantity: i64,
}

/// A struct to hold all possible rewards from a work command.
#[derive(Debug, Default)]
pub struct WorkRewards {
    pub coins: i64,
    pub xp: i64,
    pub items: Vec<(Item, i64)>,
}

/// A struct to pass updated progression data cleanly to the database function.
pub struct ProgressionUpdate {
    pub job_name: String,
    pub new_level: i32,
    pub new_xp: i64,
}

/// Retrieves a user's profile from the database. Creates one if it doesn't exist.
/// This function is generic over the executor, allowing it to be used with a connection pool or a transaction.
pub async fn get_or_create_profile<'e, E>(
    executor: E,
    user_id: UserId,
) -> Result<Profile, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let user_id_i64 = user_id.get() as i64;
    let profile = sqlx::query_as!(
        Profile,
        r#"
        WITH ins AS ( INSERT INTO profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING )
        SELECT 
            balance, last_work, work_streak, fishing_xp, fishing_level, 
            mining_xp, mining_level, coding_xp, coding_level
        FROM profiles WHERE user_id = $1
        "#,
        user_id_i64
    )
    .fetch_one(executor)
    .await?;
    Ok(profile)
}

/// Retrieves a user's entire inventory from the database.
/// (✓) MODIFIED: This function is now generic over the executor for consistency and flexibility.
pub async fn get_inventory<'e, E>(
    executor: E,
    user_id: UserId,
) -> Result<Vec<InventoryItem>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(
        InventoryItem,
        r#"
        SELECT i.name, inv.quantity
        FROM inventories inv
        JOIN items i ON inv.item_id = i.item_id
        WHERE inv.user_id = $1 AND inv.quantity > 0
        ORDER BY i.name
        "#,
        user_id_i64
    )
    .fetch_all(executor)
    .await
}

/// Gets the quantity of a single item for a user within a transaction.
/// Using `FOR UPDATE` locks the row to prevent race conditions during a sale or trade.
pub async fn get_inventory_item(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    item: Item,
) -> Result<Option<InventoryItem>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let item_id_i32 = item as i32;
    sqlx::query_as!(
        InventoryItem,
        r#"
        SELECT i.name, inv.quantity FROM inventories inv 
        JOIN items i ON inv.item_id = i.item_id 
        WHERE inv.user_id = $1 AND inv.item_id = $2 FOR UPDATE
        "#,
        user_id_i64,
        item_id_i32
    )
    .fetch_optional(&mut **tx)
    .await
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
    sqlx::query!(
        r#"
        INSERT INTO inventories (user_id, item_id, quantity) VALUES ($1, $2, $3)
        ON CONFLICT (user_id, item_id) DO UPDATE SET quantity = inventories.quantity + $3
        "#,
        user_id_i64,
        item_id_i32,
        quantity
    )
    .execute(&mut **tx)
    .await?;
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

    // Dynamically build the query to avoid complex logic with many NULLs.
    let mut set_clauses: Vec<String> = vec![
        "balance = balance + $1".to_string(),
        "last_work = $2".to_string(),
        "work_streak = $3".to_string(),
    ];

    if let Some(prog) = &progression {
        // Use placeholder numbers that will be bound later.
        set_clauses.push(format!("{}_xp = $4", prog.job_name));
        set_clauses.push(format!("{}_level = $5", prog.job_name));
    }

    let query_str = format!(
        "UPDATE profiles SET {} WHERE user_id = ${}",
        set_clauses.join(", "),
        set_clauses.len() + 1 // The user_id will be the last parameter.
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
