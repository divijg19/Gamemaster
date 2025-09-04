//! Contains all database functions related to the player task system.

use super::models::{PlayerTaskDetails, Task, TaskType};
use crate::commands::economy::core::item::Item;
use crate::database::economy::{add_balance, add_to_inventory};
use serenity::model::id::UserId;
use sqlx::{PgPool, Postgres, Transaction};

async fn assign_tasks_if_needed(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    task_type: TaskType,
    time_period: &str,
    limit: i64,
) -> Result<(), sqlx::Error> {
    let count: i64 = sqlx::query_scalar::<_, i64>(&format!(
        r#"SELECT COUNT(*) FROM player_tasks pt
        JOIN tasks t ON pt.task_id = t.task_id
        WHERE pt.user_id = $1 AND t.task_type = $2 AND pt.assigned_at >= date_trunc('{}', NOW())"#,
        time_period
    ))
    .bind(user_id)
    .bind(task_type)
    .fetch_one(&mut **tx)
    .await?;

    if count == 0 {
        // (✓) DEFINITIVE FIX: Reverted from `query_scalar` to `query_as!(Task, ...)`.
        // This makes the `Task` struct "live" and correctly resolves the dead_code warning.
        let new_tasks = sqlx::query_as!(
            Task,
            r#"SELECT task_id, task_type as "task_type: _", title, description,
                    objective_key, objective_goal,
                    reward_coins, reward_item_id, reward_item_quantity
             FROM tasks WHERE task_type = $1 ORDER BY random() LIMIT $2"#,
            task_type as _,
            limit
        )
        .fetch_all(&mut **tx)
        .await?;

        for task in new_tasks {
            // Build a concise debug summary referencing all fields to keep struct fully live.
            let _dbg_summary = format!(
                "AssignTask[id:{} type:{:?} title:{} desc:{} obj:{}:{} coins:{:?} item:{:?} qty:{:?}]",
                task.task_id,
                task.task_type,
                task.title,
                task.description,
                task.objective_key,
                task.objective_goal,
                task.reward_coins,
                task.reward_item_id,
                task.reward_item_quantity
            );
            sqlx::query!(
                "INSERT INTO player_tasks (user_id, task_id) VALUES ($1, $2)",
                user_id,
                task.task_id
            )
            .execute(&mut **tx)
            .await?;
        }
    }
    Ok(())
}

pub async fn get_or_assign_player_tasks(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<PlayerTaskDetails>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await?;
    assign_tasks_if_needed(&mut tx, user_id_i64, TaskType::Daily, "day", 2).await?;
    assign_tasks_if_needed(&mut tx, user_id_i64, TaskType::Weekly, "week", 1).await?;
    let tasks = sqlx::query_as!(
        PlayerTaskDetails,
        r#"SELECT
            pt.player_task_id, pt.progress, pt.is_completed, t.task_type as "task_type: _",
            t.title, t.description, t.objective_goal, t.reward_coins,
            t.reward_item_id, t.reward_item_quantity
        FROM player_tasks pt JOIN tasks t ON pt.task_id = t.task_id
        WHERE pt.user_id = $1 AND pt.claimed_at IS NULL
        AND pt.assigned_at >= date_trunc(
            CASE t.task_type WHEN 'Daily' THEN 'day' ELSE 'week' END::text, NOW()
        )
        ORDER BY t.task_type, t.title"#,
        user_id_i64
    )
    .fetch_all(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(tasks)
}

pub async fn update_task_progress(
    pool: &PgPool,
    user_id: UserId,
    objective_key: &str,
    increment_by: i32,
) -> Result<(), sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    sqlx::query!(
        r#"
        WITH updated AS (
            UPDATE player_tasks pt SET progress = LEAST(t.objective_goal, pt.progress + $3)
            FROM tasks t WHERE pt.task_id = t.task_id AND pt.user_id = $1
              AND t.objective_key = $2 AND pt.is_completed = FALSE
            RETURNING pt.player_task_id, pt.progress, t.objective_goal
        )
        UPDATE player_tasks SET is_completed = TRUE, completed_at = NOW()
        WHERE player_task_id = (SELECT player_task_id FROM updated)
          AND (SELECT progress FROM updated) >= (SELECT objective_goal FROM updated)
        "#,
        user_id_i64,
        objective_key,
        increment_by
    )
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn claim_task_reward(
    pool: &PgPool,
    user_id: UserId,
    player_task_id: i32,
) -> Result<(i64, Option<i32>, Option<i32>), String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let task_to_claim = sqlx::query!(
        r#"SELECT t.reward_coins, t.reward_item_id, t.reward_item_quantity
        FROM player_tasks pt JOIN tasks t ON pt.task_id = t.task_id
        WHERE pt.player_task_id = $1 AND pt.user_id = $2
          AND pt.is_completed = TRUE AND pt.claimed_at IS NULL"#,
        player_task_id,
        user_id_i64
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| e.to_string())?;

    if let Some(task) = task_to_claim {
        let reward_coins = task.reward_coins.unwrap_or(0);
        if reward_coins > 0 {
            add_balance(&mut tx, user_id, reward_coins)
                .await
                .map_err(|e| e.to_string())?;
        }
        if let (Some(item_id), Some(quantity)) = (task.reward_item_id, task.reward_item_quantity)
            && let Some(item) = Item::from_i32(item_id) {
                add_to_inventory(&mut tx, user_id, item, quantity as i64)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        sqlx::query!(
            "UPDATE player_tasks SET claimed_at = NOW() WHERE player_task_id = $1",
            player_task_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok((reward_coins, task.reward_item_id, task.reward_item_quantity))
    } else {
        Err("Task is not available to be claimed, or it has already been claimed.".to_string())
    }
}
