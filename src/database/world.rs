//! Contains all database functions related to the game world.
//! This includes fetching data about map nodes, enemies, and rewards.

use super::models::{MapNode, NodeReward, Pet};
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

/// Fetches the enemy pet master data for a specific battle node.
pub async fn get_enemies_for_node(pool: &PgPool, node_id: i32) -> Result<Vec<Pet>, sqlx::Error> {
    sqlx::query_as!(
        Pet,
        "SELECT p.* FROM pets p JOIN node_enemies ne ON p.pet_id = ne.pet_id WHERE ne.node_id = $1",
        node_id
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
