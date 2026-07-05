//! VMess share link parser.
//!
//! Two formats:
//! - Legacy (V2): `vmess://base64(json)`
//! - AEAD (V2Ray 5+): `vmess://uuid@host:port?encryption=auto&type=ws#name`

use indexmap::IndexMap;
use url::Url;

use super::models::*;
use super::proxy::{Proxy, VmessProxy};
use super::registry::NameRegistry;
use super::util::{base64_decode_auto, query_first, query_pairs_multi};
use super::vless::handle_v_share_link;
use crate::SubconvError;

/// Parse a VMess share link into a Proxy.
pub fn parse_vmess(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let body = uri
        .strip_prefix("vmess://")
        .ok_or_else(|| SubconvError::Parse("vmess: invalid scheme".into()))?;

    // Try legacy format first (base64-encoded JSON)
    match parse_legacy(body, registry) {
        Ok(proxy) => return Ok(proxy),
        Err(SubconvError::Parse(msg)) if msg.contains("legacy base64 decode failed") => {
            // Fall through to AEAD format
        }
        Err(e) => return Err(e),
    }

    // AEAD format: standard URL with query parameters
    parse_aead(uri, registry)
}

/// Parse legacy VMess format (base64-encoded JSON).
fn parse_legacy(body: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let decoded = base64_decode_auto(body).map_err(|_| {
        SubconvError::Parse("vmess: legacy base64 decode failed".into())
    })?;

    let values: serde_json::Value =
        serde_json::from_str(&decoded).map_err(|_| SubconvError::Parse("vmess: invalid legacy json".into()))?;

    let obj = values
        .as_object()
        .ok_or_else(|| SubconvError::Parse("vmess: invalid legacy json".into()))?;

    let server = require_str(obj, "add", "vmess: missing server")?;
    let uuid = require_str(obj, "id", "vmess: missing uuid")?;
    let port = parse_port(obj.get("port"))?;

    let name = registry.register(
        obj.get("ps")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );

    let alter_id = parse_int(obj.get("aid"), 0);
    let cipher = obj
        .get("scy")
        .and_then(|v| v.as_str())
        .unwrap_or("auto")
        .to_string();

    let servername = optional_str(obj.get("sni"));

    let tls_value = obj
        .get("tls")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    let tls = tls_value.ends_with("tls");

    let alpn = if tls {
        split_csv(obj.get("alpn"))
    } else {
        None
    };

    let mut network = obj
        .get("net")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let type_val = obj
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();
    if type_val == "http" {
        network = "http".to_string();
    } else if network == "http" {
        network = "h2".to_string();
    }

    let network_opt = if network.is_empty() {
        None
    } else {
        Some(network.clone())
    };

    let host = optional_str(obj.get("host"));
    let path = optional_str(obj.get("path"));

    let http_opts = if network == "http" {
        let mut headers = IndexMap::new();
        if let Some(ref h) = host {
            headers.insert("Host".to_string(), vec![h.clone()]);
        }
        Some(HttpOpts {
            method: None,
            path: Some(vec![path.clone().unwrap_or_else(|| "/".to_string())]),
            headers: if headers.is_empty() { None } else { Some(headers) },
        })
    } else {
        None
    };

    let h2_opts = if network == "h2" {
        let mut headers = IndexMap::new();
        if let Some(ref h) = host {
            headers.insert("Host".to_string(), vec![h.clone()]);
        }
        Some(H2Opts {
            host: None,
            path: path.clone().map(StringOrList::One),
            headers: if headers.is_empty() { None } else { Some(headers) },
        })
    } else {
        None
    };

    let ws_opts = if network == "ws" || network == "httpupgrade" {
        Some(build_ws_opts(path.as_deref(), host.as_deref(), &network))
    } else {
        None
    };

    let grpc_opts = if network == "grpc" {
        Some(GrpcOpts {
            grpc_service_name: path.clone(),
            grpc_user_agent: None,
        })
    } else {
        None
    };

    Ok(Proxy::Vmess(Box::new(VmessProxy {
        name,
        server,
        port,
        uuid,
        alter_id: alter_id as u32,
        cipher,
        udp: true,
        network: network_opt,
        tls,
        alpn,
        skip_cert_verify: false,
        fingerprint: None,
        certificate: None,
        private_key: None,
        servername,
        ech_opts: None,
        reality_opts: None,
        http_opts,
        h2_opts,
        grpc_opts,
        ws_opts,
        packet_addr: None,
        xudp: true,
        packet_encoding: None,
        global_padding: None,
        authenticated_length: None,
        client_fingerprint: None,
    })))
}

