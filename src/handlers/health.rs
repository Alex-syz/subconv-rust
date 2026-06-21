//! Handler for `GET /api/v1/health` — health check endpoint.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

pub async fn health_handler() -> Response {
    let body = json!({
        "status": "ok",
        "version": env!("CARGO_PKG_VERSION"),
    });

    (StatusCode::OK, axum::Json(body)).into_response()
}
