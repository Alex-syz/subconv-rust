//! Axum application builder and shared state.
//!
//! `AppState` holds all shared resources (config, HTTP client, cache) and is
//! injected into handlers via Axum's `State` extractor. `build_app` wires up
//! all routes and middleware.

use std::sync::Arc;

use axum::Router;
use axum::routing::get;

use crate::cache::LayeredCache;
use crate::cache::SubCache;
use crate::config::AppConfig;
use crate::handlers;

/// Shared application state, cloned cheaply via `Arc` internals.
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<AppConfig>,
    pub http_client: reqwest::Client,
    pub cache: Arc<LayeredCache>,
    pub sub_cache: Arc<SubCache>,
}

/// Build the Axum `Router` with all routes and middleware.
///
/// Routes:
/// - `GET /` — serve index.html (SPA entry)
/// - `GET /sub` — subscription conversion
/// - `GET /provider` — proxy-provider output
/// - `GET /proxy` — proxied rule/template fetch (whitelisted + SSRF-guarded)
/// - `GET /config` — runtime configuration JSON
/// - `GET /robots.txt` — robots.txt
/// - `GET /api/v1/health` — health check
/// - `GET /*path` — static file fallback (SPA)
pub fn build_app(state: AppState) -> Router {
    Router::new()
        .route("/", get(handlers::static_files::serve_index))
        .route("/sub", get(handlers::sub::sub_handler))
        .route("/provider", get(handlers::provider::provider_handler))
        .route("/proxy", get(handlers::proxy::proxy_handler))
        .route("/config", get(handlers::config::config_handler))
        .route("/robots.txt", get(handlers::robots::robots_handler))
        .route("/api/v1/health", get(handlers::health::health_handler))
        .fallback(get(handlers::static_files::serve_static))
        .with_state(state)
}
