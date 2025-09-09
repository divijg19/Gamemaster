//! Contains all database functions related to the game world.
//! This includes fetching data about map nodes, enemies, and rewards.

use super::models::{MapNode, NodeReward, Unit, UnitRarity};
use sqlx::PgPool;

/// Fetches the master data for a list of map nodes by their IDs.
pub async fn get_map_nodes_by_ids(
    pool: &PgPool,
    node_ids: &[i32],
) -> Result<Vec<MapNode>, sqlx::Error> {
    sqlx::query_as::<_, MapNode>(
        "SELECT node_id, area_id, name, description, story_progress_required, reward_coins, reward_unit_xp FROM map_nodes WHERE node_id = ANY($1)",
    )
    .bind(node_ids)
    .fetch_all(pool)
    .await
}

/// Fetch all map nodes (ordered by required story progress then node id) for richer map UI/UX.
pub async fn get_all_map_nodes(pool: &PgPool) -> Result<Vec<MapNode>, sqlx::Error> {
    sqlx::query_as::<_, MapNode>(
        "SELECT * FROM map_nodes ORDER BY story_progress_required, node_id",
    )
    .fetch_all(pool)
    .await
}

/// Fetches the potential loot rewards for a specific battle node.
pub async fn get_rewards_for_node(
    pool: &PgPool,
    node_id: i32,
) -> Result<Vec<NodeReward>, sqlx::Error> {
    sqlx::query_as::<_, NodeReward>(
        "SELECT item_id, quantity, drop_chance FROM node_rewards WHERE node_id = $1",
    )
    .bind(node_id)
    .fetch_all(pool)
    .await
}

/// Consolidated fetch for a single node: returns (node, enemies, rewards) in one shot.
/// Performs three queries in a single connection acquisition instead of scattered calls.
pub async fn get_full_node_bundle(
    pool: &PgPool,
    node_id: i32,
) -> Result<(MapNode, Vec<Unit>, Vec<NodeReward>), sqlx::Error> {
    let mut conn = pool.acquire().await?;
    let node_opt = sqlx::query_as::<_, MapNode>(
        "SELECT node_id, area_id, name, description, story_progress_required, reward_coins, reward_unit_xp FROM map_nodes WHERE node_id = $1",
    )
    .bind(node_id)
    .fetch_optional(&mut *conn)
    .await?;
    let node = match node_opt {
        Some(n) => n,
        None => return Err(sqlx::Error::RowNotFound),
    };
    let mut enemies = sqlx::query_as::<_, Unit>(
        "SELECT u.unit_id, u.name, u.description, u.base_attack, u.base_defense, u.base_health, u.is_recruitable, u.kind as kind, u.rarity as rarity FROM units u JOIN node_enemies ne ON u.unit_id = ne.unit_id WHERE ne.node_id = $1",
    )
    .bind(node_id)
    .fetch_all(&mut *conn)
    .await?;
    if enemies.is_empty() {
        // Fallback: generate a small enemy group based on story requirement; prefer non-recruitable creatures
        let target_size = 3_i64;
        let min_rarity: UnitRarity = if node.story_progress_required <= 2 {
            UnitRarity::Common
        } else if node.story_progress_required <= 5 {
            UnitRarity::Rare
        } else if node.story_progress_required <= 8 {
            UnitRarity::Epic
        } else {
            UnitRarity::Legendary
        };
        // Pick candidates: pets preferred, exclude humans for random generation
        let candidates = sqlx::query_as::<_, Unit>(
            "SELECT unit_id, name, description, base_attack, base_defense, base_health, is_recruitable, kind as kind, rarity as rarity FROM units WHERE kind != 'Human'::unit_kind AND rarity >= $1 ORDER BY random() LIMIT 6",
        )
        .bind(min_rarity)
        .fetch_all(&mut *conn)
        .await?;
        let mut picked: Vec<i32> = Vec::new();
        for u in candidates.into_iter().take(target_size as usize) {
            picked.push(u.unit_id);
            enemies.push(u);
        }
        // Persist generated mapping for this node for consistency
        for uid in picked {
            let _ = sqlx::query(
                "INSERT INTO node_enemies (node_id, unit_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(node_id)
            .bind(uid)
            .execute(&mut *conn)
            .await;
        }
    }
    let rewards = sqlx::query_as::<_, NodeReward>(
        "SELECT item_id, quantity, drop_chance FROM node_rewards WHERE node_id = $1",
    )
    .bind(node_id)
    .fetch_all(&mut *conn)
    .await?;
    Ok((node, enemies, rewards))
}
