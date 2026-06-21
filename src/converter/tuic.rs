//! TUIC share link parser.
//!
//! Format: `tuic://uuid:password@host:port?sni=xxx#name` (v5)
//!     or: `tuic://token@host:port?sni=xxx#name` (v4)

use indexmap::IndexMap;
use url::Url;

use super::proxy::{Proxy, TuicProxy};
use super::registry::NameRegistry;
use super::util::url_decode;
use crate::SubconvError;

/// Parse a TUIC share link into a Proxy.
pub fn parse_tuic(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("tuic: {e}")))?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("tuic: missing host".into()))?;
    let port = url
        .port()
        .ok_or_else(|| SubconvError::Parse("tuic: missing port".into()))?;

    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();
    let name = registry.register(&url_decode(url.fragment().unwrap_or("")));

    // v5: uuid:password in userinfo; v4: just token
    let (uuid, password, token) = if url.password().is_some() {
        (Some(url.username().to_string()), url.password().map(|p| p.to_string()), None)
    } else {
        let username = url.username().to_string();
        if username.is_empty() {
            (None, None, None)
        } else {
            (None, None, Some(username))
        }
    };

    let alpn = query
        .get("alpn")
        .map(|v| v.split(',').map(|s| s.to_string()).collect());

    Ok(Proxy::Tuic(TuicProxy {
        name,
        server: server.to_string(),
        port,
        udp: true,
        uuid,
        password,
        token,
        congestion_controller: query
            .get("congestion_control")
            .cloned()
            .filter(|v| !v.is_empty()),
        alpn,
        sni: query.get("sni").cloned().filter(|v| !v.is_empty()),
        disable_sni: if query.get("disable_sni").map(|v| v.as_str()) == Some("1") {
            Some(true)
        } else {
            None
        },
        udp_relay_mode: query
            .get("udp_relay_mode")
            .cloned()
            .filter(|v| !v.is_empty()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_tuic_v5() {
        let mut reg = NameRegistry::new();
        let uri = "tuic://my-uuid:my-password@server:8443?sni=example.com&congestion_control=cubic#TestTuic";
        let result = parse_tuic(uri, &mut reg).unwrap();
        if let Proxy::Tuic(p) = result {
            assert_eq!(p.uuid, Some("my-uuid".to_string()));
            assert_eq!(p.password, Some("my-password".to_string()));
            assert_eq!(p.token, None);
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 8443);
            assert_eq!(p.name, "TestTuic");
            assert_eq!(p.sni, Some("example.com".to_string()));
            assert_eq!(p.congestion_controller, Some("cubic".to_string()));
        } else {
            panic!("Expected Tuic proxy");
        }
    }

    #[test]
    fn test_parse_tuic_v4() {
        let mut reg = NameRegistry::new();
        let uri = "tuic://my-token@server:8443?sni=example.com#TuicV4";
        let result = parse_tuic(uri, &mut reg).unwrap();
        if let Proxy::Tuic(p) = result {
            assert_eq!(p.uuid, None);
            assert_eq!(p.password, None);
            assert_eq!(p.token, Some("my-token".to_string()));
        } else {
            panic!("Expected Tuic proxy");
        }
    }

    #[test]
    fn test_parse_tuic_missing_port() {
        let mut reg = NameRegistry::new();
        let result = parse_tuic("tuic://token@server", &mut reg);
        assert!(result.is_err());
    }
}
