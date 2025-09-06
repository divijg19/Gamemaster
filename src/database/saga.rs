//! Contains all database functions related to core saga progression.
//! This includes AP, TP, and story progress.

use super::models::{PlayerUnit, SagaProfile, UnitRarity};
use crate::saga;
use serenity::model::id::UserId;
use sqlx::PgPool;
use sqlx::types::chrono::Utc;

/// Fetches a user's Saga Profile, automatically updating their AP, TP, and completed training.
/// This is the primary function that should be used to get a player's up-to-date game state.
pub async fn update_and_get_saga_profile(
    pool: &PgPool,
    user_id: UserId,
) -> Result<SagaProfile, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let now = Utc::now();

    // First, check for and apply any completed training sessions.
    let completed_units = sqlx::query_as!(
        PlayerUnit,
        r#"SELECT 
        pu.player_unit_id, pu.user_id, pu.unit_id, pu.nickname, pu.current_level, pu.current_xp,
        pu.current_attack, pu.current_defense, pu.current_health, pu.is_in_party, pu.is_training,
        pu.training_stat, pu.training_ends_at, u.name, pu.rarity as "rarity: UnitRarity"
        FROM player_units pu JOIN units u ON pu.unit_id = u.unit_id 
        WHERE pu.user_id = $1 AND pu.is_training = TRUE AND pu.training_ends_at <= $2"#,
        user_id_i64,
        now
    )
    .fetch_all(pool)
    .await?;
    if !completed_units.is_empty() {
        let mut tx = pool.begin().await?;
        for unit in completed_units {
            let (stat_column, stat_gain) = match unit.training_stat.as_deref() {
                Some("attack") => ("current_attack", 1),
                Some("defense") => ("current_defense", 1),
                _ => continue,
            };
            let query_str = format!(
                "UPDATE player_units SET is_training = FALSE, training_stat = NULL, training_ends_at = NULL, {} = {} + $1 WHERE player_unit_id = $2",
                stat_column, stat_column
            );
            sqlx::query(&query_str)
                .bind(stat_gain)
                .bind(unit.player_unit_id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
    }

    // Next, update AP and TP in a single transaction.
    let mut tx = pool.begin().await?;
    // (✓) Robust UPSERT pattern: attempt insert and capture row with RETURNING. If the row
    // already exists, fall back to SELECT .. FOR UPDATE. The previous pattern using a CTE +
    // SELECT could (rarely) surface RowNotFound under high concurrency if the planner skipped
    // the write path. This pattern guarantees we either obtain the freshly inserted row or
    // lock the existing one.
    let initial_profile = if let Some(inserted) = sqlx::query_as!(
        SagaProfile,
        "INSERT INTO player_saga_profile (user_id) VALUES ($1) ON CONFLICT (user_id) DO NOTHING RETURNING current_ap, max_ap, current_tp, max_tp, last_tp_update, story_progress",
        user_id_i64
    )
    .fetch_optional(&mut *tx)
    .await? {
        inserted
    } else {
        sqlx::query_as!(
            SagaProfile,
            "SELECT current_ap, max_ap, current_tp, max_tp, last_tp_update, story_progress FROM player_saga_profile WHERE user_id = $1 FOR UPDATE",
            user_id_i64
        )
        .fetch_one(&mut *tx)
        .await?
    };

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

/// Same as `update_and_get_saga_profile` but also returns the user's full unit list
/// in the same logical flow to reduce round trips when both are required.
pub async fn update_get_profile_and_units(
    pool: &PgPool,
    user_id: UserId,
) -> Result<(SagaProfile, Vec<PlayerUnit>), sqlx::Error> {
    let profile = update_and_get_saga_profile(pool, user_id).await?;
    // Fetch units after training completion & AP/TP update so stats reflect post-training values.
    let units = sqlx::query_as!(
        PlayerUnit,
        r#"SELECT
        pu.player_unit_id, pu.user_id, pu.unit_id, pu.nickname, pu.current_level, pu.current_xp,
        pu.current_attack, pu.current_defense, pu.current_health, pu.is_in_party, pu.is_training,
        pu.training_stat, pu.training_ends_at, u.name, pu.rarity as "rarity: UnitRarity"
        FROM player_units pu JOIN units u ON pu.unit_id = u.unit_id
        WHERE pu.user_id = $1
        ORDER BY pu.is_in_party DESC, pu.current_level DESC"#,
        user_id.get() as i64
    )
    .fetch_all(pool)
    .await?;
    Ok((profile, units))
}

/// Atomically spends a player's Action Points.
pub async fn spend_action_points(
    pool: &PgPool,
    user_id: UserId,
    amount: i32,
) -> Result<bool, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let rows_affected = sqlx::query!("UPDATE player_saga_profile SET current_ap = current_ap - $1 WHERE user_id = $2 AND current_ap >= $1", amount, user_id_i64).execute(pool).await?.rows_affected();
    Ok(rows_affected > 0)
}

/// Advances a player's story progress if their current progress is lower than the new value.
pub async fn advance_story_progress(
    pool: &PgPool,
    user_id: UserId,
    new_progress: i32,
) -> Result<(), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query!("UPDATE player_saga_profile SET story_progress = $1 WHERE user_id = $2 AND story_progress < $1", new_progress, user_id_i64).execute(pool).await?;
    Ok(())
}
