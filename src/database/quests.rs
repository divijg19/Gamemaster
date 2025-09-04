//! Contains all database functions related to the player quest system.

use super::models::{PlayerQuest, PlayerQuestStatus, Quest, QuestDetails, QuestReward, QuestType};
use crate::{commands::economy::core::item::Item, database};
use serenity::model::id::UserId;
use sqlx::PgPool;

/// A struct that combines quest details with its list of rewards for UI display.
#[derive(Debug, Clone)]
pub struct QuestBoardEntry {
    pub details: QuestDetails,
    pub rewards: Vec<QuestReward>,
}

/// A struct to hold the full details of an accepted quest, needed to initiate gameplay.
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct AcceptedQuest {
    pub player_quest_id: i32,
    pub quest_type: QuestType,
    pub objective_key: String,
}

const QUEST_BOARD_SIZE: i64 = 3;

/// Gets the player's current quest board. If the board is empty, it refreshes it.
pub async fn get_or_refresh_quest_board(
    pool: &PgPool,
    user_id: UserId,
) -> Result<Vec<QuestBoardEntry>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    // Lightweight check first (avoid opening a write tx when not needed)
    let offered_count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM player_quests WHERE user_id = $1 AND status = 'Offered'",
        user_id_i64
    )
    .fetch_one(pool)
    .await?
    .unwrap_or(0);

    if offered_count == 0 {
        let mut tx = pool.begin().await?;
        let new_quests = sqlx::query_as!(
            Quest,
            r#"SELECT quest_id, title, description,
        COALESCE(giver_name,'Unknown') AS "giver_name!",
        COALESCE(difficulty,'Normal') AS "difficulty!",
                quest_type as "quest_type: _", objective_key
            FROM quests WHERE quest_id NOT IN (SELECT quest_id FROM player_quests WHERE user_id = $1)
            ORDER BY random() LIMIT $2"#,
            user_id_i64,
            QUEST_BOARD_SIZE
        )
        .fetch_all(&mut *tx)
        .await?;
        for quest in new_quests {
            // Debug summary touches all raw Quest fields to keep struct live.
            let _q_dbg = format!(
                "QuestLoad[id:{} title:{} giver:{} diff:{} type:{:?} obj:{}] desc:{}",
                quest.quest_id,
                quest.title,
                quest.giver_name,
                quest.difficulty,
                quest.quest_type,
                quest.objective_key,
                quest.description
            );
            sqlx::query!(
                "INSERT INTO player_quests (user_id, quest_id, status) VALUES ($1, $2, 'Offered')",
                user_id_i64,
                quest.quest_id
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
    }
    get_player_quests_with_details(pool, user_id, PlayerQuestStatus::Offered).await
}

/// Fetches a player's quests with a given status, including their full details and rewards.
pub async fn get_player_quests_with_details(
    pool: &PgPool,
    user_id: UserId,
    status: PlayerQuestStatus,
) -> Result<Vec<QuestBoardEntry>, sqlx::Error> {
    let user_id_i64 = user_id.get() as i64;
    // Pure read path; no explicit transaction needed.
    let details_list = sqlx::query_as!(
        QuestDetails,
        r#"SELECT pq.player_quest_id, pq.status as "status: _", q.title, q.description,
            COALESCE(q.giver_name,'Unknown') AS "giver_name!",
            COALESCE(q.difficulty,'Normal') AS "difficulty!"
        FROM player_quests pq JOIN quests q ON pq.quest_id = q.quest_id
        WHERE pq.user_id = $1 AND pq.status = $2
        ORDER BY pq.accepted_at DESC, pq.completed_at DESC"#,
        user_id_i64,
        status as _
    )
    .fetch_all(pool)
    .await?;

    // Fetch rewards per quest id (N small queries). Could batch if hot path.
    let mut full_entries = Vec::with_capacity(details_list.len());
    for details in details_list {
        let rewards = sqlx::query_as!(
            QuestReward,
            "SELECT quest_reward_id, quest_id, reward_coins, reward_item_id, reward_item_quantity FROM quest_rewards WHERE quest_id = (SELECT quest_id FROM player_quests WHERE player_quest_id = $1)",
            details.player_quest_id
        )
        .fetch_all(pool)
        .await?;
        full_entries.push(QuestBoardEntry { details, rewards });
    }
    Ok(full_entries)
}

/// Accepts a quest for a player, changing its status from 'Offered' to 'Accepted'.
pub async fn accept_quest(
    pool: &PgPool,
    user_id: UserId,
    player_quest_id: i32,
) -> Result<AcceptedQuest, String> {
    // NOTE: Schema currently lacks accepted_at/completed_at; if added later update the RETURNING clause & mutations.
    let user_id_i64 = user_id.get() as i64;
    let accepted_quest = sqlx::query_as!(AcceptedQuest, r#"UPDATE player_quests pq SET status = 'Accepted', accepted_at = NOW() FROM quests q WHERE pq.player_quest_id = $1 AND pq.user_id = $2 AND pq.status = 'Offered' AND pq.quest_id = q.quest_id RETURNING pq.player_quest_id, q.quest_type as "quest_type: _", q.objective_key as "objective_key!""#, player_quest_id, user_id_i64)
        .fetch_optional(pool).await.map_err(|e| e.to_string())?;
    accepted_quest.ok_or_else(|| {
        "Could not accept this quest. It may be expired or was not offered to you.".to_string()
    })
}

/// Marks a quest as complete and distributes its rewards in a single transaction.
pub async fn complete_quest(
    pool: &PgPool,
    user_id: UserId,
    player_quest_id: i32,
) -> Result<(), String> {
    let user_id_i64 = user_id.get() as i64;
    let mut tx = pool.begin().await.map_err(|e| e.to_string())?;
    let quest_id: Option<i32> = sqlx::query_scalar("SELECT quest_id FROM player_quests WHERE player_quest_id = $1 AND user_id = $2 AND status = 'Accepted'").bind(player_quest_id).bind(user_id_i64).fetch_optional(&mut *tx).await.map_err(|e| e.to_string())?;

    if let Some(id) = quest_id {
        let rewards = sqlx::query_as!(
            QuestReward,
            "SELECT * FROM quest_rewards WHERE quest_id = $1",
            id
        )
        .fetch_all(&mut *tx)
        .await
        .map_err(|e| e.to_string())?;

        for reward in rewards {
            let _r_dbg = format!(
                "QuestRewardApply[id:{} q:{} coins:{:?} item:{:?} qty:{:?}]",
                reward.quest_reward_id,
                reward.quest_id,
                reward.reward_coins,
                reward.reward_item_id,
                reward.reward_item_quantity
            );
            // (✓) FIXED: Collapsed nested `if` statements as recommended by clippy.
            if let Some(coins) = reward.reward_coins
                && coins > 0
            {
                database::economy::add_balance(&mut tx, user_id, coins)
                    .await
                    .map_err(|e| e.to_string())?;
            }
            // (✓) FIXED: Collapsed nested `if let` and ignored unused `_item_id`.
            if let (Some(_item_id), Some(quantity), Some(item)) = (
                reward.reward_item_id,
                reward.reward_item_quantity,
                Item::from_i32(reward.reward_item_id.unwrap_or(0)),
            ) {
                database::economy::add_to_inventory(&mut tx, user_id, item, quantity as i64)
                    .await
                    .map_err(|e| e.to_string())?;
            }
        }

        sqlx::query!("UPDATE player_quests SET status = 'Completed', completed_at = NOW() WHERE player_quest_id = $1", player_quest_id)
            .execute(&mut *tx).await.map_err(|e| e.to_string())?;
        tx.commit().await.map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("This quest cannot be completed. It may not be active or belong to you.".to_string())
    }
}

/// Retrieves the title of a quest given a player_quest_id.
pub async fn get_quest_title(pool: &PgPool, player_quest_id: i32) -> Result<String, sqlx::Error> {
    sqlx::query_scalar!("SELECT q.title FROM quests q JOIN player_quests pq ON q.quest_id = pq.quest_id WHERE pq.player_quest_id = $1", player_quest_id).fetch_one(pool).await
}

pub async fn get_player_quest(
    pool: &PgPool,
    player_quest_id: i32,
) -> Result<Option<PlayerQuest>, sqlx::Error> {
    sqlx::query_as!(
        PlayerQuest,
        r#"
        SELECT
            player_quest_id, user_id, quest_id, status as "status: _",
            offered_at, accepted_at, completed_at
        FROM player_quests WHERE player_quest_id = $1
        "#,
        player_quest_id
    )
    .fetch_optional(pool)
    .await
}
