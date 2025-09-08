//! Contains all database functions related to the game world.
//! This includes fetching data about map nodes, enemies, and rewards.

use super::models::{MapNode, NodeReward, Unit, UnitKind, UnitRarity};
use sqlx::PgPool;

/// Fetches the master data for a list of map nodes by their IDs.
pub async fn get_map_nodes_by_ids(
    pool: &PgPool,
    node_ids: &[i32],
) -> Result<Vec<MapNode>, sqlx::Error> {
    sqlx::query_as!(
        MapNode,
        "SELECT * FROM map_nodes WHERE node_id = ANY($1)",
        node_ids
    )
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
    sqlx::query_as!(
        NodeReward,
        "SELECT item_id, quantity, drop_chance FROM node_rewards WHERE node_id = $1",
        node_id
    )
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
    let node_opt = sqlx::query_as!(
        MapNode,
        "SELECT * FROM map_nodes WHERE node_id = $1",
        node_id
    )
    .fetch_optional(&mut *conn)
    .await?;
    let node = match node_opt {
        Some(n) => n,
        None => return Err(sqlx::Error::RowNotFound),
    };
    let enemies = sqlx::query_as!(
        Unit,
    "SELECT u.unit_id, u.name, u.description, u.base_attack, u.base_defense, u.base_health, u.is_recruitable, u.kind as \"kind: UnitKind\", u.rarity as \"rarity: UnitRarity\" FROM units u JOIN node_enemies ne ON u.unit_id = ne.unit_id WHERE ne.node_id = $1",
        node_id
    )
    .fetch_all(&mut *conn)
    .await?;
    let rewards = sqlx::query_as!(
        NodeReward,
        "SELECT item_id, quantity, drop_chance FROM node_rewards WHERE node_id = $1",
        node_id
    )
    .fetch_all(&mut *conn)
    .await?;
    Ok((node, enemies, rewards))
}
