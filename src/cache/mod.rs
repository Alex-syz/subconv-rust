//! Cache layer for remote rule files and templates.
//!
//! Two-level architecture:
//! - **L1** (`memory`): In-memory `HashMap` protected by `std::sync::RwLock`.
//!   Fast lookups, limited capacity, lost on restart.
//! - **L2** (`disk`): File-based cache with JSON metadata sidecars.
//!   Persists across restarts, survives container recycling.
//!
//! The `layer` module combines both into a single `LayeredCache` that
//! provides the public API used by the rest of the application.

pub mod disk;
pub mod layer;
pub mod memory;
pub mod subscription;

pub use layer::LayeredCache;
pub use subscription::SubCache;

use std::sync::Arc;
use std::time::Duration;

/// Start a background task that periodically evicts expired cache entries.
///
/// The task runs every `cleanup_interval` and removes stale entries from
/// both L1 and L2. It stops when the `JoinHandle` is dropped or the
/// Tokio runtime shuts down.
pub fn start_cleanup_task(
    cache: Arc<LayeredCache>,
    cleanup_interval: Duration,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(cleanup_interval);
        // First tick completes immediately; skip it so we don't evict
        // right after startup when the disk cache is just being populated.
        interval.tick().await;

        loop {
            interval.tick().await;
            tracing::debug!("running periodic cache cleanup");
            cache.evict_expired().await;
        }
    })
}
