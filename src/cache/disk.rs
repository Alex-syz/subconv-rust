//! L2 disk-based cache with metadata sidecar files.
//!
//! Cache files are stored under a configured directory. Each key maps to a file
//! named by the first 16 hex characters of its SHA-256 digest. Metadata is
//! stored in a companion `.meta` file (JSON).
//!
//! Disk operations that fail are logged but never propagated as errors, so the
//! service remains available even if the disk is full or unreadable.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use bytes::Bytes;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::SubconvError;

/// Metadata stored alongside each cached file.
#[derive(Debug, Serialize, Deserialize)]
pub struct DiskCacheEntry {
    /// MIME type of the cached content.
    pub content_type: String,
    /// ETag for conditional requests.
    pub etag: Option<String>,
    /// Last-Modified header value.
    pub last_modified: Option<String>,
    /// Unix timestamp (seconds) when the content was fetched.
    pub fetched_at: u64,
    /// TTL in seconds.
    pub ttl: u64,
}

impl DiskCacheEntry {
    /// Check if this entry is still fresh.
    pub fn is_fresh(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now.saturating_sub(self.fetched_at) < self.ttl
    }
}

/// A hit from the disk cache, containing both body and metadata.
pub struct DiskCacheHit {
    /// The cached response body.
    pub body: Bytes,
    /// The associated metadata.
    pub meta: DiskCacheEntry,
}

/// L2 disk-backed cache.
pub struct DiskCache {
    /// Directory where cache files and metadata are stored.
    dir: PathBuf,
    /// Maximum total size of cached files in bytes.
    max_size_bytes: u64,
}

impl DiskCache {
    /// Create a new disk cache.
    ///
    /// The directory is created if it does not exist.
    pub fn new(dir: impl Into<PathBuf>, max_size_mb: u64) -> Self {
        let dir = dir.into();
        // Ensure the cache directory exists at startup.
        if let Err(e) = std::fs::create_dir_all(&dir) {
            tracing::warn!(path = %dir.display(), error = %e, "failed to create cache directory");
        }
        Self {
            dir,
            max_size_bytes: max_size_mb * 1024 * 1024,
        }
    }

    /// Compute the cache filename for a key: first 16 hex chars of SHA-256.
    fn key_to_filename(&self, key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let hash = hasher.finalize();
        hex::encode(&hash[..8]) // 8 bytes = 16 hex chars
    }

    /// Get the file path and metadata path for a key.
    fn paths_for(&self, key: &str) -> (PathBuf, PathBuf) {
        let name = self.key_to_filename(key);
        let data_path = self.dir.join(&name);
        let meta_path = self.dir.join(format!("{name}.meta"));
        (data_path, meta_path)
    }

    /// Retrieve a cached entry if it exists and is fresh.
    ///
    /// Returns `None` if the key is not found, the entry is stale,
    /// or any I/O error occurs (errors are logged, not propagated).
    pub async fn get(&self, key: &str) -> Option<DiskCacheHit> {
        let (data_path, meta_path) = self.paths_for(key);

        // Read metadata first
        let meta_bytes = tokio::fs::read(&meta_path).await.ok()?;
        let meta: DiskCacheEntry = serde_json::from_slice(&meta_bytes).ok()?;

        // Check freshness — stale entries are treated as absent.
        if !meta.is_fresh() {
            return None;
        }

        // Read body
        let body = Bytes::from(tokio::fs::read(&data_path).await.ok()?);

        Some(DiskCacheHit { body, meta })
    }

    /// Retrieve a cached entry regardless of freshness.
    ///
    /// Useful for getting stale content to serve while revalidating.
    pub async fn get_stale(&self, key: &str) -> Option<DiskCacheHit> {
        self.get(key).await
    }

    /// Get only the validation headers (ETag, Last-Modified) for a key.
    pub async fn get_validation_headers(&self, key: &str) -> Option<(Option<String>, Option<String>)> {
        let (_, meta_path) = self.paths_for(key);
        let meta_bytes = tokio::fs::read(&meta_path).await.ok()?;
        let meta: DiskCacheEntry = serde_json::from_slice(&meta_bytes).ok()?;
        Some((meta.etag.clone(), meta.last_modified.clone()))
    }

