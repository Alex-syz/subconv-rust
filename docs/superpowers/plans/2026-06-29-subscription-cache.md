# Subscription Cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add in-memory caching with request coalescing to `/sub` and `/provider` endpoints so that burst requests from OpenClash result in only one upstream fetch.

**Architecture:** A new `SubCache` struct uses `RwLock<HashMap>` for cached entries and per-key `Arc<Mutex<()>>` for request coalescing. Cache key is the endpoint prefix + full query string. TTL and lock timeout are configurable.

**Tech Stack:** Rust, tokio (Mutex), std::sync::RwLock, bytes::Bytes, std::time::Instant

## Global Constraints

- Only add new code; do not modify existing logic in any file
- Follow existing code style: `std::sync::RwLock` for synchronous data, `tokio::sync::Mutex` for async locks
- Existing tests must continue to pass
- No new crate dependencies required (tokio, bytes already in Cargo.toml)

---

### Task 1: Add Config Fields to AppConfig

**Files:**
- Modify: `src/config/app_config.rs`

**Interfaces:**
- Produces: `AppConfig.sub_cache_ttl: u64` (default 300), `AppConfig.sub_cache_lock_timeout: u64` (default 3)

- [ ] **Step 1: Add default functions**

In `src/config/app_config.rs`, add after `default_cache_max_size()` (line 36):

```rust
fn default_sub_cache_ttl() -> u64 {
    300
}
fn default_sub_cache_lock_timeout() -> u64 {
    3
}
```

- [ ] **Step 2: Add fields to AppConfig struct**

In the `AppConfig` struct, add after `cache_max_size_mb` (line 74):

```rust
    #[serde(rename = "SUB_CACHE_TTL", default = "default_sub_cache_ttl")]
    pub sub_cache_ttl: u64,

    #[serde(rename = "SUB_CACHE_LOCK_TIMEOUT", default = "default_sub_cache_lock_timeout")]
    pub sub_cache_lock_timeout: u64,
```

- [ ] **Step 3: Add env overrides**

In `apply_env_overrides()`, add after the `SUBCONV_CACHE_MAX_SIZE_MB` block (after line 189):

```rust
        if let Ok(v) = std::env::var("SUBCONV_SUB_CACHE_TTL") {
            config.sub_cache_ttl = v.parse().map_err(|_| {
                SubconvError::Config("SUBCONV_SUB_CACHE_TTL is not a valid u64".into())
            })?;
        }
        if let Ok(v) = std::env::var("SUBCONV_SUB_CACHE_LOCK_TIMEOUT") {
            config.sub_cache_lock_timeout = v.parse().map_err(|_| {
                SubconvError::Config("SUBCONV_SUB_CACHE_LOCK_TIMEOUT is not a valid u64".into())
            })?;
        }
```

- [ ] **Step 4: Verify config loads with defaults**

Run: `cargo test`
Expected: All existing tests pass (new fields use serde defaults)

- [ ] **Step 5: Commit**

```bash
git add src/config/app_config.rs
git commit -m "feat: add sub_cache_ttl and sub_cache_lock_timeout config fields"
```

---

### Task 2: Create SubCache Module (TDD)

**Files:**
- Modify: `src/cache/mod.rs`
- Create: `src/cache/subscription.rs`

**Interfaces:**
- Produces: `SubCache::new(ttl: Duration, lock_timeout: Duration) -> Self`
- Produces: `SubCache::get(&self, key: &str) -> Option<CacheEntry>`
- Produces: `SubCache::put(&self, key: String, entry: CacheEntry)`
- Produces: `SubCache::get_or_create_lock(&self, key: &str) -> Arc<Mutex<()>>`
- Produces: `SubCache::evict_expired(&self) -> usize`
- Produces: `SubCache::cleanup_inflight(&self)`

- [ ] **Step 1: Create module file with types (no logic)**

Create `src/cache/subscription.rs`:

```rust
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
}
```

- [ ] **Step 2: Add module declaration**

In `src/cache/mod.rs`, add after `pub mod memory;` (line 14):

```rust
pub mod subscription;
```

Add to the `pub use` line (line 16):

```rust
pub use subscription::SubCache;
```

- [ ] **Step 3: Write failing tests for get/put**

Append to `src/cache/subscription.rs`:

