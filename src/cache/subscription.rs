//! Subscription cache with request coalescing.
//!
//! Caches successful subscription responses in memory. Concurrent requests
//! for the same URL are coalesced via per-key mutexes so only one upstream
//! fetch occurs.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use bytes::Bytes;
use tokio::sync::Mutex;

/// A cached subscription response.
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// The response body.
    pub body: Bytes,
    /// Value of the `subscription-userinfo` header, if present.
    pub subscription_userinfo: Option<String>,
    /// Value of the `content-disposition` header, if present.
    pub content_disposition: Option<String>,
    /// When this entry was fetched from upstream.
    pub fetched_at: Instant,
}

impl CacheEntry {
    /// Check if this entry is still fresh within the given TTL.
    pub fn is_fresh(&self, ttl: Duration) -> bool {
        self.fetched_at.elapsed() < ttl
    }
}

/// In-memory cache for subscription responses with per-key request coalescing.
pub struct SubCache {
    /// Cached responses keyed by endpoint-prefixed query string.
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// Per-key mutexes for request coalescing.
    inflight: RwLock<HashMap<String, Arc<Mutex<()>>>>,
    /// Time-to-live for cached entries.
    ttl: Duration,
    /// Maximum time to wait for an in-flight request before falling through.
    lock_timeout: Duration,
}

