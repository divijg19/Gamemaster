//! This module contains all database queries related to leaderboards.

use sqlx::{FromRow, PgPool};
// (✓) FIXED: Removed unused import.

/// Represents a single entry in a leaderboard.
#[derive(FromRow, Debug)]
pub struct LeaderboardEntry {
    pub user_id: i64,
    pub score: i64,
}

/// Fetches the top players based on the primary weighted "Gamemaster Score".
///
/// The formula is: (balance / 10) + (work_streak * 50) + (story_progress * 1000)
pub async fn get_gamemaster_leaderboard(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    sqlx::query_as!(
        LeaderboardEntry,
        r#"
        SELECT
            p.user_id,
            (p.balance / 10 + p.work_streak * 50 + COALESCE(psp.story_progress, 0) * 1000) as "score!"
        FROM
            profiles p
        LEFT JOIN
            player_saga_profile psp ON p.user_id = psp.user_id
        ORDER BY
            -- (✓) FIXED: Repeat the calculation in the ORDER BY clause instead of using the alias.
            (p.balance / 10 + p.work_streak * 50 + COALESCE(psp.story_progress, 0) * 1000) DESC
        LIMIT $1;
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}

/// Fetches the top players based on their coin balance.
pub async fn get_wealth_leaderboard(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    sqlx::query_as!(
        LeaderboardEntry,
        r#"
        SELECT user_id, balance as "score!"
        FROM profiles
        ORDER BY balance DESC
        LIMIT $1;
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}

/// Fetches the top players based on their work streak.
pub async fn get_streak_leaderboard(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<LeaderboardEntry>, sqlx::Error> {
    sqlx::query_as!(
        LeaderboardEntry,
        r#"
        SELECT user_id, work_streak as "score!"
        FROM profiles
        ORDER BY work_streak DESC
        LIMIT $1;
        "#,
        limit
    )
    .fetch_all(pool)
    .await
}
