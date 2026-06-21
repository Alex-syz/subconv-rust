//! Layered cache combining L1 (memory) and L2 (disk).
//!
//! Lookup order: memory -> disk -> upstream.
//! On a disk hit, the memory cache is backfilled for subsequent requests.
//! When a 304 Not Modified is received, TTL is refreshed in both layers.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bytes::Bytes;
use crate::config::AppConfig;
use crate::SubconvError;

use super::disk::{DiskCache, DiskCacheEntry};
use super::memory::{CacheEntry, MemoryCache};

/// Unified content returned by the layered cache.
#[derive(Debug, Clone)]
pub struct CachedContent {
    /// The cached response body.
    pub body: Bytes,
    /// MIME type of the content.
    pub content_type: String,
    /// ETag for conditional requests.
    pub etag: Option<String>,
    /// Last-Modified header value.
    pub last_modified: Option<String>,
}

/// Validation headers for conditional requests.
#[derive(Debug, Clone)]
pub struct ValidationHeaders {
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// Two-level cache: L1 in-memory, L2 on-disk.
pub struct LayeredCache {
    memory: MemoryCache,
    disk: DiskCache,
    default_ttl: Duration,
}

/// Maximum number of entries in the L1 memory cache.
const MEMORY_MAX_ENTRIES: usize = 512;

impl LayeredCache {
    /// Create a new layered cache from application configuration.
    pub fn new(config: &AppConfig) -> Self {
        let default_ttl = Duration::from_secs(config.cache_ttl);
        let memory = MemoryCache::new(default_ttl, MEMORY_MAX_ENTRIES);
        let disk = DiskCache::new(&config.cache_dir, config.cache_max_size_mb);

        Self {
            memory,
            disk,
            default_ttl,
        }
    }

    /// Look up a key in the layered cache.
    ///
    /// 1. Check L1 memory cache (TTL-fresh only).
    /// 2. Check L2 disk cache (TTL-fresh only); backfill L1 on hit.
    ///
    /// Returns `None` if the key is not found in either layer or is stale.
    pub async fn get(&self, key: &str) -> Option<CachedContent> {
        // L1: Memory cache (fast path)
        if let Some(entry) = self.memory.get_if_fresh(key) {
            return Some(CachedContent {
                body: entry.body,
                content_type: entry.content_type,
                etag: entry.etag,
                last_modified: entry.last_modified,
            });
        }

        // L2: Disk cache
        if let Some(hit) = self.disk.get(key).await {
            if hit.meta.is_fresh() {
                // Backfill L1
                let mem_entry = CacheEntry {
                    body: hit.body.clone(),
                    content_type: hit.meta.content_type.clone(),
                    etag: hit.meta.etag.clone(),
                    last_modified: hit.meta.last_modified.clone(),
                    fetched_at: std::time::Instant::now(),
                };
                self.memory.insert(key.to_string(), mem_entry);

                return Some(CachedContent {
                    body: hit.body,
                    content_type: hit.meta.content_type,
                    etag: hit.meta.etag,
                    last_modified: hit.meta.last_modified,
                });
            }
        }

        None
    }

    /// Store content in both cache layers.
    pub async fn put(&self, key: &str, content: CachedContent) -> Result<(), SubconvError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        // L1: Memory cache
        let mem_entry = CacheEntry {
            body: content.body.clone(),
            content_type: content.content_type.clone(),
            etag: content.etag.clone(),
            last_modified: content.last_modified.clone(),
            fetched_at: std::time::Instant::now(),
        };
        self.memory.insert(key.to_string(), mem_entry);

        // L2: Disk cache
        let disk_entry = DiskCacheEntry {
            content_type: content.content_type.clone(),
            etag: content.etag.clone(),
            last_modified: content.last_modified.clone(),
            fetched_at: now,
            ttl: self.default_ttl.as_secs(),
        };
        // Disk write failure is non-fatal for the service; log and continue.
        if let Err(e) = self.disk.put(key, &content.body, &disk_entry).await {
            tracing::warn!(key = %key, error = %e, "failed to write to disk cache");
        }

        Ok(())
    }

    /// Get validation headers (ETag, Last-Modified) for conditional requests.
    ///
    /// Checks L1 first, then L2. Returns the headers regardless of freshness,
    /// since stale ETags are still valid for conditional requests.
    pub async fn get_validation_headers(&self, key: &str) -> Option<ValidationHeaders> {
        // Try L1 first
        if let Some(entry) = self.memory.get(key) {
            if entry.etag.is_some() || entry.last_modified.is_some() {
                return Some(ValidationHeaders {
                    etag: entry.etag,
                    last_modified: entry.last_modified,
                });
            }
        }

        // Fall back to L2
        if let Some((etag, last_modified)) = self.disk.get_validation_headers(key).await {
            if etag.is_some() || last_modified.is_some() {
                return Some(ValidationHeaders {
                    etag,
                    last_modified,
                });
            }
        }

        None
    }

