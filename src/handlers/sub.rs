//! Handler for `GET /sub` — the main subscription conversion endpoint.
//!
//! Accepts subscription URLs and/or standalone proxy links, fetches and parses
//! them, then assembles a complete Mihomo YAML config using the selected template.

use axum::extract::{Query, State};
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::Deserialize;

use crate::app::AppState;
use crate::config::TemplateConfig;
use crate::converter::registry::NameRegistry;
use crate::error::SubconvError;
use crate::packer;
use crate::subscription;

// ── Query parameters ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SubParams {
    /// Required: subscription URL(s), multiple separated by `|`.
    pub url: String,
    /// Optional: template name (defaults to configured default).
    pub template: Option<String>,
    /// Optional: proxy-provider refresh interval in seconds.
    pub interval: Option<String>,
    /// Optional: if present, omit the HEAD section.
    pub short: Option<String>,
    /// Optional: if present, rule URLs are not proxied through /proxy.
    pub npr: Option<String>,
    /// Optional: standby subscription URL(s).
    pub urlstandby: Option<String>,
}

// ── Handler ───────────────────────────────────────────────────────────────────

pub async fn sub_handler(
    State(state): State<AppState>,
    Query(params): Query<SubParams>,
    headers: HeaderMap,
    uri: axum::http::Uri,
) -> Result<Response, SubconvError> {
    let interval = params.interval.as_deref().unwrap_or("1800");
    let short = params.short.is_some();
    let notproxyrule = params.npr.is_some();

    // Check subscription cache.
    let cache_key = format!("sub:{}", uri.query().unwrap_or(""));
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        if let Some(info) = cached.subscription_userinfo {
            if let Ok(val) = axum::http::HeaderValue::from_str(&info) {
                resp.headers_mut().insert("subscription-userinfo", val);
            }
        }
        if let Some(disp) = cached.content_disposition {
            if let Ok(val) = axum::http::HeaderValue::from_str(&disp) {
                resp.headers_mut().insert("Content-Disposition", val);
            }
        }
        return Ok(resp);
    }

    // Request coalescing: wait for in-flight request or proceed.
    let lock = state.sub_cache.get_or_create_lock(&cache_key);
    let guard = tokio::time::timeout(state.sub_cache.lock_timeout(), lock.lock())
        .await;
    let _guard = guard.ok();

    // Double-check cache after acquiring lock.
    if let Some(cached) = state.sub_cache.get(&cache_key) {
        let mut resp = cached.body.into_response();
        resp.headers_mut().insert(
            "Content-Type",
            axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
        );
        if let Some(info) = cached.subscription_userinfo {
            if let Ok(val) = axum::http::HeaderValue::from_str(&info) {
                resp.headers_mut().insert("subscription-userinfo", val);
            }
        }
        if let Some(disp) = cached.content_disposition {
            if let Ok(val) = axum::http::HeaderValue::from_str(&disp) {
                resp.headers_mut().insert("Content-Disposition", val);
            }
        }
        return Ok(resp);
    }
    // If lock timed out, we proceed without the lock. This may result in
    // duplicate fetches but is not a correctness issue — the cache write
    // is protected by RwLock and the response is still valid.

    // Resolve template.
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

    // Split primary sources.
    let (remote_urls, standalone_links) = subscription::split_sources(&params.url);

    // Split standby sources.
    let (standby_urls, standby_links) = params
        .urlstandby
        .as_deref()
        .map(subscription::split_sources)
        .unwrap_or_default();

    // Parse standalone proxy links.
    let mut registry = NameRegistry::new();
    let standalone_proxies = if !standalone_links.is_empty() {
        let joined = standalone_links.join("\n");
        Some(crate::converter::converts_v2ray(&joined, &mut registry)?)
    } else {
        None
    };

    let standby_proxies = if !standby_links.is_empty() {
        let joined = standby_links.join("\n");
        Some(crate::converter::converts_v2ray(&joined, &mut registry)?)
    } else {
        None
    };

    // Fetch and parse remote subscriptions.
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("v2rayn");

    let mut subscription_userinfo: Option<String> = None;
    let mut content_disposition: Option<String> = None;

    // Collect proxy names from providers for group regex matching.
    let mut provider_proxy_names: Vec<String> = Vec::new();

    for (i, url) in remote_urls.iter().enumerate() {
        let resp = subscription::fetch_subscription(&state.http_client, url, user_agent).await?;
        let resp_headers = resp.headers().clone();
        let text = resp.text().await.map_err(|e| {
            SubconvError::UpstreamFetch(format!("failed to read response body: {e}"))
        })?;

        let mut sub_reg = NameRegistry::new();
        let proxies = subscription::parse_subscription(&text, &mut sub_reg)?;

        // Collect proxy names from this subscription for group matching.
        provider_proxy_names.extend(proxies.iter().map(|p| p.name().to_string()));

        // Capture headers from the first subscription only.
        if i == 0 {
            if let Some(val) = resp_headers.get("subscription-userinfo") {
                subscription_userinfo = val.to_str().ok().map(|s| s.to_string());
            }
            if let Some(val) = resp_headers.get("content-disposition") {
                content_disposition = val
                    .to_str()
                    .ok()
                    .map(|s| s.replace("attachment", "inline"));
            }
        }
    }

    // Also collect names from standalone proxies.
    if let Some(ref proxies) = standalone_proxies {
        provider_proxy_names.extend(proxies.iter().map(|p| p.name().to_string()));
    }
    if let Some(ref proxies) = standby_proxies {
        provider_proxy_names.extend(proxies.iter().map(|p| p.name().to_string()));
    }

    // Determine base URL and domain from request headers.
    let (base_url, domain) = extract_base_url_and_domain(&headers)?;

    // Assemble the final config.
    let yaml = packer::pack(
        standalone_proxies.unwrap_or_default(),
        standby_proxies.unwrap_or_default(),
        &remote_urls,
        &standby_urls,
        interval,
        &base_url,
        template_name,
        &template_config,
        short,
        notproxyrule,
        &provider_proxy_names,
        &domain,
    )?;

    // Build response with transparent headers.
    let resp_body = Bytes::from(yaml.clone());
    let mut resp = yaml.into_response();
    resp.headers_mut().insert(
        "Content-Type",
        axum::http::HeaderValue::from_static("text/yaml;charset=utf-8"),
    );

    if let Some(ref info) = subscription_userinfo {
        if let Ok(val) = axum::http::HeaderValue::from_str(info) {
            resp.headers_mut().insert("subscription-userinfo", val);
        }
    }
    if let Some(ref disp) = content_disposition {
        if let Ok(val) = axum::http::HeaderValue::from_str(disp) {
            resp.headers_mut().insert("Content-Disposition", val);
        }
    }

    // Cache the successful response.
    let cache_entry = crate::cache::subscription::CacheEntry {
        body: resp_body.clone(),
        subscription_userinfo: subscription_userinfo.clone(),
        content_disposition: content_disposition.clone(),
        fetched_at: std::time::Instant::now(),
    };
    state.sub_cache.put(cache_key, cache_entry);

    Ok(resp)
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract the base URL and domain from request headers.
///
/// Uses the `Host` header to construct `http(s)://{host}/` as the base URL.
/// The domain is the hostname portion (without port).
fn extract_base_url_and_domain(headers: &HeaderMap) -> Result<(String, String), SubconvError> {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| SubconvError::BadRequest("missing Host header".into()))?;

    // Determine scheme: assume HTTPS if standard port or common proxy headers.
    let scheme = if host.ends_with(":443") || headers.get("x-forwarded-proto").and_then(|v| v.to_str().ok()) == Some("https") {
        "https"
    } else {
        "http"
    };

    let base_url = format!("{scheme}://{host}/");

    // Extract domain (strip port).
    let domain = host.split(':').next().unwrap_or(host).to_string();

    Ok((base_url, domain))
}
