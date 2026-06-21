//! Handler for `GET /config` — runtime configuration JSON.
//!
//! Returns the default template name and a list of available template names
//! so that the frontend can populate a template selector dropdown.

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

use crate::app::AppState;

pub async fn config_handler(State(state): State<AppState>) -> Response {
    let body = json!({
        "defaultTemplate": state.config.default_template_name(),
        "availableTemplates": state.config.available_templates(),
    });

    (StatusCode::OK, axum::Json(body)).into_response()
}
