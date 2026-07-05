//! Handler for `GET /proxy` — proxied rule/template fetch with caching.
//!
//! This endpoint fetches remote rule files on behalf of Mihomo, with:
//! - **Whitelist validation**: the URL must appear in the active template's RULESET.
//! - **SSRF protection**: private/internal IPs are rejected.
//! - **Conditional requests**: ETag / Last-Modified support for 304 responses.
//! - **Layered cache**: L1 memory + L2 disk, with TTL refresh on 304.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use serde::Deserialize;

use crate::app::AppState;
use crate::cache::layer::CachedContent;
use crate::config::TemplateConfig;
use crate::error::SubconvError;

#[derive(Debug, Deserialize)]
pub struct ProxyParams {
    /// Required: URL to fetch (must be in the template's RULESET whitelist).
    pub url: String,
    /// Optional: template name (affects which whitelist is checked).
    pub template: Option<String>,
}

pub async fn proxy_handler(
    State(state): State<AppState>,
    Query(params): Query<ProxyParams>,
    headers: HeaderMap,
) -> Result<Response, SubconvError> {
    // 1. SSRF validation.
    crate::ssrf::validate_remote_url(&params.url)?;

    // 2. Resolve template and check whitelist.
    let template_name = params
        .template
        .as_deref()
        .unwrap_or(&state.config.default_template);
    let template_config = TemplateConfig::resolve_template(
        template_name,
        &state.config,
        &state.http_client,
    )
    .await?;

    let whitelisted = template_config
        .ruleset_urls()
        .iter()
        .any(|u| *u == params.url);

    if !whitelisted {
        return Err(SubconvError::Forbidden(
            "URL not in template RULESET whitelist".into(),
        ));
    }

    // 3. Check cache.
    let cache_key = &params.url;
    if let Some(cached) = state.cache.get(cache_key).await {
        // Cache hit and fresh — return directly.
        return build_response_from_cached(cached);
    }

    // 4. Cache miss or stale — try conditional request.
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("v2rayn");

    let mut request = state.http_client.get(&params.url);
    request = request.header("User-Agent", user_agent);

    // Add conditional headers if we have cached validation data.
    if let Some(validation) = state.cache.get_validation_headers(cache_key).await {
        if let Some(etag) = &validation.etag {
            request = request.header("If-None-Match", etag);
        }
        if let Some(lm) = &validation.last_modified {
            request = request.header("If-Modified-Since", lm);
        }
    }

    let resp = request.send().await.map_err(|e| {
        SubconvError::UpstreamFetch(format!("failed to fetch proxied URL: {e}"))
    })?;

    // 5. Handle 304 Not Modified.
    if resp.status() == axum::http::StatusCode::NOT_MODIFIED {
        // Refresh TTL and return cached content.
        state.cache.refresh_ttl(cache_key).await?;

        // Re-fetch from cache (now refreshed).
        if let Some(cached) = state.cache.get(cache_key).await {
            return build_response_from_cached(cached);
        }

        // Edge case: cache was evicted between refresh and get.
        // Fall through to re-fetch.
    }

    // 6. Handle error responses.
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(SubconvError::UpstreamFetch(format!(
            "upstream returned HTTP {status}: {body}"
        )));
    }

    // 7. Success — extract body and headers, store in cache.
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("text/plain")
        .to_string();

    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let last_modified = resp
        .headers()
        .get("last-modified")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let body = resp.bytes().await.map_err(|e| {
        SubconvError::UpstreamFetch(format!("failed to read proxied response body: {e}"))
    })?;

    let cached = CachedContent {
        body: body.clone(),
        content_type: content_type.clone(),
        etag: etag.clone(),
        last_modified: last_modified.clone(),
    };

    // Store in cache (non-fatal if it fails).
    if let Err(e) = state.cache.put(cache_key, cached).await {
        tracing::warn!(key = %cache_key, error = %e, "failed to cache proxied response");
    }

    // 8. Build response.
    let mut response = body.into_response();
    response.headers_mut().insert(
        "Content-Type",
        axum::http::HeaderValue::from_str(&content_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("text/plain")),
    );

    if let Some(etag) = etag {
        if let Ok(val) = axum::http::HeaderValue::from_str(&etag) {
            response.headers_mut().insert("ETag", val);
        }
    }
    if let Some(lm) = last_modified {
        if let Ok(val) = axum::http::HeaderValue::from_str(&lm) {
            response.headers_mut().insert("Last-Modified", val);
        }
    }

    Ok(response)
}

/// Build an HTTP response from cached content.
fn build_response_from_cached(cached: CachedContent) -> Result<Response, SubconvError> {
    let mut response = cached.body.into_response();
    response.headers_mut().insert(
        "Content-Type",
        axum::http::HeaderValue::from_str(&cached.content_type)
            .unwrap_or_else(|_| axum::http::HeaderValue::from_static("text/plain")),
    );

    if let Some(etag) = &cached.etag {
        if let Ok(val) = axum::http::HeaderValue::from_str(etag) {
            response.headers_mut().insert("ETag", val);
        }
    }
    if let Some(lm) = &cached.last_modified {
        if let Ok(val) = axum::http::HeaderValue::from_str(lm) {
            response.headers_mut().insert("Last-Modified", val);
        }
    }

    Ok(response)
}
