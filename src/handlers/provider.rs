//! Handler for `GET /provider` — proxy-provider output.
//!
//! Fetches and parses a subscription URL, then returns only the `proxies`
//! array as YAML. This is used by Mihomo's `proxy-provider` feature to
//! fetch node lists independently of the main config.

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::app::AppState;
use crate::converter::registry::NameRegistry;
use crate::error::SubconvError;
use crate::subscription;

#[derive(Debug, Deserialize)]
pub struct ProviderParams {
    /// Required: subscription URL to fetch and parse.
    pub url: String,
}

pub async fn provider_handler(
    State(state): State<AppState>,
    Query(params): Query<ProviderParams>,
    headers: axum::http::HeaderMap,
) -> Result<Response, SubconvError> {
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("v2rayn");

    let resp = subscription::fetch_subscription(&state.http_client, &params.url, user_agent).await?;
    let text = resp.text().await.map_err(|e| {
        SubconvError::UpstreamFetch(format!("failed to read provider response: {e}"))
    })?;

    let mut registry = NameRegistry::new();
    let proxies = subscription::parse_subscription(&text, &mut registry)?;

    // Serialize as a proxy-provider compatible YAML with `proxies:` key.
    // Mihomo proxy-provider expects: `proxies:\n  - type: ...\n  - ...`
    let proxies_value = serde_yaml::to_value(&proxies)?;
    let mut map = serde_yaml::Mapping::new();
    map.insert(
        serde_yaml::Value::String("proxies".into()),
        proxies_value,
    );
    let yaml = serde_yaml::to_string(&map)?;

    let mut response = yaml.into_response();
    response.headers_mut().insert(
        "Content-Type",
        axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
    );

    Ok(response)
}
