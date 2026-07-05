//! Hysteria2 share link parser.
//!
//! Formats: `hysteria2://auth@host:port?sni=xxx#name` or `hy2://...`

use indexmap::IndexMap;
use url::Url;

use super::proxy::{Hysteria2Proxy, Proxy};
use super::registry::NameRegistry;
use super::util::{parse_bool, url_decode};
use crate::SubconvError;

/// Parse a Hysteria2 share link into a Proxy.
pub fn parse_hysteria2(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("hysteria2: {e}")))?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("hysteria2: missing host".into()))?;
    let port = url.port().unwrap_or(443);

    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();
    let name = registry.register(&url_decode(url.fragment().unwrap_or("")));

    let password = if !url.username().is_empty() {
        Some(url.username().to_string())
    } else {
        None
    };

    let alpn = query
        .get("alpn")
        .map(|v| v.split(',').map(|s| s.to_string()).collect());

    let skip_cert_verify = query.get("insecure").map(|val| parse_bool(val));

    Ok(Proxy::Hysteria2(Hysteria2Proxy {
        name,
        server: server.to_string(),
        port,
        password,
        obfs: query.get("obfs").cloned().filter(|v| !v.is_empty()),
        obfs_password: query
            .get("obfs-password")
            .cloned()
            .filter(|v| !v.is_empty()),
        sni: query.get("sni").cloned().filter(|v| !v.is_empty()),
        skip_cert_verify,
        alpn,
        fingerprint: query
            .get("pinSHA256")
            .cloned()
            .filter(|v| !v.is_empty()),
        down: query.get("down").cloned().filter(|v| !v.is_empty()),
        up: query.get("up").cloned().filter(|v| !v.is_empty()),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hysteria2_basic() {
        let mut reg = NameRegistry::new();
        let uri = "hysteria2://authpassword@server:443?sni=example.com&obfs=salamander&obfs-password=obfspass#TestHy2";
        let result = parse_hysteria2(uri, &mut reg).unwrap();
        if let Proxy::Hysteria2(p) = result {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 443);
            assert_eq!(p.name, "TestHy2");
            assert_eq!(p.password, Some("authpassword".to_string()));
            assert_eq!(p.sni, Some("example.com".to_string()));
            assert_eq!(p.obfs, Some("salamander".to_string()));
            assert_eq!(p.obfs_password, Some("obfspass".to_string()));
        } else {
            panic!("Expected Hysteria2 proxy");
        }
    }

    #[test]
    fn test_parse_hysteria2_default_port() {
        let mut reg = NameRegistry::new();
        let uri = "hysteria2://auth@server?sni=example.com#NoPort";
        let result = parse_hysteria2(uri, &mut reg).unwrap();
        if let Proxy::Hysteria2(p) = result {
            assert_eq!(p.port, 443);
        } else {
            panic!("Expected Hysteria2 proxy");
        }
    }
}
