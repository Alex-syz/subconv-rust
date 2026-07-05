//! AnyTLS share link parser.
//!
//! Format: `anytls://username:password@host:port?sni=xxx&hpkp=xxx&insecure=1#name`

use indexmap::IndexMap;
use url::Url;

use super::proxy::{AnytlsProxy, Proxy};
use super::registry::NameRegistry;
use super::util::url_decode;
use crate::SubconvError;

/// Parse an AnyTLS share link into a Proxy.
pub fn parse_anytls(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("anytls: {e}")))?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("anytls: missing host".into()))?;
    let port = url
        .port()
        .ok_or_else(|| SubconvError::Parse("anytls: missing port".into()))?;

    let username = if !url.username().is_empty() {
        Some(url_decode(url.username()))
    } else {
        None
    };

    let password = url
        .password()
        .map(url_decode)
        .or_else(|| username.clone());

    let password = password
        .filter(|v| !v.is_empty())
        .ok_or_else(|| SubconvError::Parse("anytls: missing password".into()))?;

    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();

    let fragment = url.fragment().unwrap_or("");
    let name = if fragment.is_empty() {
        registry.register(&format!("{server}:{port}"))
    } else {
        registry.register(&url_decode(fragment))
    };

    Ok(Proxy::Anytls(AnytlsProxy {
        name,
        server: server.to_string(),
        port,
        username,
        password,
        sni: query.get("sni").cloned().filter(|v| !v.is_empty()),
        fingerprint: query.get("hpkp").cloned().filter(|v| !v.is_empty()),
        skip_cert_verify: query.get("insecure").map(|v| v == "1").unwrap_or(false),
        udp: true,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_anytls_basic() {
        let mut reg = NameRegistry::new();
        let uri = "anytls://user:pass@server:443?sni=example.com&hpkp=sha256/abc&insecure=1#TestAnyTLS";
        let result = parse_anytls(uri, &mut reg).unwrap();
        if let Proxy::Anytls(p) = result {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 443);
            assert_eq!(p.name, "TestAnyTLS");
            assert_eq!(p.username, Some("user".to_string()));
            assert_eq!(p.password, "pass");
            assert_eq!(p.sni, Some("example.com".to_string()));
            assert_eq!(p.fingerprint, Some("sha256/abc".to_string()));
            assert!(p.skip_cert_verify);
        } else {
            panic!("Expected Anytls proxy");
        }
    }

    #[test]
    fn test_parse_anytls_password_fallback() {
        let mut reg = NameRegistry::new();
        let uri = "anytls://onlyuser@server:443#Fallback";
        let result = parse_anytls(uri, &mut reg).unwrap();
        if let Proxy::Anytls(p) = result {
            assert_eq!(p.username, Some("onlyuser".to_string()));
            assert_eq!(p.password, "onlyuser");
        } else {
            panic!("Expected Anytls proxy");
        }
    }

    #[test]
    fn test_parse_anytls_missing_host() {
        let mut reg = NameRegistry::new();
        let result = parse_anytls("anytls://pass@:443", &mut reg);
        assert!(result.is_err());
    }
}
