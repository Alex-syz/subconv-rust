//! Hysteria share link parser.
//!
//! Format: `hysteria://host:port?peer=xxx&auth=xxx&upmbps=xxx#name`

use indexmap::IndexMap;
use url::Url;

use super::proxy::{HysteriaProxy, Proxy};
use super::registry::NameRegistry;
use super::util::{parse_bool, url_decode};
use crate::SubconvError;

/// Parse a Hysteria share link into a Proxy.
pub fn parse_hysteria(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("hysteria: {e}")))?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("hysteria: missing host".into()))?;
    let port = url
        .port()
        .ok_or_else(|| SubconvError::Parse("hysteria: missing port".into()))?;

    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();
    let name = registry.register(&url_decode(url.fragment().unwrap_or("")));

    let up = query
        .get("up")
        .or(query.get("upmbps"))
        .cloned()
        .filter(|v| !v.is_empty());
    let down = query
        .get("down")
        .or(query.get("downmbps"))
        .cloned()
        .filter(|v| !v.is_empty());

    let alpn = query
        .get("alpn")
        .map(|v| v.split(',').map(|s| s.to_string()).collect());

    let skip_cert_verify = query.get("insecure").map(|val| parse_bool(val));

    Ok(Proxy::Hysteria(HysteriaProxy {
        name,
        server: server.to_string(),
        port,
        sni: query.get("peer").cloned().filter(|v| !v.is_empty()),
        obfs: query.get("obfs").cloned().filter(|v| !v.is_empty()),
        alpn,
        auth_str: query.get("auth").cloned().filter(|v| !v.is_empty()),
        protocol: query.get("protocol").cloned().filter(|v| !v.is_empty()),
        up,
        down,
        skip_cert_verify,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hysteria_basic() {
        let mut reg = NameRegistry::new();
        let uri = "hysteria://server:443?peer=sni.example.com&auth=myauth&upmbps=100&downmbps=200&obfs=salamander#TestHysteria";
        let result = parse_hysteria(uri, &mut reg).unwrap();
        if let Proxy::Hysteria(p) = result {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 443);
            assert_eq!(p.name, "TestHysteria");
            assert_eq!(p.sni, Some("sni.example.com".to_string()));
            assert_eq!(p.auth_str, Some("myauth".to_string()));
            assert_eq!(p.up, Some("100".to_string()));
            assert_eq!(p.down, Some("200".to_string()));
            assert_eq!(p.obfs, Some("salamander".to_string()));
        } else {
            panic!("Expected Hysteria proxy");
        }
    }

    #[test]
    fn test_parse_hysteria_missing_port() {
        let mut reg = NameRegistry::new();
        let result = parse_hysteria("hysteria://server", &mut reg);
        assert!(result.is_err());
    }
}
