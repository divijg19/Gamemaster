//! This module defines the shared data structures used throughout the application.
//! These structs are used as `TypeMapKey`s to store shared state in Serenity's global context.

// (✓) CORRECTED: Use the full, correct path to the generic GameManager.
use crate::commands::games::engine::GameManager;
// (✓) CORRECTED: Use the concrete PgPool type directly.
use crate::database::models::UnitRarity;
use serenity::gateway::ShardManager;
use serenity::prelude::TypeMapKey;
use sqlx::PgPool;
use std::sync::Arc;
use std::{collections::HashMap, time::Instant};
use tokio::sync::RwLock;

// Type aliases to reduce clippy::type_complexity noise and clarify intent.
type BondedEquippablesMap = HashMap<i32, Vec<(i32, String, UnitRarity)>>; // host_player_unit_id -> equipped list
type UserBondCacheEntry = (Instant, BondedEquippablesMap);
type UserBondCache = HashMap<u64, UserBondCacheEntry>;
type EquipmentBonusMap = HashMap<i32, (i32, i32, i32)>; // player_unit_id -> (atk,def,hp)
type UserBonusCacheEntry = (Instant, EquipmentBonusMap);
type UserBonusCache = HashMap<u64, UserBonusCacheEntry>;
// Contract status caching (human recruitment progress) -----------------------
// Vec cached: (Unit, defeats, required, drafted, recruited, last_defeat_ts)
type ContractStatus = Vec<(
    crate::database::models::Unit,
    i32,
    i32,
    bool,
    bool,
    Option<chrono::DateTime<chrono::Utc>>,
)>;
type UserContractCacheEntry = (Instant, ContractStatus);
type UserContractCache = HashMap<u64, UserContractCacheEntry>;
// Research progress caching (unit_id -> count) per user
type ResearchProgress = Vec<(i32, i32)>; // (unit_id, count)
type UserResearchCacheEntry = (Instant, ResearchProgress);
type UserResearchCache = HashMap<u64, UserResearchCacheEntry>;

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
    /// Cached bond mappings per user: host_player_unit_id -> Vec<(equipped_id, name, rarity)>
    pub bond_cache: Arc<RwLock<UserBondCache>>,
    /// Cached equipment bonuses per user (player_unit_id -> (atk,def,hp)) with TTL.
    pub bonus_cache: Arc<RwLock<UserBonusCache>>,
    /// Configurable starter unit id for tutorial; defaults to 1.
    pub starter_unit_id: Arc<RwLock<i32>>, // runtime configurable via /config
    /// Cached human contract / defeat progress per user with TTL.
    pub contract_cache: Arc<RwLock<UserContractCache>>,
    /// Cached pet research progress per user with TTL.
    pub research_cache: Arc<RwLock<UserResearchCache>>,
}

impl AppState {
    pub async fn from_ctx(ctx: &serenity::prelude::Context) -> Option<Arc<Self>> {
        ctx.data.read().await.get::<AppState>().cloned()
    }

    /// Invalidate user-specific bond & bonus caches after bonding state changes.
    pub async fn invalidate_user_caches(&self, user_id: serenity::model::id::UserId) {
        {
            let mut bonus = self.bonus_cache.write().await;
            bonus.remove(&user_id.get());
        }
        {
            let mut bonds = self.bond_cache.write().await;
            bonds.remove(&user_id.get());
        }
        {
            let mut contracts = self.contract_cache.write().await;
            contracts.remove(&user_id.get());
        }
        {
            let mut research = self.research_cache.write().await;
            research.remove(&user_id.get());
        }
    }
}

impl TypeMapKey for AppState {
    type Value = Arc<AppState>;
}
