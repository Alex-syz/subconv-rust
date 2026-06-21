//! SubConv entry point.
//!
//! Initializes tracing, loads configuration, builds shared state, and starts
//! the Axum HTTP server with graceful shutdown.

use std::sync::Arc;
use std::time::Duration;

use subconv::app::{AppState, build_app};
use subconv::cache::LayeredCache;
use subconv::config::AppConfig;
use subconv::SubconvError;

#[tokio::main]
async fn main() -> Result<(), SubconvError> {
    // 1. Initialize tracing.
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tracing::info!("subconv starting");

    // 2. Load application configuration.
    let config = AppConfig::load()?;
    let host = config.host.clone();
    let port = config.port;
    tracing::info!(
        host = %host,
        port = %port,
        default_template = %config.default_template,
        "configuration loaded"
    );

    // 3. Build reqwest HTTP client.
    let http_client = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| SubconvError::Config(format!("failed to build HTTP client: {e}")))?;

    // 4. Build layered cache.
    let cache = Arc::new(LayeredCache::new(&config));

    // 5. Start cache cleanup background task (every 10 minutes).
    let cleanup_handle = subconv::cache::start_cleanup_task(
        Arc::clone(&cache),
        Duration::from_secs(600),
    );

    // 6. Validate templates on startup.
    let available = config.available_templates();
    if available.is_empty() {
        tracing::warn!("no templates found; the /sub endpoint will fail without templates");
    } else {
        tracing::info!(templates = ?available, "available templates");
    }

    let default = config.default_template_name();
    if !available.contains(&default.to_string()) {
        tracing::warn!(
            template = %default,
            "default template not found in available templates"
        );
    }

    // 7. Build shared state.
    let state = AppState {
        config: Arc::new(config),
        http_client,
        cache,
    };

    // 8. Build Axum app.
    let app = build_app(state);

    // 9. Bind and serve.
    let listener = tokio::net::TcpListener::bind(format!("{host}:{port}"))
        .await
        .map_err(|e| SubconvError::Config(format!("failed to bind {host}:{port}: {e}")))?;

    tracing::info!("listening on {host}:{port}");

    // 10. Serve with graceful shutdown.
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|e| SubconvError::Config(format!("server error: {e}")))?;

    // Clean up background tasks.
    cleanup_handle.abort();

    tracing::info!("subconv shutting down");
    Ok(())
}

/// Wait for SIGINT (Ctrl+C) or SIGTERM to trigger graceful shutdown.
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("received Ctrl+C, shutting down");
        },
        _ = terminate => {
            tracing::info!("received SIGTERM, shutting down");
        },
    }
}
