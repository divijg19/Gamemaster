use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Simple generic TTL cache keyed by u64.
/// Optimized for small per-process caches; no eviction strategy beyond TTL expiration on access.
pub struct TtlCache<V> {
    ttl: Duration,
    map: HashMap<u64, (Instant, V)>,
}

impl<V> TtlCache<V> {
    pub fn new(ttl: Duration) -> Self {
        Self {
            ttl,
            map: HashMap::new(),
        }
    }
    pub fn insert(&mut self, key: u64, value: V) {
        self.map.insert(key, (Instant::now(), value));
    }
    pub fn get(&self, key: &u64) -> Option<&V> {
        self.map.get(key).and_then(|(ts, v)| {
            if ts.elapsed() < self.ttl {
                Some(v)
            } else {
                None
            }
        })
    }
    pub fn invalidate(&mut self, key: &u64) {
        self.map.remove(key);
    }
    pub fn clear(&mut self) {
        self.map.clear();
    }
}