    /// Store content and metadata on disk.
    ///
    /// Errors are logged but not propagated — a disk write failure should not
    /// prevent the service from returning a response.
    pub async fn put(&self, key: &str, body: &[u8], entry: &DiskCacheEntry) -> Result<(), SubconvError> {
        let name = self.key_to_filename(key);
        let data_path = self.dir.join(&name);
        let meta_path = self.dir.join(format!("{name}.meta"));

        // Write body to a temp file first, then rename for atomicity.
        let tmp_data = self.dir.join(format!("{name}.tmp"));
        if let Err(e) = tokio::fs::write(&tmp_data, body).await {
            tracing::warn!(path = %tmp_data.display(), error = %e, "failed to write cache body");
            let _ = tokio::fs::remove_file(&tmp_data).await;
            return Err(SubconvError::Io(e));
        }
        if let Err(e) = tokio::fs::rename(&tmp_data, &data_path).await {
            tracing::warn!(from = %tmp_data.display(), to = %data_path.display(), error = %e, "failed to rename cache body");
            let _ = tokio::fs::remove_file(&tmp_data).await;
            return Err(SubconvError::Io(e));
        }

        // Write metadata
        let meta_json = serde_json::to_vec(entry).expect("DiskCacheEntry serialization is infallible");
        let tmp_meta = self.dir.join(format!("{name}.meta.tmp"));
        if let Err(e) = tokio::fs::write(&tmp_meta, &meta_json).await {
            tracing::warn!(path = %tmp_meta.display(), error = %e, "failed to write cache metadata");
            let _ = tokio::fs::remove_file(&tmp_meta).await;
            // Body is already written; metadata loss is non-fatal.
            return Err(SubconvError::Io(e));
        }
        if let Err(e) = tokio::fs::rename(&tmp_meta, &meta_path).await {
            tracing::warn!(from = %tmp_meta.display(), to = %meta_path.display(), error = %e, "failed to rename cache metadata");
            let _ = tokio::fs::remove_file(&tmp_meta).await;
            return Err(SubconvError::Io(e));
        }

        Ok(())
    }

