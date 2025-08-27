//! This module defines the shared data structures used throughout the application.
//! These structs are used as `TypeMapKey`s to store shared state in Serenity's global context.

// (✓) CORRECTED: Use the full, correct path to the generic GameManager.
use crate::commands::games::engine::GameManager;
// (✓) CORRECTED: Use the concrete PgPool type directly.
use serenity::gateway::ShardManager;
use serenity::prelude::TypeMapKey;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;

/// A container for the ShardManager, allowing it to be stored in the global context.
/// This provides access to shard-specific information, like gateway latency.
pub struct ShardManagerContainer;

impl TypeMapKey for ShardManagerContainer {
    type Value = Arc<ShardManager>;
}

/// The central, shared state of the application.
/// An `Arc<AppState>` is stored in the global context for easy and safe access
/// from any command or event handler.
pub struct AppState {
    /// The manager for all active game instances, such as Blackjack or Poker.
    /// This is the single point of entry for all game-related logic.
    pub game_manager: Arc<RwLock<GameManager>>,
    /// The connection pool for the PostgreSQL database.
    pub db: PgPool,
    /// The current command prefix, which can be changed at runtime by administrators.
    pub prefix: Arc<RwLock<String>>,
}

impl TypeMapKey for AppState {
    type Value = Arc<AppState>;
}
