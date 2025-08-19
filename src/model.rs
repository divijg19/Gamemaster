//! This module defines the shared data structures used throughout the application.
//! These structs are used as `TypeMapKey`s to store shared state in Serenity's global context.

use crate::commands::rps::state::GameState;
use crate::database::init::DbPool;
use serenity::gateway::ShardManager;
use serenity::model::id::MessageId;
use serenity::prelude::TypeMapKey;
use std::collections::HashMap;
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
    /// A map of currently active Rock, Paper, Scissors games, keyed by message ID.
    pub active_games: Arc<RwLock<HashMap<MessageId, GameState>>>,
    /// The connection pool for the PostgreSQL database.
    pub db: DbPool,
    /// The current command prefix, which can be changed at runtime by administrators.
    pub prefix: Arc<RwLock<String>>,
}

impl TypeMapKey for AppState {
    type Value = Arc<AppState>;
}
