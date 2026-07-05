//! Trojan share link parser.
//!
//! Format: `trojan://password@host:port?sni=xxx&alpn=xxx&type=ws&path=xxx#name`

use indexmap::IndexMap;
use url::Url;

use super::models::{GrpcOpts, WsOpts};
use super::proxy::{Proxy, TrojanProxy};
use super::registry::NameRegistry;
use super::util::{parse_bool, rand_user_agent, url_decode};
use crate::SubconvError;

/// Parse a Trojan share link into a Proxy.
pub fn parse_trojan(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("trojan: {e}")))?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("trojan: missing host".into()))?;
    let port = url
        .port()
        .ok_or_else(|| SubconvError::Parse("trojan: missing port".into()))?;
    let password = url.username().to_string();
    if password.is_empty() {
        return Err(SubconvError::Parse("trojan: missing password".into()));
    }

    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();
    let name = registry.register(&url_decode(url.fragment().unwrap_or("")));

    let skip_cert_verify = query.get("allowInsecure").map(|val| parse_bool(val));

    let sni = query.get("sni").cloned();

    let alpn = query
        .get("alpn")
        .map(|v| v.split(',').map(|s| s.to_string()).collect());

    let network = query
        .get("type")
        .map(|v| v.to_lowercase())
        .filter(|v| !v.is_empty());

    let ws_opts = if network.as_deref() == Some("ws") {
        let mut headers = IndexMap::new();
        headers.insert("User-Agent".to_string(), rand_user_agent().to_string());
        Some(WsOpts {
            path: query.get("path").cloned(),
            headers: Some(headers),
            max_early_data: None,
            early_data_header_name: None,
            v2ray_http_upgrade: None,
            v2ray_http_upgrade_fast_open: None,
        })
    } else {
        None
    };

    let grpc_opts = if network.as_deref() == Some("grpc") {
        Some(GrpcOpts {
            grpc_service_name: query.get("serviceName").cloned(),
            grpc_user_agent: None,
        })
    } else {
        None
    };

    let fp = query.get("fp").map(|v| v.as_str()).unwrap_or("");
    let client_fingerprint = if fp.is_empty() {
        Some("chrome".to_string())
    } else {
        Some(fp.to_string())
    };

    let fingerprint = query.get("pcs").cloned();

    Ok(Proxy::Trojan(TrojanProxy {
        name,
        server: server.to_string(),
        port,
        password,
        udp: true,
        skip_cert_verify,
        sni,
        alpn,
        network,
        grpc_opts,
        ws_opts,
        client_fingerprint,
        fingerprint,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_trojan_basic() {
        let mut reg = NameRegistry::new();
        let uri = "trojan://password123@server:443?sni=example.com&type=ws&path=/ws#TestTrojan";
        let result = parse_trojan(uri, &mut reg).unwrap();
        if let Proxy::Trojan(p) = result {
            assert_eq!(p.password, "password123");
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 443);
            assert_eq!(p.name, "TestTrojan");
            assert_eq!(p.sni, Some("example.com".to_string()));
            assert_eq!(p.network, Some("ws".to_string()));
            assert!(p.ws_opts.is_some());
        } else {
            panic!("Expected Trojan proxy");
        }
    }

    #[test]
    fn test_parse_trojan_missing_host() {
        let mut reg = NameRegistry::new();
        let result = parse_trojan("trojan://pass@:443", &mut reg);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_trojan_grpc() {
        let mut reg = NameRegistry::new();
        let uri = "trojan://pass@server:443?type=grpc&serviceName=myservice#GrpcTrojan";
        let result = parse_trojan(uri, &mut reg).unwrap();
        if let Proxy::Trojan(p) = result {
            assert_eq!(p.network, Some("grpc".to_string()));
            assert!(p.grpc_opts.is_some());
            let grpc = p.grpc_opts.unwrap();
            assert_eq!(grpc.grpc_service_name, Some("myservice".to_string()));
        } else {
            panic!("Expected Trojan proxy");
        }
    }
}
