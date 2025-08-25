//! This module contains all functions for interacting with the `profiles` table in the database.
//! It is the single source of truth for creating, retrieving, and updating user-specific data.

use crate::database::init::DbPool;
use chrono::{DateTime, Utc};
use serenity::model::id::UserId;

/// Represents a user's economic profile as stored in the database.
/// The `user_id` is excluded as it's the key we use to fetch this data, not part of the data itself.
#[derive(sqlx::FromRow, Debug)]
pub struct Profile {
    // (✓) CORRECTED: The `user_id` field has been removed to resolve the dead_code warning.
    pub balance: i64,
    pub last_work: Option<DateTime<Utc>>,
    pub fish: i64,
    pub ores: i64,
    pub gems: i64,
    pub rare_finds: i64,
}

/// A struct to hold all possible rewards from a work command.
#[derive(Debug, Default)]
pub struct WorkRewards {
    pub coins: i64,
    pub fish: i64,
    pub ores: i64,
    pub gems: i64,
    pub rare_finds: i64,
}

/// Retrieves a user's profile from the database.
///
/// If a profile for the given `user_id` does not exist, a new one is created
/// with default values and then returned.
pub async fn get_or_create_profile(pool: &DbPool, user_id: UserId) -> Result<Profile, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;

    // First, attempt to insert. If the user_id already exists, ON CONFLICT does nothing.
    sqlx::query("INSERT INTO profiles (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING")
        .bind(user_id_i64)
        .execute(pool)
        .await?;

    // (✓) CORRECTED: The query no longer uses `SELECT *`. Instead, it explicitly selects
    // only the columns that are present in the `Profile` struct, making it more efficient.
    sqlx::query_as::<_, Profile>(
        "SELECT balance, last_work, fish, ores, gems, rare_finds FROM profiles WHERE user_id = $1",
    )
    .bind(user_id_i64)
    .fetch_one(pool)
    .await
}

/// Atomically updates a user's profile with all rewards from a work session.
pub async fn update_work_rewards(
    pool: &DbPool,
    user_id: UserId,
    rewards: &WorkRewards,
) -> Result<(), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query(
        "UPDATE profiles SET
            balance = balance + $1,
            fish = fish + $2,
            ores = ores + $3,
            gems = gems + $4,
            rare_finds = rare_finds + $5,
            last_work = $6
        WHERE user_id = $7",
    )
    .bind(rewards.coins)
    .bind(rewards.fish)
    .bind(rewards.ores)
    .bind(rewards.gems)
    .bind(rewards.rare_finds)
    .bind(Utc::now())
    .bind(user_id_i64)
    .execute(pool)
    .await?;
    Ok(())
}
