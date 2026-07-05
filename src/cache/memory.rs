//! L1 in-memory cache with TTL support.
//!
//! Uses `std::sync::RwLock` since all operations are pure in-memory
//! and never cross `.await` points. This avoids the overhead and
//! footguns of `tokio::sync::RwLock` for synchronous data access.

use std::collections::HashMap;
use std::sync::RwLock;
use std::time::{Duration, Instant};

use bytes::Bytes;

/// A cached entry with metadata for conditional requests.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The cached response body.
    pub body: Bytes,
    /// Content-Type header value.
    pub content_type: String,
    /// ETag for conditional requests (If-None-Match).
    pub etag: Option<String>,
    /// Last-Modified for conditional requests (If-Modified-Since).
    pub last_modified: Option<String>,
    /// When this entry was fetched from upstream.
    pub fetched_at: Instant,
}

impl CacheEntry {
    /// Check if this entry is still fresh within the given TTL.
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.fetched_at.elapsed() < ttl
    }
}

/// L1 in-memory cache with TTL and entry limit.
pub struct MemoryCache {
    /// The underlying store protected by a RwLock.
    store: RwLock<HashMap<String, CacheEntry>>,
    /// Default TTL for freshness checks.
    ttl: Duration,
    /// Maximum number of entries before eviction.
    max_entries: usize,
}

impl MemoryCache {
    /// Create a new memory cache.
    ///
    /// # Arguments
    /// * `ttl` - Time-to-live for cached entries.
    /// * `max_entries` - Maximum entries before LRU-style eviction.
    pub fn new(ttl: Duration, max_entries: usize) -> Self {
        Self {
            store: RwLock::new(HashMap::new()),
            ttl,
            max_entries,
        }
    }

    /// Get a cached entry regardless of freshness.
    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        let store = self.store.read().unwrap();
        store.get(key).cloned()
    }

    /// Get a cached entry only if it's still fresh within TTL.
    pub fn get_if_fresh(&self, key: &str) -> Option<CacheEntry> {
        let store = self.store.read().unwrap();
        store.get(key).filter(|e| e.is_fresh(self.ttl)).cloned()
    }

    /// Insert a new entry into the cache.
    ///
    /// If the cache is at capacity, evicts expired entries first,
    /// then evicts the oldest entry if still over capacity.
    pub fn insert(&self, key: String, entry: CacheEntry) {
        let mut store = self.store.write().unwrap();

        if store.len() >= self.max_entries {
            // Remove expired entries first
            store.retain(|_, e| e.is_fresh(self.ttl));

            // If still at capacity, remove the oldest entry
            if store.len() >= self.max_entries {
                let oldest = store
                    .iter()
                    .min_by_key(|(_, e)| e.fetched_at)
                    .map(|(k, _)| k.clone());

                if let Some(k) = oldest {
                    store.remove(&k);
                }
            }
        }

        store.insert(key, entry);
    }

    /// Remove an entry from the cache.
    pub fn remove(&self, key: &str) {
        let mut store = self.store.write().unwrap();
        store.remove(key);
    }

    /// Get the current number of entries.
    pub fn len(&self) -> usize {
        let store = self.store.read().unwrap();
        store.len()
    }

    /// Check if the cache is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Evict all expired entries.
    ///
    /// Returns the number of entries removed.
    pub fn evict_expired(&self) -> usize {
        let mut store = self.store.write().unwrap();
        let before = store.len();
        store.retain(|_, e| e.is_fresh(self.ttl));
        before - store.len()
    }

    /// Update the fetched_at timestamp for an existing entry.
    ///
    /// Used when a 304 Not Modified response refreshes the TTL.
    pub fn refresh(&self, key: &str) -> bool {
        let mut store = self.store.write().unwrap();
        if let Some(entry) = store.get_mut(key) {
            entry.fetched_at = Instant::now();
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_get() {
        let cache = MemoryCache::new(Duration::from_secs(60), 100);
        let entry = CacheEntry {
            body: Bytes::from_static(b"test"),
            content_type: "text/plain".into(),
            etag: Some("\"abc\"".into()),
            last_modified: None,
            fetched_at: Instant::now(),
        };

        cache.insert("key1".into(), entry.clone());
        let retrieved = cache.get("key1");
        assert!(retrieved.is_some());
        assert_eq!(&retrieved.unwrap().body[..], &b"test"[..]);
    }

    #[test]
    fn get_if_fresh_respects_ttl() {
        let cache = MemoryCache::new(Duration::from_millis(10), 100);
        let entry = CacheEntry {
            body: Bytes::from_static(b"test"),
            content_type: "text/plain".into(),
            etag: None,
            last_modified: None,
            fetched_at: Instant::now(),
        };

        cache.insert("key1".into(), entry);
        assert!(cache.get_if_fresh("key1").is_some());

        // Wait for TTL to expire
        std::thread::sleep(Duration::from_millis(20));
        assert!(cache.get_if_fresh("key1").is_none());
        // But get() should still return the stale entry
        assert!(cache.get("key1").is_some());
    }

    #[test]
    fn evict_expired() {
        let cache = MemoryCache::new(Duration::from_millis(10), 100);

        // Insert entry that will expire
        let entry = CacheEntry {
            body: Bytes::from_static(b"old"),
            content_type: "text/plain".into(),
            etag: None,
            last_modified: None,
            fetched_at: Instant::now(),
        };
        cache.insert("old".into(), entry);

        std::thread::sleep(Duration::from_millis(20));

        // Insert fresh entry
        let fresh = CacheEntry {
            body: Bytes::from_static(b"fresh"),
            content_type: "text/plain".into(),
            etag: None,
            last_modified: None,
            fetched_at: Instant::now(),
        };
        cache.insert("fresh".into(), fresh);

        let evicted = cache.evict_expired();
        assert_eq!(evicted, 1);
        assert!(cache.get("old").is_none());
        assert!(cache.get("fresh").is_some());
    }

    #[test]
    fn max_entries_eviction() {
        let cache = MemoryCache::new(Duration::from_secs(60), 2);

        let e1 = CacheEntry {
            body: Bytes::from_static(b"1"),
            content_type: "text/plain".into(),
            etag: None,
            last_modified: None,
            fetched_at: Instant::now(),
        };
        cache.insert("1".into(), e1);

        // Small delay to ensure different timestamps
        std::thread::sleep(Duration::from_millis(1));

        let e2 = CacheEntry {
            body: Bytes::from_static(b"2"),
            content_type: "text/plain".into(),
            etag: None,
            last_modified: None,
            fetched_at: Instant::now(),
        };
        cache.insert("2".into(), e2);

        std::thread::sleep(Duration::from_millis(1));

        let e3 = CacheEntry {
            body: Bytes::from_static(b"3"),
            content_type: "text/plain".into(),
            etag: None,
            last_modified: None,
            fetched_at: Instant::now(),
        };
        cache.insert("3".into(), e3);

        // Entry "1" should have been evicted (oldest)
        assert!(cache.get("1").is_none());
        assert!(cache.get("2").is_some());
        assert!(cache.get("3").is_some());
    }
}
