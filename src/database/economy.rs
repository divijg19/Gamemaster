//! Contains all database functions related to the core player economy.
//! This includes profiles, balances, inventories, and work stats.

use super::models::{InventoryItem, Profile, ProgressionUpdate, WorkRewards};
use crate::commands::economy::core::item::Item;
use serenity::model::id::UserId;
use sqlx::PgPool;
use sqlx::{Postgres, Transaction};

/// Retrieves a user's entire inventory from the database.
pub async fn get_inventory(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<InventoryItem>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(InventoryItem, "SELECT i.name, inv.quantity FROM inventories inv JOIN items i ON inv.item_id = i.item_id WHERE inv.user_id = $1 AND inv.quantity > 0 ORDER BY i.name", user_id_i64).fetch_all(pool).await
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
pub async fn get_or_create_profile(pool: &PgPool, user_id: UserId) -> Result<Profile, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query_as!(
        Profile,
        "WITH ins AS (INSERT INTO profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING) SELECT balance, last_work, work_streak, fishing_xp, fishing_level, mining_xp, mining_level, coding_xp, coding_level FROM profiles WHERE user_id = $1",
        user_id_i64
    )
    .fetch_one(pool)
    .await
}

/// Adds (or subtracts) coins from a user's balance within an existing transaction.
/// Returns Ok(()) on success or Err if the update failed (e.g., insufficient funds when negative result).
pub async fn add_balance(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    delta: i64,
) -> Result<(), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    // Prevent negative balances.
    sqlx::query!(
        "UPDATE profiles SET balance = balance + $2 WHERE user_id = $1 AND balance + $2 >= 0",
        user_id_i64,
        delta
    )
    .execute(&mut **tx)
    .await
    .and_then(|res| {
        if res.rows_affected() == 1 {
            Ok(())
        } else {
            Err(sqlx::Error::RowNotFound)
        }
    })
}

/// Adds (or removes when negative) a quantity of an item to a user's inventory atomically.
/// Will insert the item row if it doesn't exist and quantity is positive.
pub async fn add_to_inventory(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    item: Item,
    delta_qty: i64,
) -> Result<(), sqlx::Error> {
    if delta_qty == 0 {
        return Ok(());
    }
    let user_id_i64 = user_id.get() as i64;
    let item_id_i32 = item as i32;
    if delta_qty > 0 {
        sqlx::query!(
            r#"INSERT INTO inventories (user_id, item_id, quantity)
            VALUES ($1, $2, $3)
            ON CONFLICT (user_id, item_id) DO UPDATE SET quantity = inventories.quantity + EXCLUDED.quantity"#,
            user_id_i64,
            item_id_i32,
            delta_qty
        )
        .execute(&mut **tx)
        .await?;
        Ok(())
    } else {
        // Negative adjustment: ensure sufficient quantity.
        sqlx::query!(
            "UPDATE inventories SET quantity = quantity + $3 WHERE user_id = $1 AND item_id = $2 AND quantity + $3 >= 0",
            user_id_i64,
            item_id_i32,
            delta_qty
        )
        .execute(&mut **tx)
        .await
        .and_then(|res| if res.rows_affected() == 1 { Ok(()) } else { Err(sqlx::Error::RowNotFound) })
    }
}

/// Updates work stats (streak, last_work timestamp) and applies rewards: coins, xp, items.
/// Returns (new_profile, progression_updates) where progression_updates contains any level ups.
pub async fn update_work_stats(
    tx: &mut Transaction<'_, Postgres>,
    user_id: UserId,
    rewards: &WorkRewards,
    job_name: &str,
) -> Result<(Profile, Vec<ProgressionUpdate>), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    if rewards.coins != 0 {
        add_balance(tx, user_id, rewards.coins).await?;
    }
    for (item, qty) in &rewards.items {
        add_to_inventory(tx, user_id, *item, *qty).await?;
    }
    // Fetch current profile FOR UPDATE.
    let mut profile = sqlx::query_as!(
        Profile,
        "SELECT balance, last_work, work_streak, fishing_xp, fishing_level, mining_xp, mining_level, coding_xp, coding_level FROM profiles WHERE user_id = $1 FOR UPDATE",
        user_id_i64
    )
    .fetch_one(&mut **tx)
    .await?;

    let mut progress_updates = Vec::new();
    // Closure to process xp + potential level up.
    let mut apply_xp = |job_name: &str, xp_field: &mut i64, lvl_field: &mut i32, gained: i64| {
        if gained <= 0 {
            return;
        }
        *xp_field += gained;
        let mut leveled = false;
        loop {
            let needed = (*lvl_field as i64 * 100).max(100);
            if *xp_field >= needed {
                *xp_field -= needed;
                *lvl_field += 1;
                leveled = true;
            } else {
                break;
            }
        }
        if leveled {
            progress_updates.push(ProgressionUpdate {
                job_name: job_name.to_string(),
                new_level: *lvl_field,
                new_xp: *xp_field,
            });
        }
    };
    match job_name {
        "fishing" => apply_xp(
            "Fishing",
            &mut profile.fishing_xp,
            &mut profile.fishing_level,
            rewards.xp,
        ),
        "mining" => apply_xp(
            "Mining",
            &mut profile.mining_xp,
            &mut profile.mining_level,
            rewards.xp,
        ),
        "coding" => apply_xp(
            "Coding",
            &mut profile.coding_xp,
            &mut profile.coding_level,
            rewards.xp,
        ),
        other => {
            tracing::warn!(target="economy.work", job=%other, "Unknown job name passed to update_work_stats; defaulting xp to fishing");
            apply_xp(
                "Fishing",
                &mut profile.fishing_xp,
                &mut profile.fishing_level,
                rewards.xp,
            );
        }
    };

    // Update streak and timestamp.
    sqlx::query!(
        "UPDATE profiles SET balance = $2, last_work = NOW(), work_streak = CASE WHEN last_work IS NULL OR last_work < NOW() - INTERVAL '1 day' THEN 1 ELSE work_streak + 1 END, fishing_xp = $3, fishing_level = $4, mining_xp = $5, mining_level = $6, coding_xp = $7, coding_level = $8 WHERE user_id = $1",
        user_id_i64,
        profile.balance,
        profile.fishing_xp,
        profile.fishing_level,
        profile.mining_xp,
        profile.mining_level,
        profile.coding_xp,
        profile.coding_level
    )
    .execute(&mut **tx)
    .await?;
    Ok((profile, progress_updates))
}

/// Backwards-compatible helper for callers expecting a simple item fetch w/o transaction.
pub async fn get_inventory_item_simple(
    pool: &PgPool,
    user_id: UserId,
    item: Item,
) -> Result<Option<InventoryItem>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let item_id_i32 = item as i32;
    sqlx::query_as!(InventoryItem, "SELECT i.name, inv.quantity FROM inventories inv JOIN items i ON inv.item_id = i.item_id WHERE inv.user_id = $1 AND inv.item_id = $2", user_id_i64, item_id_i32).fetch_optional(pool).await
}
