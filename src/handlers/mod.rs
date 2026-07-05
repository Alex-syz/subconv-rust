//! HTTP handlers for all API endpoints.
//!
//! Each endpoint is implemented in its own module for clarity:
//! - `sub` — subscription conversion (`GET /sub`)
//! - `provider` — proxy-provider output (`GET /provider`)
//! - `proxy` — proxied rule/template fetch (`GET /proxy`)
//! - `config` — runtime configuration (`GET /config`)
//! - `health` — health check (`GET /api/v1/health`)
//! - `robots` — robots.txt (`GET /robots.txt`)
//! - `static_files` — SPA static file serving

pub mod config;
pub mod health;
pub mod provider;
pub mod proxy;
pub mod robots;
pub mod static_files;
pub mod sub;
