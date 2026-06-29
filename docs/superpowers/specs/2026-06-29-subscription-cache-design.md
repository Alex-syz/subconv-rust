# Subscription Cache with Request Coalescing

## Problem

The upstream provider `dash.xn--cp3a08l.com` rate-limits requests, returning HTTP 403 when hit too frequently. The `/sub` and `/provider` endpoints have no caching — every client request goes directly to upstream. When OpenClash auto-updates (~20:00 daily), 5-7 requests arrive within 10 seconds, triggering the rate limit.

Additionally, OpenClash has two subscriptions pointing to the same upstream:
- "源" (direct, bypasses subconv)
- "内网3" (through subconv)

Both update at the same time, doubling the upstream load.

## Goal

Reduce upstream requests from ~7 per update burst to 1, using an in-memory cache with request coalescing on the `/sub` and `/provider` endpoints.

## Design

### Architecture

Add a new `SubCache` struct alongside the existing `LayeredCache`. The two systems are independent — they serve different endpoints and have different configurations.

```
AppState
├── cache: Arc<LayeredCache>       ← /proxy endpoint (existing)
└── sub_cache: Arc<SubCache>       ← /sub, /provider endpoints (new)
```

### SubCache Structure

```rust
pub struct SubCache {
    /// Cached subscription responses keyed by full query string.
    entries: RwLock<HashMap<String, CacheEntry>>,
    /// Per-key mutexes for request coalescing.
    inflight: RwLock<HashMap<String, Arc<Mutex<()>>>>,
    /// Time-to-live for cached entries (default: 300s).
    ttl: Duration,
    /// Maximum time to wait for an in-flight request (default: 3s).
    lock_timeout: Duration,
}

struct CacheEntry {
    body: Bytes,
    subscription_userinfo: Option<String>,
    content_disposition: Option<String>,
    fetched_at: Instant,
}
```

- `entries` stores successful upstream responses with metadata headers
- `inflight` stores per-key mutexes so concurrent requests for the same URL wait instead of all hitting upstream
- `CacheEntry` mirrors the subscription-relevant headers from upstream

### Cache Key

The full query string of the request serves as the cache key. This ensures different parameter combinations (url, template, interval, short, npr) get separate cache entries.

- `/sub?url=X&template=meta-rules` → key = `url=X&template=meta-rules`
- `/provider?url=X` → key = `url=X`

### Request Flow

```
1. Construct cache key from query parameters
2. Check entries cache
   → HIT and fresh: return cached response immediately
3. Get or create per-key mutex from inflight map
4. Acquire mutex (with lock_timeout)
   → timeout: proceed without lock (fetch upstream directly)
5. Double-check entries cache (another request may have populated it while we waited)
   → HIT and fresh: return cached response
6. Fetch upstream
   → success: write to entries cache, return response
   → failure (403 etc): return error, do NOT cache
7. Mutex auto-releases on drop; waiting requests wake up and hit step 5
```

### Configuration

Two new fields in `AppConfig`:

```yaml
# config.yaml
SUB_CACHE_TTL: 300          # seconds, default 300
SUB_CACHE_LOCK_TIMEOUT: 3   # seconds, default 3
```

Environment variable overrides:
- `SUBCONV_SUB_CACHE_TTL` → `SUB_CACHE_TTL`
- `SUBCONV_SUB_CACHE_LOCK_TIMEOUT` → `SUB_CACHE_LOCK_TIMEOUT`

These fields are independent from the existing `CACHE_TTL` (used by `LayeredCache` for `/proxy`).

### Cleanup

Expired entries are evicted during the existing periodic cache cleanup task in `main.rs`. The `SubCache` exposes an `evict_expired()` method called alongside `LayeredCache::evict_expired()`.

The `inflight` map does not need explicit cleanup — entries are short-lived (mutex drops after fetch completes) and the map is small.

## File Changes

| File | Change |
|------|--------|
| `src/cache/mod.rs` | Add `pub mod subscription;` |
| `src/cache/subscription.rs` | **New** — `SubCache`, `CacheEntry`, core logic |
| `src/app.rs` | Add `sub_cache: Arc<SubCache>` to `AppState` |
| `src/main.rs` | Initialize `SubCache`, add eviction to cleanup task |
| `src/handlers/sub.rs` | Cache lookup before fetch, cache write after success |
| `src/handlers/provider.rs` | Same as above |
| `src/config/app_config.rs` | Add `sub_cache_ttl`, `sub_cache_lock_timeout` fields + env overrides |

## Error Handling

- Upstream 403 / network errors: returned to client as-is, NOT cached
- Lock timeout (3s): proceed with direct upstream fetch, no error
- Cache write failure: logged as warning, response still returned to client
- Cache read failure: treat as cache miss, proceed to fetch

## Testing

Unit tests for `SubCache`:
- `put_and_get_fresh` — basic cache hit
- `get_returns_none_for_missing_key` — cache miss
- `get_returns_none_when_expired` — TTL expiry
- `evict_expired_removes_stale` — cleanup
- `concurrent_requests_share_fetch` — request coalescing (only one upstream fetch)
- `lock_timeout_allows_fallback` — timeout proceeds to fetch

Integration test:
- Verify `/sub` returns cached response on second request within TTL
- Verify upstream is called only once for concurrent requests
