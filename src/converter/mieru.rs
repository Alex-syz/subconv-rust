//! Mieru share link parser.
//!
//! Format: `mierus://username:password@host?port=xxx&protocol=xxx#name`
//! Can produce multiple proxies when multiple port/protocol pairs are given.

use url::Url;

use super::proxy::{MieruProxy, Proxy};
use super::registry::NameRegistry;
use super::util::{query_first, query_pairs_multi, url_decode};
use crate::SubconvError;

/// Parse a Mieru share link into one or more Proxies.
pub fn parse_mieru(uri: &str, registry: &mut NameRegistry) -> Result<Vec<Proxy>, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("mieru: {e}")))?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("mieru: missing host".into()))?;

    let query = query_pairs_multi(&url);

    let ports = query.get("port").cloned().unwrap_or_default();
    let protocols = query.get("protocol").cloned().unwrap_or_default();

    if ports.len() != protocols.len() || ports.is_empty() {
        return Err(SubconvError::Parse("mieru: mismatched port/protocol".into()));
    }

    let username = if !url.username().is_empty() {
        Some(url_decode(url.username()))
    } else {
        None
    };

    let password = url.password().map(url_decode);

    let base_name = if let Some(f) = url.fragment() {
        if !f.is_empty() {
            url_decode(f)
        } else {
            let p = query_first(&query, "profile");
            if p.is_empty() { server.to_string() } else { p }
        }
    } else {
        let p = query_first(&query, "profile");
        if p.is_empty() { server.to_string() } else { p }
    };

    let mut proxies = Vec::new();

    for (port_value, protocol) in ports.iter().zip(protocols.iter()) {
        let proxy_name = registry.register(&format!("{base_name}:{port_value}/{protocol}"));

        let (port, port_range) = if port_value.contains('-') {
            (None, Some(port_value.clone()))
        } else {
            match port_value.parse::<u16>() {
                Ok(p) => (Some(p), None),
                Err(_) => continue,
            }
        };

        let multiplexing = query_first(&query, "multiplexing");
        let multiplexing = if multiplexing.is_empty() { None } else { Some(multiplexing) };
        let handshake_mode = query_first(&query, "handshake-mode");
        let handshake_mode = if handshake_mode.is_empty() { None } else { Some(handshake_mode) };
        let traffic_pattern = query_first(&query, "traffic-pattern");
        let traffic_pattern = if traffic_pattern.is_empty() { None } else { Some(traffic_pattern) };

        proxies.push(Proxy::Mieru(MieruProxy {
            name: proxy_name,
            server: server.to_string(),
            transport: protocol.clone(),
            udp: true,
            username: username.clone(),
            password: password.clone(),
            port,
            port_range,
            multiplexing,
            handshake_mode,
            traffic_pattern,
        }));
    }

    Ok(proxies)
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_mieru_basic() {
        let mut reg = NameRegistry::new();
        let uri = "mierus://user:pass@server?port=8964&protocol=tcp#TestMieru";
        let result = parse_mieru(uri, &mut reg).unwrap();
        assert_eq!(result.len(), 1);
        if let Proxy::Mieru(p) = &result[0] {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, Some(8964));
            assert_eq!(p.transport, "tcp");
            assert_eq!(p.name, "TestMieru:8964/tcp");
            assert_eq!(p.username, Some("user".to_string()));
            assert_eq!(p.password, Some("pass".to_string()));
        } else {
            panic!("Expected Mieru proxy");
        }
    }

    #[test]
    fn test_parse_mieru_multiple_ports() {
        let mut reg = NameRegistry::new();
        let uri = "mierus://server?port=8964&port=8965&protocol=tcp&protocol=udp#MultiPort";
        let result = parse_mieru(uri, &mut reg).unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_mieru_port_range() {
        let mut reg = NameRegistry::new();
        let uri = "mierus://server?port=8964-8970&protocol=tcp#PortRange";
        let result = parse_mieru(uri, &mut reg).unwrap();
        if let Proxy::Mieru(p) = &result[0] {
            assert_eq!(p.port, None);
            assert_eq!(p.port_range, Some("8964-8970".to_string()));
        } else {
            panic!("Expected Mieru proxy");
        }
    }

    #[test]
    fn test_parse_mieru_mismatched() {
        let mut reg = NameRegistry::new();
        let uri = "mierus://server?port=8964&protocol=tcp&protocol=udp#Mismatch";
        let result = parse_mieru(uri, &mut reg);
        assert!(result.is_err());
    }
}