```rust
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

    #[tokio::test]
    async fn concurrent_requests_share_fetch() {
        let cache = Arc::new(SubCache::new(Duration::from_secs(300), Duration::from_secs(3)));
        let fetch_count = Arc::new(std::sync::atomic::AtomicU32::new(0));

        let mut handles = Vec::new();
        for _ in 0..5 {
            let c = Arc::clone(&cache);
            let fc = Arc::clone(&fetch_count);
            handles.push(tokio::spawn(async move {
                // Simulate the handler flow: check cache, lock, double-check, fetch
                let key = "sub:url=https://example.com";
                if c.get(key).is_some() {
                    return "cached";
                }
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
```

- [ ] **Step 4: Run tests to verify they fail**

Run: `cargo test -p subconv --lib cache::subscription`
Expected: Compilation errors — `get`, `put`, `get_or_create_lock`, `evict_expired`, `lock_timeout` methods not found

- [ ] **Step 5: Implement SubCache methods**

Replace the `impl SubCache` block in `src/cache/subscription.rs` with:

```rust
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
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test -p subconv --lib cache::subscription`
Expected: All 7 tests pass

Note: The `concurrent_requests_share_fetch` test uses `futures::future::join_all`. If `futures` is not in dependencies, add it as a dev-dependency, or use a manual join loop:

```rust
let mut results = Vec::new();
for h in handles {
    results.push(h.await.unwrap());
}
```

- [ ] **Step 7: Run full test suite**

Run: `cargo test`
Expected: All tests pass (new module is additive)

- [ ] **Step 8: Commit**

```bash
git add src/cache/mod.rs src/cache/subscription.rs
git commit -m "feat: add SubCache with per-key request coalescing"
```

---

### Task 3: Wire SubCache into Handlers

**Files:**
- Modify: `src/app.rs`
- Modify: `src/main.rs`
- Modify: `src/handlers/sub.rs`
- Modify: `src/handlers/provider.rs`

**Interfaces:**
- Consumes: `SubCache::new(ttl, lock_timeout)`, `SubCache::get()`, `SubCache::put()`, `SubCache::get_or_create_lock()`, `SubCache::lock_timeout()`
- Consumes: `AppConfig.sub_cache_ttl`, `AppConfig.sub_cache_lock_timeout`

- [ ] **Step 1: Add SubCache to AppState**

In `src/app.rs`, add import at the top:

```rust
use crate::cache::LayeredCache;
use crate::cache::SubCache;
```

Add field to `AppState`:

```rust
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub http_client: reqwest::Client,
    pub cache: Arc<LayeredCache>,
    pub sub_cache: Arc<SubCache>,
}
```

- [ ] **Step 2: Initialize SubCache in main.rs**

In `src/main.rs`, add import:

```rust
use subconv::cache::{LayeredCache, SubCache};
```

After the `LayeredCache` initialization (line 45), add:

```rust
    // 4b. Build subscription cache.
    let sub_cache = Arc::new(SubCache::new(
        Duration::from_secs(config.sub_cache_ttl),
        Duration::from_secs(config.sub_cache_lock_timeout),
    ));
```

Update the cleanup task to also evict subscription cache. Replace the cleanup block (lines 48-51) with:

```rust
    // 5. Start cache cleanup background task (every 10 minutes).
    let cleanup_cache = Arc::clone(&cache);
    let cleanup_sub_cache = Arc::clone(&sub_cache);
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(600));
        interval.tick().await; // skip first tick
        loop {
            interval.tick().await;
            tracing::debug!("running periodic cache cleanup");
            cleanup_cache.evict_expired().await;
            cleanup_sub_cache.cleanup_inflight();
            let evicted = cleanup_sub_cache.evict_expired();
            if evicted > 0 {
                tracing::info!(count = evicted, "evicted expired subscription cache entries");
            }
        }
    });
```

Update AppState construction:

```rust
    let state = AppState {
        config: Arc::new(config),
        http_client,
        cache,
        sub_cache,
    };
```

- [ ] **Step 3: Run tests to verify wiring compiles**

Run: `cargo build`
Expected: Compilation succeeds

- [ ] **Step 4: Add cache to /sub handler**

In `src/handlers/sub.rs`, add at the top of `sub_handler` (after line 45, before template resolution):

