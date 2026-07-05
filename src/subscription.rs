//! Subscription fetching and parsing.
//!
//! Handles two concerns:
//! - **Fetching**: HTTP GET with configurable User-Agent, returning raw text.
//! - **Parsing**: Detects Clash YAML vs V2Ray share links, dispatches to the
//!   appropriate parser, and returns a flat `Vec<Proxy>`.
//!
//! `split_sources` classifies input tokens as remote subscription URLs or
//! standalone proxy links based on their URI scheme.

use crate::converter::proxy::Proxy;
use crate::converter::registry::NameRegistry;
use crate::converter::converts_v2ray;
use crate::error::SubconvError;

// ── Fetching ──────────────────────────────────────────────────────────────────

/// Fetch remote subscription content via HTTP GET.
///
/// Follows redirects. Returns the response body as a String.
pub async fn fetch_subscription(
    client: &reqwest::Client,
    url: &str,
    user_agent: &str,
) -> Result<reqwest::Response, SubconvError> {
    // Validate URL to prevent SSRF.
    crate::ssrf::validate_remote_url(url)?;

    let resp = client
        .get(url)
        .header("User-Agent", user_agent)
        .send()
        .await
        .map_err(|e| SubconvError::UpstreamFetch(format!("failed to fetch {url}: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(SubconvError::UpstreamFetch(format!(
            "upstream {url} returned HTTP {status}: {body}"
        )));
    }

    Ok(resp)
}

// ── Parsing ───────────────────────────────────────────────────────────────────

/// Parse subscription content: try Clash YAML first, fall back to V2Ray links.
///
/// Clash YAML is detected by the presence of a top-level `proxies` key.
/// If that fails, the content is treated as V2Ray share links (possibly
/// base64-encoded).
pub fn parse_subscription(
    content: &str,
    registry: &mut NameRegistry,
) -> Result<Vec<Proxy>, SubconvError> {
    // Try Clash YAML first.
    if let Ok(proxies) = parse_clash_yaml(content, registry) {
        if !proxies.is_empty() {
            return Ok(proxies);
        }
    }

    // Fall back to V2Ray share links.
    converts_v2ray(content, registry)
}

/// Extract proxy nodes from a Clash YAML subscription.
///
/// Looks for a top-level `proxies` array and deserializes each entry via
/// the `Proxy` enum's custom deserializer. Unrecognized proxy types are
/// captured in the `Unknown` variant and passed through intact.
fn parse_clash_yaml(
    content: &str,
    registry: &mut NameRegistry,
) -> Result<Vec<Proxy>, SubconvError> {
    let doc: serde_yaml::Value = serde_yaml::from_str(content)
        .map_err(|e| SubconvError::Parse(format!("invalid YAML: {e}")))?;

    let proxies_val = doc
        .get("proxies")
        .ok_or_else(|| SubconvError::Parse("no 'proxies' key in Clash YAML".into()))?;

    let proxy_list = proxies_val
        .as_sequence()
        .ok_or_else(|| SubconvError::Parse("'proxies' is not a sequence".into()))?;

    let mut result = Vec::with_capacity(proxy_list.len());
    for item in proxy_list {
        match serde_yaml::from_value::<Proxy>(item.clone()) {
            Ok(mut proxy) => {
                // Deduplicate name via registry.
                let unique_name = registry.register(proxy.name());
                proxy.set_name(&unique_name);
                result.push(proxy);
            }
            Err(e) => {
                tracing::debug!("skipping unparseable proxy entry: {e}");
            }
        }
    }

    Ok(result)
}

// ── Source splitting ──────────────────────────────────────────────────────────

/// URI schemes that indicate a standalone proxy link (not a subscription URL).
const STANDALONE_SCHEMES: &[&str] = &[
    "ss://",
    "ssr://",
    "vmess://",
    "vless://",
    "trojan://",
    "hysteria://",
    "hysteria2://",
    "tuic://",
    "socks5://",
    "tg://",
    "anytls://",
    "mierus://",
];

/// Split raw input into remote subscription URLs and standalone proxy links.
///
/// Tokens are separated by `|` or newlines. A token is classified as a
/// standalone link if it starts with a known proxy scheme or is a Telegram
/// proxy link (`https://t.me/`). Everything else is a remote subscription URL.
///
/// Returns `(remote_urls, standalone_links)`.
pub fn split_sources(input: &str) -> (Vec<String>, Vec<String>) {
    let mut remote_urls: Vec<String> = Vec::new();
    let mut standalone_links: Vec<String> = Vec::new();

    for token in input.split(['|', '\n']) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if is_standalone_link(token) {
            standalone_links.push(token.to_string());
        } else {
            remote_urls.push(token.to_string());
        }
    }

    (remote_urls, standalone_links)
}

