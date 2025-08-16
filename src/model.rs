// In src/model.rs
use std::collections::HashMap;
use std::sync::Arc;

use serenity::gateway::ShardManager;
use serenity::model::id::MessageId;
use serenity::prelude::TypeMapKey;
use sqlx::PgPool;
use tokio::sync::RwLock; // For our new database pool

use crate::commands::rps::state::GameState;

pub struct ShardManagerContainer;
impl TypeMapKey for ShuttleSerenity {
    type Value = Arc<ShardManager>;
}

// AppState now includes a connection pool to our Shuttle database.
pub struct AppState {
    pub active_games: Arc<RwLock<HashMap<MessageId, GameState>>>,
    pub db_pool: PgPool,
}

impl TypeMapKey for AppState {
    type Value = Arc<AppState>;
}
