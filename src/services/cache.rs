//! Generic TTL cache helper utilities.
//! These helpers wrap the common pattern of (Instant, Value) stored in a HashMap behind an `RwLock`.
//! They are intentionally minimal so existing cache maps in `AppState` can be incrementally migrated
//! without large structural changes.
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Attempt to fetch a cloned value from a `(Instant, V)` TTL cache.
/// Returns `None` if the key is absent or the entry is expired. Expired entries are eagerly removed.
static HIT_COUNTER: tokio::sync::OnceCell<tokio::sync::RwLock<u64>> =
    tokio::sync::OnceCell::const_new();
static MISS_COUNTER: tokio::sync::OnceCell<tokio::sync::RwLock<u64>> =
    tokio::sync::OnceCell::const_new();

async fn inc(hit: bool) {
    let cell = if hit { &HIT_COUNTER } else { &MISS_COUNTER };
    let lock = cell
        .get_or_init(|| async { tokio::sync::RwLock::new(0_u64) })
        .await;
    let mut w = lock.write().await;
    *w += 1;
}

/// Expose counters for diagnostics (hit, miss)
pub async fn cache_stats() -> (u64, u64) {
    let h = if let Some(lock) = HIT_COUNTER.get() {
        *lock.read().await
    } else {
        0
    };
    let m = if let Some(lock) = MISS_COUNTER.get() {
        *lock.read().await
    } else {
        0
    };
    (h, m)
}

pub async fn get_with_ttl<K, V>(
    map: &RwLock<HashMap<K, (Instant, V)>>,
    key: &K,
    ttl: Duration,
) -> Option<V>
where
    K: Eq + Hash + Clone,
    V: Clone,
{
    // Fast path: read lock
    if let Some((ts, val)) = map.read().await.get(key).cloned() {
        if ts.elapsed() < ttl {
            inc(true).await;
            return Some(val);
        }
    } else {
        inc(false).await;
        return None;
    }
    // Entry expired: acquire write lock to remove (avoid holding write unless needed)
    let mut write = map.write().await;
    if let Some((ts, _)) = write.get(key)
        && ts.elapsed() >= ttl {
            write.remove(key);
        }
    inc(false).await;
    None
}

/// Insert / overwrite a value in the TTL cache with current timestamp.
pub async fn insert<K, V>(map: &RwLock<HashMap<K, (Instant, V)>>, key: K, value: V)
where
    K: Eq + Hash,
{
    map.write().await.insert(key, (Instant::now(), value));
}