/// Determine whether a token is a standalone proxy link.
fn is_standalone_link(token: &str) -> bool {
    // Check known proxy schemes.
    STANDALONE_SCHEMES.iter().any(|s| token.starts_with(s))
        || token.starts_with("https://t.me/")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_sources_separates_urls_and_links() {
        let input = "https://sub.example.com|ss://abc|vmess://def|https://other.com";
        let (urls, links) = split_sources(input);
        assert_eq!(urls, vec!["https://sub.example.com", "https://other.com"]);
        assert_eq!(links, vec!["ss://abc", "vmess://def"]);
    }

    #[test]
    fn split_sources_newline_separator() {
        let input = "https://sub.example.com\nss://abc\nhttps://other.com";
        let (urls, links) = split_sources(input);
        assert_eq!(urls, vec!["https://sub.example.com", "https://other.com"]);
        assert_eq!(links, vec!["ss://abc"]);
    }

    #[test]
    fn split_sources_telegram_link() {
        let input = "https://t.me/proxy?server=1.2.3.4";
        let (urls, links) = split_sources(input);
        assert!(urls.is_empty());
        assert_eq!(links, vec!["https://t.me/proxy?server=1.2.3.4"]);
    }

    #[test]
    fn split_sources_all_standalone_schemes() {
        let input = "ss://a|ssr://b|vmess://c|vless://d|trojan://e|hysteria://f|hysteria2://g|tuic://h|socks5://i|tg://j|anytls://k|mierus://l";
        let (urls, links) = split_sources(input);
        assert!(urls.is_empty());
        assert_eq!(links.len(), 12);
    }

    #[test]
    fn split_sources_empty_input() {
        let (urls, links) = split_sources("");
        assert!(urls.is_empty());
        assert!(links.is_empty());
    }

    #[test]
    fn split_sources_whitespace_trimmed() {
        let input = "  https://sub.example.com  |  ss://abc  ";
        let (urls, links) = split_sources(input);
        assert_eq!(urls, vec!["https://sub.example.com"]);
        assert_eq!(links, vec!["ss://abc"]);
    }

    #[test]
    fn parse_clash_yaml_extracts_proxies() {
        let yaml = r#"
proxies:
  - type: ss
    name: my-ss
    server: 1.2.3.4
    port: 8388
    cipher: aes-256-gcm
    password: pass123
  - type: trojan
    name: my-trojan
    server: 5.6.7.8
    port: 443
    password: trojan-pass
"#;
        let mut reg = NameRegistry::new();
        let proxies = parse_clash_yaml(yaml, &mut reg).unwrap();
        assert_eq!(proxies.len(), 2);
        assert!(matches!(proxies[0], Proxy::Ss(_)));
        assert!(matches!(proxies[1], Proxy::Trojan(_)));
    }

    #[test]
    fn parse_clash_yaml_unknown_type_passes_through() {
        let yaml = r#"
proxies:
  - type: wireguard
    name: wg1
    server: 10.0.0.1
    port: 51820
    private-key: abc
"#;
        let mut reg = NameRegistry::new();
        let proxies = parse_clash_yaml(yaml, &mut reg).unwrap();
        assert_eq!(proxies.len(), 1);
        assert!(matches!(proxies[0], Proxy::Unknown(_)));
        assert_eq!(proxies[0].name(), "wg1");
    }

    #[test]
    fn parse_clash_yaml_deduplicates_names() {
        let yaml = r#"
proxies:
  - type: ss
    name: dup
    server: 1.1.1.1
    port: 8388
    cipher: aes-256-gcm
    password: a
  - type: ss
    name: dup
    server: 2.2.2.2
    port: 8388
    cipher: aes-256-gcm
    password: b
"#;
        let mut reg = NameRegistry::new();
        let proxies = parse_clash_yaml(yaml, &mut reg).unwrap();
        assert_eq!(proxies.len(), 2);
        assert_eq!(proxies[0].name(), "dup");
        assert_eq!(proxies[1].name(), "dup-01");
    }

    #[test]
    fn parse_subscription_falls_back_to_v2ray() {
        // Content without a 'proxies' key should fall back to V2Ray parsing.
        let content = "trojan://password123@server:443?sni=example.com#TestTrojan";
        let mut reg = NameRegistry::new();
        let proxies = parse_subscription(content, &mut reg).unwrap();
        assert_eq!(proxies.len(), 1);
        assert!(matches!(proxies[0], Proxy::Trojan(_)));
    }

    #[test]
    fn parse_subscription_prefers_clash_yaml() {
        let yaml = r#"
proxies:
  - type: ss
    name: clash-ss
    server: 1.2.3.4
    port: 8388
    cipher: aes-256-gcm
    password: pass
"#;
        let mut reg = NameRegistry::new();
        let proxies = parse_subscription(yaml, &mut reg).unwrap();
        assert_eq!(proxies.len(), 1);
        assert!(matches!(proxies[0], Proxy::Ss(_)));
        assert_eq!(proxies[0].name(), "clash-ss");
    }
}
