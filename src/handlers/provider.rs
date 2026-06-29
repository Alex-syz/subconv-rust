//! Handler for `GET /provider` — proxy-provider output.
//!
//! Fetches and parses a subscription URL, then returns only the `proxies`
//! array as YAML. This is used by Mihomo's `proxy-provider` feature to
//! fetch node lists independently of the main config.

use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
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
    uri: axum::http::Uri,
) -> Result<Response, SubconvError> {
    // Check subscription cache.
    let cache_key = format!("provider:{}", uri.query().unwrap_or(""));
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        return Ok(resp);
    }

    // Request coalescing.
    let lock = state.sub_cache.get_or_create_lock(&cache_key);
    let guard = tokio::time::timeout(state.sub_cache.lock_timeout(), lock.lock())
        .await;
    let _guard = guard.ok();

    // Double-check.
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        return Ok(resp);
    }

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

    let yaml_bytes = Bytes::from(yaml.clone());
    let mut response = yaml.into_response();
    response.headers_mut().insert(
        "Content-Type",
        axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
    );

    // Cache the successful response.
    let cache_entry = crate::cache::subscription::CacheEntry {
        body: yaml_bytes,
        subscription_userinfo: None,
        content_disposition: None,
        fetched_at: std::time::Instant::now(),
    };
    state.sub_cache.put(cache_key, cache_entry);

    Ok(response)
}