    /// Refresh the TTL after receiving a 304 Not Modified response.
    ///
    /// Updates both L1 and L2 so the entry is considered fresh again.
    pub async fn refresh_ttl(&self, key: &str) -> Result<(), SubconvError> {
        // Refresh L1
        self.memory.refresh(key);

        // Refresh L2
        if let Err(e) = self.disk.refresh_ttl(key).await {
            tracing::warn!(key = %key, error = %e, "failed to refresh disk cache TTL");
            return Err(e);
        }

        tracing::debug!(key = %key, "refreshed cache TTL after 304");
        Ok(())
    }

    /// Evict expired entries from both layers.
    pub async fn evict_expired(&self) {
        let mem_evicted = self.memory.evict_expired();
        self.disk.evict().await;

        if mem_evicted > 0 {
            tracing::info!(count = mem_evicted, "evicted expired memory cache entries");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_config() -> AppConfig {
        // Use serde defaults by parsing an empty YAML string.
        // Override cache_dir to a temp location in actual tests.
        serde_yaml::from_str("").unwrap()
    }

    #[tokio::test]
    async fn put_and_get_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config();
        config.cache_dir = dir.path().to_string_lossy().to_string();
        config.cache_ttl = 3600;

        let cache = LayeredCache::new(&config);

        let content = CachedContent {
            body: Bytes::from_static(b"hello world"),
            content_type: "text/plain".into(),
            etag: Some("\"abc123\"".into()),
            last_modified: None,
        };

        cache.put("test-key", content).await.unwrap();
        let hit = cache.get("test-key").await.unwrap();
        assert_eq!(&hit.body[..], &b"hello world"[..]);
        assert_eq!(hit.etag.as_deref(), Some("\"abc123\""));
    }

    #[tokio::test]
    async fn get_returns_none_for_missing_key() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config();
        config.cache_dir = dir.path().to_string_lossy().to_string();

        let cache = LayeredCache::new(&config);
        assert!(cache.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn validation_headers_from_memory() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config();
        config.cache_dir = dir.path().to_string_lossy().to_string();
        config.cache_ttl = 3600;

        let cache = LayeredCache::new(&config);

        let content = CachedContent {
            body: Bytes::from_static(b"data"),
            content_type: "text/plain".into(),
            etag: Some("\"etag-val\"".into()),
            last_modified: Some("Wed, 01 Jan 2025 00:00:00 GMT".into()),
        };

        cache.put("vkey", content).await.unwrap();

        let headers = cache.get_validation_headers("vkey").await.unwrap();
        assert_eq!(headers.etag.as_deref(), Some("\"etag-val\""));
        assert_eq!(
            headers.last_modified.as_deref(),
            Some("Wed, 01 Jan 2025 00:00:00 GMT")
        );
    }

    #[tokio::test]
    async fn refresh_ttl_keeps_entry_fresh() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config();
        config.cache_dir = dir.path().to_string_lossy().to_string();
        config.cache_ttl = 3600;

        let cache = LayeredCache::new(&config);

        let content = CachedContent {
            body: Bytes::from_static(b"refresh-me"),
            content_type: "text/plain".into(),
            etag: Some("\"r1\"".into()),
            last_modified: None,
        };

        cache.put("refresh-key", content).await.unwrap();
        cache.refresh_ttl("refresh-key").await.unwrap();

        // Entry should still be fresh after refresh
        let hit = cache.get("refresh-key").await;
        assert!(hit.is_some());
    }

    #[tokio::test]
    async fn disk_backfills_memory() {
        let dir = tempfile::tempdir().unwrap();
        let mut config = make_config();
        config.cache_dir = dir.path().to_string_lossy().to_string();
        config.cache_ttl = 3600;

        let cache = LayeredCache::new(&config);

        let content = CachedContent {
            body: Bytes::from_static(b"backfill"),
            content_type: "text/plain".into(),
            etag: None,
            last_modified: None,
        };

        // Write to cache
        cache.put("bf-key", content).await.unwrap();

        // Manually evict from memory to simulate a memory miss
        cache.memory.remove("bf-key");
        assert!(cache.memory.get("bf-key").is_none());

        // get() should find it on disk and backfill memory
        let hit = cache.get("bf-key").await.unwrap();
        assert_eq!(&hit.body[..], &b"backfill"[..]);

        // Now memory should have it
        assert!(cache.memory.get("bf-key").is_some());
    }
}