```rust
    // Check subscription cache.
    let cache_key = format!("sub:{}", request.uri().query().unwrap_or(""));
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        if let Some(info) = cached.subscription_userinfo {
            if let Ok(val) = axum::http::HeaderValue::from_str(&info) {
                resp.headers_mut().insert("subscription-userinfo", val);
            }
        }
        if let Some(disp) = cached.content_disposition {
            if let Ok(val) = axum::http::HeaderValue::from_str(&disp) {
                resp.headers_mut().insert("Content-Disposition", val);
            }
        }
        return Ok(resp);
    }

    // Request coalescing: wait for in-flight request or proceed.
    let lock = state.sub_cache.get_or_create_lock(&cache_key);
    let guard = tokio::time::timeout(state.sub_cache.lock_timeout(), lock.lock())
        .await;
    let _guard = guard.ok();

    // Double-check cache after acquiring lock.
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        if let Some(info) = cached.subscription_userinfo {
            if let Ok(val) = axum::http::HeaderValue::from_str(&info) {
                resp.headers_mut().insert("subscription-userinfo", val);
            }
        }
        if let Some(disp) = cached.content_disposition {
            if let Ok(val) = axum::http::HeaderValue::from_str(&disp) {
                resp.headers_mut().insert("Content-Disposition", val);
            }
        }
        return Ok(resp);
    }
    // If lock timed out, we proceed without the lock. This may result in
    // duplicate fetches but is not a correctness issue — the cache write
    // is protected by RwLock and the response is still valid.

This requires the handler to accept the raw request URI. Update the handler signature to also extract `uri: axum::http::Uri`:

```rust
pub async fn sub_handler(
    State(state): State<AppState>,
    Query(params): Query<SubParams>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Result<Response, SubconvError> {
```

- [ ] **Step 5: Write to cache after successful fetch in /sub**

After the response is built (after the `if let Some(disp)` block for content-disposition, before `Ok(resp)` — around line 167), add:

```rust
    // Cache the successful response.
    let cache_entry = crate::cache::subscription::CacheEntry {
        body: resp_body.clone(),
        subscription_userinfo: subscription_userinfo.clone(),
        content_disposition: content_disposition.clone(),
        fetched_at: std::time::Instant::now(),
    };
    state.sub_cache.put(cache_key, cache_entry);
```

This requires capturing `resp_body` before the response is built. Modify the response construction to capture the body:

Before `let mut resp = yaml.into_response();`, add:

```rust
    let resp_body = Bytes::from(yaml.clone());
```

And add `use bytes::Bytes;` at the top of the file.

- [ ] **Step 6: Add cache to /provider handler**

In `src/handlers/provider.rs`, add the same cache check pattern at the top of `provider_handler`:

```rust
    // Check subscription cache.
    let cache_key = format!("provider:{}", request.uri().query().unwrap_or(""));
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        return Ok(resp);
    }

    // Request coalescing.
    let lock = state.sub_cache.get_or_create_lock(&cache_key);
    let guard = tokio::time::timeout(state.sub_cache.lock_timeout(), lock.lock())
        .await;
    let _guard = guard.ok();

    // Double-check.
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        return Ok(resp);
    }
```

Update handler signature to include `uri`:

```rust
pub async fn provider_handler(
    State(state): State<AppState>,
    Query(params): Query<ProviderParams>,
    headers: axum::http::HeaderMap,
    uri: axum::http::Uri,
) -> Result<Response, SubconvError> {
```

After the response is built (before `Ok(response)`), add cache write:

```rust
    // Cache the successful response.
    let cache_entry = crate::cache::subscription::CacheEntry {
        body: Bytes::from(yaml.clone()),
        subscription_userinfo: None,
        content_disposition: None,
        fetched_at: std::time::Instant::now(),
    };
    state.sub_cache.put(cache_key, cache_entry);
```

Add `use bytes::Bytes;` at the top.

- [ ] **Step 7: Build and run all tests**

Run: `cargo test`
Expected: All tests pass

Run: `cargo build`
Expected: Compilation succeeds

- [ ] **Step 8: Commit**

```bash
git add src/app.rs src/main.rs src/handlers/sub.rs src/handlers/provider.rs
git commit -m "feat: wire subscription cache into /sub and /provider handlers"
```

---

### Task 4: Manual Verification

- [ ] **Step 1: Start the server**

Run: `cargo run`
Expected: `subconv starting`, `configuration loaded`, `listening on 0.0.0.0:8080`

- [ ] **Step 2: Test cache behavior**

Send the same request twice:

```bash
# First request (cache miss, fetches upstream)
curl -s -o /dev/null -w "status: %{http_code}, time: %{time_total}s\n" \
  "http://localhost:8080/sub?url=https://example.com/sub"

# Second request (cache hit, instant)
curl -s -o /dev/null -w "status: %{http_code}, time: %{time_total}s\n" \
  "http://localhost:8080/sub?url=https://example.com/sub"
```

Expected: Second request is significantly faster (< 10ms vs 1-2s)

- [ ] **Step 3: Verify logs**

Check server logs for:
- First request: upstream fetch log (no cache hit)
- Second request: no upstream fetch log (cache hit)
