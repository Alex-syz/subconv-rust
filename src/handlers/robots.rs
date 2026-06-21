//! Handler for `GET /robots.txt`.
//!
//! Returns a disallow-all robots.txt when `DISALLOW_ROBOTS` is true,
//! otherwise 404.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};

use crate::app::AppState;

pub async fn robots_handler(State(state): State<AppState>) -> Response {
    if state.config.disallow_robots {
        (
            StatusCode::OK,
            [("Content-Type", "text/plain")],
            "User-agent: *\nDisallow: /",
        )
            .into_response()
    } else {
        StatusCode::NOT_FOUND.into_response()
    }
}