/// Parse AEAD VMess format (standard URL with query parameters).
fn parse_aead(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("vmess aead: {e}")))?;

    if url.host_str().is_none() || url.port().is_none() {
        return Err(SubconvError::Parse(
            "vmess: missing host or port in aead format".into(),
        ));
    }

    let query = query_pairs_multi(&url);

    let shared = handle_v_share_link(&url, registry, "vless", &query)?;

    let encryption = query_first(&query, "encryption");
    let encryption = if encryption.is_empty() {
        "auto".to_string()
    } else {
        encryption
    };

    Ok(Proxy::Vmess(Box::new(VmessProxy {
        name: shared.name,
        server: shared.server,
        port: shared.port,
        uuid: shared.uuid,
        alter_id: 0,
        cipher: encryption,
        udp: true,
        network: shared.network,
        tls: shared.tls.unwrap_or(false),
        alpn: shared.alpn,
        skip_cert_verify: shared.skip_cert_verify.unwrap_or(false),
        fingerprint: shared.fingerprint,
        certificate: None,
        private_key: None,
        servername: shared.servername,
        ech_opts: None,
        reality_opts: shared.reality_opts,
        http_opts: shared.http_opts,
        h2_opts: shared.h2_opts,
        grpc_opts: shared.grpc_opts,
        ws_opts: shared.ws_opts,
        packet_addr: shared.packet_addr,
        xudp: true,
        packet_encoding: None,
        global_padding: None,
        authenticated_length: None,
        client_fingerprint: shared.client_fingerprint,
    })))
}

/// Build WsOpts from path, host, and network type.
fn build_ws_opts(path: Option<&str>, host: Option<&str>, network: &str) -> WsOpts {
    let ws_path = path.unwrap_or("/");
    let mut headers = IndexMap::new();
    if let Some(h) = host {
        headers.insert("Host".to_string(), h.to_string());
    }

    let mut max_early_data: Option<u32> = None;
    let mut early_data_header_name: Option<String> = None;
    let mut v2ray_http_upgrade_fast_open: Option<bool> = None;
    let mut final_path = ws_path.to_string();

    if let Some(p) = path {
        if let Ok(parsed) = Url::parse(&format!("http://dummy{p}")) {
            let mut query_pairs: Vec<(String, String)> = parsed
                .query_pairs()
                .map(|(k, v)| (k.into_owned(), v.into_owned()))
                .collect();

            if let Some(ed_pos) = query_pairs.iter().position(|(k, _)| k == "ed") {
                let ed_value = query_pairs[ed_pos].1.clone();
                if let Ok(ed_int) = ed_value.parse::<u32>() {
                    if network == "ws" {
                        max_early_data = Some(ed_int);
                        early_data_header_name = Some("Sec-WebSocket-Protocol".to_string());
                    } else {
                        v2ray_http_upgrade_fast_open = Some(true);
                    }
                }
                query_pairs.remove(ed_pos);

                if query_pairs.is_empty() {
                    final_path = parsed.path().to_string();
                } else {
                    let qs: String = query_pairs
                        .iter()
                        .map(|(k, v)| format!("{k}={v}"))
                        .collect::<Vec<_>>()
                        .join("&");
                    final_path = format!("{}?{qs}", parsed.path());
                }
            }

            if let Some(eh_pos) = query_pairs.iter().position(|(k, _)| k == "eh") {
                early_data_header_name = Some(query_pairs[eh_pos].1.clone());
            }
        }
    }

    WsOpts {
        path: Some(final_path),
        headers: if headers.is_empty() { None } else { Some(headers) },
        max_early_data,
        early_data_header_name,
        v2ray_http_upgrade: None,
        v2ray_http_upgrade_fast_open,
    }
}


fn require_str(
    obj: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    msg: &str,
) -> Result<String, SubconvError> {
    optional_str(obj.get(key)).ok_or_else(|| SubconvError::Parse(msg.into()))
}

fn optional_str(value: Option<&serde_json::Value>) -> Option<String> {
    value.and_then(|v| v.as_str()).filter(|s| !s.is_empty()).map(|s| s.to_string())
}

fn parse_port(value: Option<&serde_json::Value>) -> Result<u16, SubconvError> {
    let port = parse_int(value, -1);
    if port <= 0 || port > u16::MAX as i64 {
        return Err(SubconvError::Parse("vmess: invalid port".into()));
    }
    Ok(port as u16)
}

fn parse_int(value: Option<&serde_json::Value>, default: i64) -> i64 {
    value
        .and_then(|v| {
            v.as_i64()
                .or_else(|| v.as_str().and_then(|s| s.parse::<i64>().ok()))
        })
        .unwrap_or(default)
}

fn split_csv(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    optional_str(value).map(|text| {
        text.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use super::*;

    #[test]
    fn test_parse_vmess_legacy() {
        let mut reg = NameRegistry::new();
        let json = r#"{"v":"2","ps":"TestVMess","add":"server","port":"443","id":"12345678-1234-1234-1234-123456789abc","aid":"0","scy":"auto","net":"ws","type":"none","host":"","path":"/ws","tls":"tls"}"#;
        let encoded = base64::engine::general_purpose::STANDARD.encode(json);
        let uri = format!("vmess://{encoded}");
        let result = parse_vmess(&uri, &mut reg).unwrap();
        if let Proxy::Vmess(p) = result {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 443);
            assert_eq!(p.uuid, "12345678-1234-1234-1234-123456789abc");
            assert_eq!(p.name, "TestVMess");
            assert!(p.tls);
            assert_eq!(p.network, Some("ws".to_string()));
        } else {
            panic!("Expected Vmess proxy");
        }
    }

    #[test]
    fn test_parse_vmess_invalid() {
        let mut reg = NameRegistry::new();
        let result = parse_vmess("vmess://not-base64-at-all!!!", &mut reg);
        assert!(result.is_err());
    }
}