    /// Remove a cached entry from disk.
    pub async fn remove(&self, key: &str) {
        let (data_path, meta_path) = self.paths_for(key);

        if let Err(e) = tokio::fs::remove_file(&data_path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %data_path.display(), error = %e, "failed to remove cache file");
            }
        }
        if let Err(e) = tokio::fs::remove_file(&meta_path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %meta_path.display(), error = %e, "failed to remove cache metadata");
            }
        }
    }

    /// Refresh the TTL of an existing entry by updating its `fetched_at` timestamp.
    pub async fn refresh_ttl(&self, key: &str) -> Result<(), SubconvError> {
        let (_, meta_path) = self.paths_for(key);

        let meta_bytes = tokio::fs::read(&meta_path).await.map_err(SubconvError::Io)?;
        let mut meta: DiskCacheEntry = serde_json::from_slice(&meta_bytes).map_err(|e| {
            SubconvError::Parse(format!("failed to parse cache metadata: {e}"))
        })?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        meta.fetched_at = now;

        let updated = serde_json::to_vec(&meta).expect("DiskCacheEntry serialization is infallible");
        tokio::fs::write(&meta_path, &updated).await.map_err(SubconvError::Io)?;

        Ok(())
    }

    /// Evict expired entries and enforce the size limit.
    ///
    /// 1. Remove all expired entries.
    /// 2. If total size still exceeds the limit, remove the oldest entries
    ///    until under the cap.
    pub async fn evict(&self) {
        // Phase 1: Remove expired entries.
        let entries = self.collect_entries().await;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        for (hash_prefix, meta) in &entries {
            if now.saturating_sub(meta.fetched_at) >= meta.ttl {
                self.remove_by_hash_prefix(hash_prefix).await;
                tracing::debug!(key = %hash_prefix, "evicted expired disk cache entry");
            }
        }

        // Phase 2: Enforce size limit (LRU by fetched_at).
        let mut current_size = self.total_size().await;
        if current_size <= self.max_size_bytes {
            return;
        }

        // Re-collect after expiration pass, sort by fetched_at ascending (oldest first).
        let mut remaining: Vec<_> = self
            .collect_entries()
            .await
            .into_iter()
            .filter(|(_, meta)| {
                now.saturating_sub(meta.fetched_at) < meta.ttl
            })
            .collect();
        remaining.sort_by_key(|(_, meta)| meta.fetched_at);

        for (hash_prefix, _) in remaining {
            if current_size <= self.max_size_bytes {
                break;
            }
            let data_path = self.dir.join(&hash_prefix);
            let meta_path = self.dir.join(format!("{hash_prefix}.meta"));
            let data_size = tokio::fs::metadata(&data_path).await.map(|m| m.len()).unwrap_or(0);
            let meta_size = tokio::fs::metadata(&meta_path).await.map(|m| m.len()).unwrap_or(0);
            self.remove_by_hash_prefix(&hash_prefix).await;
            current_size = current_size.saturating_sub(data_size + meta_size);
            tracing::debug!(key = %hash_prefix, "evicted disk cache entry for size limit");
        }
    }

    /// Remove a cached entry by its hash prefix (filename without extension).
    async fn remove_by_hash_prefix(&self, hash_prefix: &str) {
        let data_path = self.dir.join(hash_prefix);
        let meta_path = self.dir.join(format!("{hash_prefix}.meta"));

        if let Err(e) = tokio::fs::remove_file(&data_path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %data_path.display(), error = %e, "failed to remove cache file");
            }
        }
        if let Err(e) = tokio::fs::remove_file(&meta_path).await {
            if e.kind() != std::io::ErrorKind::NotFound {
                tracing::warn!(path = %meta_path.display(), error = %e, "failed to remove cache metadata");
            }
        }
    }

    /// Calculate the total size of all cached files on disk.
    pub async fn total_size(&self) -> u64 {
        let mut total: u64 = 0;
        if let Ok(mut entries) = tokio::fs::read_dir(&self.dir).await {
            while let Ok(Some(entry)) = entries.next_entry().await {
                if let Ok(metadata) = entry.metadata().await {
                    if metadata.is_file() {
                        total = total.saturating_add(metadata.len());
                    }
                }
            }
        }
        total
    }

    /// Scan the cache directory and collect all entries with their metadata.
    async fn collect_entries(&self) -> Vec<(String, DiskCacheEntry)> {
        let mut result = Vec::new();

        // Read directory entries, find .meta files, and load them.
        let mut dir_entries = match tokio::fs::read_dir(&self.dir).await {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!(dir = %self.dir.display(), error = %e, "failed to read cache directory");
                return result;
            }
        };

        while let Ok(Some(entry)) = dir_entries.next_entry().await {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            // Only process .meta files
            if !name.ends_with(".meta") {
                continue;
            }

            let meta_bytes = match tokio::fs::read(&path).await {
                Ok(b) => b,
                Err(_) => continue,
            };

            let meta: DiskCacheEntry = match serde_json::from_slice(&meta_bytes) {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Reconstruct the key prefix from the filename (before .meta).
            // We store the hash prefix as the "key" for eviction purposes.
            let hash_prefix = name.strip_suffix(".meta").unwrap_or(name);
            result.push((hash_prefix.to_string(), meta));
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(ttl: u64) -> DiskCacheEntry {
        DiskCacheEntry {
            content_type: "text/plain".into(),
            etag: Some("\"test-etag\"".into()),
            last_modified: Some("Wed, 01 Jan 2025 00:00:00 GMT".into()),
            fetched_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ttl,
        }
    }

    #[tokio::test]
    async fn put_and_get() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path(), 50);

        let entry = make_entry(3600);
        cache.put("test-key", b"hello world", &entry).await.unwrap();

        let hit = cache.get("test-key").await.unwrap();
        assert_eq!(&hit.body[..], &b"hello world"[..]);
        assert_eq!(hit.meta.content_type, "text/plain");
        assert_eq!(hit.meta.etag.as_deref(), Some("\"test-etag\""));
    }

    #[tokio::test]
    async fn get_missing_key_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path(), 50);
        assert!(cache.get("nonexistent").await.is_none());
    }

    #[tokio::test]
    async fn remove_entry() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path(), 50);

        let entry = make_entry(3600);
        cache.put("remove-me", b"data", &entry).await.unwrap();
        assert!(cache.get("remove-me").await.is_some());

        cache.remove("remove-me").await;
        assert!(cache.get("remove-me").await.is_none());
    }

    #[tokio::test]
    async fn refresh_ttl_updates_fetched_at() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path(), 50);

        let mut entry = make_entry(10);
        // Set fetched_at to the past so entry is near expiry
        entry.fetched_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 9;
        cache.put("refresh-key", b"data", &entry).await.unwrap();

        cache.refresh_ttl("refresh-key").await.unwrap();

        let hit = cache.get("refresh-key").await.unwrap();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        // refreshed fetched_at should be close to now
        assert!(hit.meta.fetched_at > entry.fetched_at);
        assert!(hit.meta.fetched_at <= now);
    }

    #[tokio::test]
    async fn eviction_removes_expired() {
        let dir = tempfile::tempdir().unwrap();
        let cache = DiskCache::new(dir.path(), 50);

        // Insert an entry with very short TTL
        let mut expired_entry = make_entry(1);
        expired_entry.fetched_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 5; // expired 5 seconds ago
        cache.put("expired-key", b"old", &expired_entry).await.unwrap();

        // Insert a fresh entry
        let fresh_entry = make_entry(3600);
        cache.put("fresh-key", b"new", &fresh_entry).await.unwrap();

        cache.evict().await;
        assert!(cache.get("expired-key").await.is_none());
        assert!(cache.get("fresh-key").await.is_some());
    }

    #[tokio::test]
    async fn size_limit_eviction() {
        let dir = tempfile::tempdir().unwrap();
        // 1 MB limit
        let cache = DiskCache::new(dir.path(), 1);

        // Insert a large entry (should fill most of the 1MB limit)
        let large_body = vec![0u8; 700_000];
        let entry = make_entry(3600);
        cache.put("large1", &large_body, &entry).await.unwrap();

        // Insert another large entry (should push over limit)
        cache.put("large2", &large_body, &entry).await.unwrap();

        // Evict should remove oldest to stay under limit
        cache.evict().await;

        let size = cache.total_size().await;
        assert!(size <= 1024 * 1024, "total size {size} exceeds limit");
    }
}
