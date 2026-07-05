//! Telegram proxy share link parser.
//!
//! Formats:
//! - `tg://proxy?server=xxx&port=xxx&user=xxx&pass=xxx`
//! - `https://t.me/proxy?server=xxx&port=xxx&user=xxx&pass=xxx`

use indexmap::IndexMap;
use url::Url;

use super::proxy::{Proxy, TelegramProxy};
use super::registry::NameRegistry;
use crate::SubconvError;

/// Parse a `tg://` share link into a Proxy.
pub fn parse_telegram(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("telegram: {e}")))?;

    let proxy_type = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("telegram: missing proxy type".into()))?;

    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();

    build_telegram_proxy(proxy_type, &query, registry)
}

/// Parse a `https://t.me/...` share link into a Proxy.
pub fn parse_telegram_https(
    uri: &str,
    registry: &mut NameRegistry,
) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("telegram https: {e}")))?;

    let hostname = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("telegram https: invalid host".into()))?;

    if hostname != "t.me" {
        return Err(SubconvError::Parse("telegram https: invalid host".into()));
    }

    let proxy_type = url.path().trim_start_matches('/');
    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();

    build_telegram_proxy(proxy_type, &query, registry)
}

fn build_telegram_proxy(
    proxy_type: &str,
    query: &IndexMap<String, String>,
    registry: &mut NameRegistry,
) -> Result<Proxy, SubconvError> {
    if proxy_type.is_empty() {
        return Err(SubconvError::Parse("telegram: missing proxy type".into()));
    }

    let server = query
        .get("server")
        .cloned()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| SubconvError::Parse("telegram: missing server".into()))?;

    let port_value = query
        .get("port")
        .cloned()
        .filter(|v| !v.is_empty())
        .ok_or_else(|| SubconvError::Parse("telegram: missing port".into()))?;

    let port: u16 = port_value
        .parse()
        .map_err(|_| SubconvError::Parse("telegram: invalid port".into()))?;

    let remark = query
        .get("remark")
        .or(query.get("remarks"))
        .cloned()
        .unwrap_or_else(|| proxy_type.to_string());

    let name = registry.register(&remark);

    Ok(Proxy::Telegram(TelegramProxy {
        name,
        server,
        port,
        username: query.get("user").cloned().filter(|v| !v.is_empty()),
        password: query.get("pass").cloned().filter(|v| !v.is_empty()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_telegram_tg() {
        let mut reg = NameRegistry::new();
        let uri = "tg://proxy?server=1.2.3.4&port=1080&user=secret&pass=secret2#TestTG";
        let result = parse_telegram(uri, &mut reg).unwrap();
        if let Proxy::Telegram(p) = result {
            assert_eq!(p.server, "1.2.3.4");
            assert_eq!(p.port, 1080);
            assert_eq!(p.username, Some("secret".to_string()));
            assert_eq!(p.password, Some("secret2".to_string()));
        } else {
            panic!("Expected Telegram proxy");
        }
    }

    #[test]
    fn test_parse_telegram_https() {
        let mut reg = NameRegistry::new();
        let uri = "https://t.me/proxy?server=1.2.3.4&port=1080&user=secret&pass=secret2";
        let result = parse_telegram_https(uri, &mut reg).unwrap();
        if let Proxy::Telegram(p) = result {
            assert_eq!(p.server, "1.2.3.4");
            assert_eq!(p.port, 1080);
        } else {
            panic!("Expected Telegram proxy");
        }
    }

    #[test]
    fn test_parse_telegram_missing_server() {
        let mut reg = NameRegistry::new();
        let uri = "tg://proxy?port=1080";
        let result = parse_telegram(uri, &mut reg);
        assert!(result.is_err());
    }
}