impl SubCache {
    /// Create a new subscription cache.
    pub fn new(ttl: Duration, lock_timeout: Duration) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            inflight: RwLock::new(HashMap::new()),
            ttl,
            lock_timeout,
        }
    }

    /// Get a cached entry if it exists and is still fresh.
    pub fn get(&self, key: &str) -> Option<CacheEntry> {
        let entries = self.entries.read().unwrap();
        entries.get(key).filter(|e| e.is_fresh(self.ttl)).cloned()
    }

    /// Store a successful response in the cache.
    pub fn put(&self, key: String, entry: CacheEntry) {
        let mut entries = self.entries.write().unwrap();
        entries.insert(key, entry);
    }

    /// Get or create a per-key mutex for request coalescing.
    ///
    /// If a mutex already exists for the key, returns the existing one.
    /// Otherwise creates a new one.
    pub fn get_or_create_lock(&self, key: &str) -> Arc<Mutex<()>> {
        // Try read lock first (fast path)
        {
            let inflight = self.inflight.read().unwrap();
            if let Some(lock) = inflight.get(key) {
                return Arc::clone(lock);
            }
        }
        // Write lock to insert new entry
        let mut inflight = self.inflight.write().unwrap();
        // Double-check after acquiring write lock
        inflight
            .entry(key.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }

    /// Get the lock timeout duration.
    pub fn lock_timeout(&self) -> Duration {
        self.lock_timeout
    }

    /// Evict all expired entries from the cache.
    ///
    /// Returns the number of entries removed.
    pub fn evict_expired(&self) -> usize {
        let mut entries = self.entries.write().unwrap();
        let before = entries.len();
        entries.retain(|_, e| e.is_fresh(self.ttl));
        before - entries.len()
    }

    /// Remove inflight entries whose lock is no longer held by any task.
    ///
    /// Called during periodic cleanup to prevent memory leaks from stale entries.
    pub fn cleanup_inflight(&self) {
        let mut inflight = self.inflight.write().unwrap();
        inflight.retain(|_, lock| Arc::strong_count(lock) > 1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_and_get_fresh() {
        let cache = SubCache::new(Duration::from_secs(300), Duration::from_secs(3));
        let entry = CacheEntry {
            body: Bytes::from_static(b"proxies:\n  - name: test"),
            subscription_userinfo: Some("upload=100; download=200".into()),
            content_disposition: None,
            fetched_at: Instant::now(),
        };

        cache.put("sub:url=https://example.com".into(), entry);
        let hit = cache.get("sub:url=https://example.com").unwrap();
        assert_eq!(&hit.body[..], b"proxies:\n  - name: test");
        assert_eq!(hit.subscription_userinfo.as_deref(), Some("upload=100; download=200"));
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let cache = SubCache::new(Duration::from_secs(300), Duration::from_secs(3));
        assert!(cache.get("nonexistent").is_none());
    }

    #[test]
    fn get_returns_none_when_expired() {
        let cache = SubCache::new(Duration::from_millis(10), Duration::from_secs(3));
        let entry = CacheEntry {
            body: Bytes::from_static(b"stale"),
            subscription_userinfo: None,
            content_disposition: None,
            fetched_at: Instant::now(),
        };
        cache.put("key1".into(), entry);

        // Fresh immediately
        assert!(cache.get("key1").is_some());

        // Expired after TTL
        std::thread::sleep(Duration::from_millis(20));
        assert!(cache.get("key1").is_none());
    }

    #[test]
    fn evict_expired_removes_stale() {
        let cache = SubCache::new(Duration::from_millis(10), Duration::from_secs(3));

        let old = CacheEntry {
            body: Bytes::from_static(b"old"),
            subscription_userinfo: None,
            content_disposition: None,
            fetched_at: Instant::now(),
        };
        cache.put("old".into(), old);

        std::thread::sleep(Duration::from_millis(20));

        let fresh = CacheEntry {
            body: Bytes::from_static(b"fresh"),
            subscription_userinfo: None,
            content_disposition: None,
            fetched_at: Instant::now(),
        };
        cache.put("fresh".into(), fresh);

        let evicted = cache.evict_expired();
        assert_eq!(evicted, 1);
        assert!(cache.get("old").is_none());
        assert!(cache.get("fresh").is_some());
    }

    #[test]
    fn get_or_create_lock_returns_same_arc() {
        let cache = SubCache::new(Duration::from_secs(300), Duration::from_secs(3));
        let lock1 = cache.get_or_create_lock("key1");
        let lock2 = cache.get_or_create_lock("key1");
        assert!(Arc::ptr_eq(&lock1, &lock2));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 5)]
    async fn concurrent_requests_share_fetch() {
        let cache = Arc::new(SubCache::new(Duration::from_secs(300), Duration::from_secs(3)));
        let fetch_count = Arc::new(std::sync::atomic::AtomicU32::new(0));
        // Barrier to ensure all tasks check cache before any proceeds to the lock
        let cache_check = Arc::new(tokio::sync::Barrier::new(5));

        let mut handles = Vec::new();
        for _ in 0..5 {
            let c = Arc::clone(&cache);
            let fc = Arc::clone(&fetch_count);
            let cb = Arc::clone(&cache_check);
            handles.push(tokio::spawn(async move {
                // Simulate the handler flow: check cache, lock, double-check, fetch
                let key = "sub:url=https://example.com";
                if c.get(key).is_some() {
                    return "cached";
                }
                // Ensure all tasks have checked cache before any proceeds to lock
                cb.wait().await;
                let lock = c.get_or_create_lock(key);
                let guard = tokio::time::timeout(c.lock_timeout(), lock.lock()).await;
                let _guard = guard.ok();
                if c.get(key).is_some() {
                    return "double_check_hit";
                }
                // Simulate fetch
                fc.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let entry = CacheEntry {
                    body: Bytes::from_static(b"fetched"),
                    subscription_userinfo: None,
                    content_disposition: None,
                    fetched_at: Instant::now(),
                };
                c.put(key.into(), entry);
                "fetched"
            }));
        }

        let mut statuses = Vec::new();
        for h in handles {
            statuses.push(h.await.unwrap());
        }

        // Only one request should have fetched
        assert_eq!(fetch_count.load(std::sync::atomic::Ordering::SeqCst), 1);
        // Others should have hit cache via double-check
        let fetched_count = statuses.iter().filter(|&&s| s == "fetched").count();
        let cached_count = statuses.iter().filter(|&&s| s == "double_check_hit").count();
        assert_eq!(fetched_count, 1);
        assert_eq!(cached_count, 4);
    }

    #[test]
    fn cleanup_inflight_removes_unused_locks() {
        let cache = SubCache::new(Duration::from_secs(300), Duration::from_secs(3));

        // Create and hold a lock (strong_count = 2: map + held)
        let lock1 = cache.get_or_create_lock("key1");
        assert_eq!(Arc::strong_count(&lock1), 2);

        // Cleanup should keep it (tasks still hold references)
        cache.cleanup_inflight();
        let lock1b = cache.get_or_create_lock("key1");
        assert!(Arc::ptr_eq(&lock1, &lock1b));

        // Drop all task references (strong_count drops to 1: only map)
        drop(lock1);
        drop(lock1b);

        // Cleanup should remove it now
        cache.cleanup_inflight();

        // New lock is a different Arc (old entry was removed)
        let lock2 = cache.get_or_create_lock("key1");
        assert_eq!(Arc::strong_count(&lock2), 2);
    }

    #[tokio::test]
    async fn lock_timeout_allows_fallback() {
        let cache = Arc::new(SubCache::new(Duration::from_secs(300), Duration::from_millis(50)));

        // Hold the lock for a long time
        let lock = cache.get_or_create_lock("key1");
        let _guard = lock.lock().await;

        // Another request with short timeout should proceed
        let c = Arc::clone(&cache);
        let result = tokio::spawn(async move {
            let lock2 = c.get_or_create_lock("key1");
            let acquired = tokio::time::timeout(c.lock_timeout(), lock2.lock()).await;
            if acquired.is_err() {
                "timeout_proceeded"
            } else {
                "acquired"
            }
        })
        .await
        .unwrap();

        assert_eq!(result, "timeout_proceeded");
    }
}
