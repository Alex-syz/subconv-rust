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

