//! SubConv - Subscription converter for Mihomo (Clash Meta)
//!
//! A Rust rewrite of the Python SubConv project, using idiomatic Rust patterns
//! and strong typing throughout.

pub mod app;
pub mod cache;
pub mod config;
pub mod converter;
pub mod error;
pub mod handlers;
pub mod packer;
pub mod ssrf;
pub mod subscription;

pub mod types;

pub use error::SubconvError;
