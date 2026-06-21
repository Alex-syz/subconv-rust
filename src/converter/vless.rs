//! VLESS share link parser.
//!
//! Format: `vless://uuid@host:port?security=tls&type=ws&path=xxx#name`
//!
//! Also exports `handle_v_share_link` which is shared with the VMess parser.

use indexmap::IndexMap;
use url::Url;

use super::models::*;
use super::proxy::{Proxy, VlessProxy};
use super::registry::NameRegistry;
use super::util::{base64_decode_auto, query_first, query_pairs_multi, rand_user_agent, url_decode};
use crate::SubconvError;

/// Shared state extracted by `handle_v_share_link`.
/// Used by both VLESS and VMess parsers to avoid duplicating transport logic.
pub struct VShareLinkResult {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub uuid: String,
    pub tls: Option<bool>,
    pub client_fingerprint: Option<String>,
    pub alpn: Option<Vec<String>>,
    pub fingerprint: Option<String>,
    pub servername: Option<String>,
    pub reality_opts: Option<RealityOpts>,
    pub packet_addr: Option<bool>,
    pub xudp: Option<bool>,
    pub network: Option<String>,
    pub http_opts: Option<HttpOpts>,
    pub h2_opts: Option<H2Opts>,
    pub ws_opts: Option<WsOpts>,
    pub grpc_opts: Option<GrpcOpts>,
    pub xhttp_opts: Option<XhttpOpts>,
    pub skip_cert_verify: Option<bool>,
}

/// Parse a VLESS share link into a Proxy.
pub fn parse_vless(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = decode_vless_host(uri)?;
    let query = query_pairs_multi(&url);

    let shared = handle_v_share_link(&url, registry, "vless", &query)?;

    let flow = query_first(&query, "flow");
    let flow = if flow.is_empty() {
        None
    } else {
        Some(flow.to_lowercase())
    };

    let encryption = query_first(&query, "encryption");
    let encryption = if encryption.is_empty() {
        None
    } else {
        Some(encryption)
    };

    Ok(Proxy::Vless(Box::new(VlessProxy {
        name: shared.name,
        server: shared.server,
        port: shared.port,
        uuid: shared.uuid,
        flow,
        tls: shared.tls,
        alpn: shared.alpn,
        udp: true,
        packet_addr: shared.packet_addr,
        xudp: shared.xudp,
        packet_encoding: None,
        encryption,
        network: shared.network,
        ech_opts: None,
        reality_opts: shared.reality_opts,
        http_opts: shared.http_opts,
        h2_opts: shared.h2_opts,
        grpc_opts: shared.grpc_opts,
        ws_opts: shared.ws_opts,
        xhttp_opts: shared.xhttp_opts,
        ws_headers: None,
        skip_cert_verify: shared.skip_cert_verify,
        fingerprint: shared.fingerprint,
        certificate: None,
        private_key: None,
        servername: shared.servername,
        client_fingerprint: shared.client_fingerprint,
    })))
}

/// Shared logic for VLESS and VMess share links.
pub fn handle_v_share_link(
    url: &Url,
    registry: &mut NameRegistry,
    _scheme: &str,
    query: &IndexMap<String, Vec<String>>,
) -> Result<VShareLinkResult, SubconvError> {
    let hostname = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("v share link: missing host".into()))?;
    let port = url
        .port()
        .ok_or_else(|| SubconvError::Parse("v share link: missing port".into()))?;
    let uuid = url.username().to_string();
    let name = registry.register(&url_decode(url.fragment().unwrap_or("")));

    let security = query_first(query, "security").to_lowercase();
    let mut tls = None;
    let mut client_fingerprint: Option<String> = None;
    let mut alpn: Option<Vec<String>> = None;
    let mut fingerprint: Option<String> = None;
    let mut servername: Option<String> = None;
    let mut reality_opts: Option<RealityOpts> = None;
    let mut packet_addr: Option<bool> = None;
    let mut xudp: Option<bool> = None;
    let mut skip_cert_verify: Option<bool> = None;

    if security.ends_with("tls") || security == "reality" {
        tls = Some(true);
        let fp = query_first(query, "fp");
        client_fingerprint = if fp.is_empty() {
            Some("chrome".to_string())
        } else {
            Some(fp)
        };

        let alpn_str = query_first(query, "alpn");
        if !alpn_str.is_empty() {
            alpn = Some(alpn_str.split(',').map(|s| s.to_string()).collect());
        }

        let pcs = query_first(query, "pcs");
        if !pcs.is_empty() {
            fingerprint = Some(pcs);
        }

        if security == "reality" {
            skip_cert_verify = Some(false);
        }
    }

    let sni = query_first(query, "sni");
    if !sni.is_empty() {
        servername = Some(sni);
    }

    let pbk = query_first(query, "pbk");
    if !pbk.is_empty() {
        let sid = query_first(query, "sid");
        reality_opts = Some(RealityOpts {
            public_key: pbk,
            short_id: if sid.is_empty() { None } else { Some(sid) },
        });
    }

    let packet_encoding = query_first(query, "packetEncoding");
    if packet_encoding == "packet" {
        packet_addr = Some(true);
    } else if packet_encoding != "none" {
        xudp = Some(true);
    }

    let mut network = query_first(query, "type").to_lowercase();
    if network.is_empty() {
        network = "tcp".to_string();
    }

    let fake_type = query_first(query, "headerType").to_lowercase();
    if network == "tcp" && fake_type == "http" {
        network = "http".to_string();
    } else if network == "http" {
        network = "h2".to_string();
    }

    let network_opt = if network == "tcp" {
        None
    } else {
        Some(network.clone())
    };

    let http_opts = if network == "http" {
        let mut headers = IndexMap::new();
        let host = query_first(query, "host");
        if !host.is_empty() {
            headers.insert("Host".to_string(), vec![host]);
        }
        let method = query_first(query, "method");
        let path_str = query_first(query, "path");
        Some(HttpOpts {
            method: if method.is_empty() { None } else { Some(method) },
            path: Some(vec![if path_str.is_empty() { "/".to_string() } else { path_str }]),
            headers: if headers.is_empty() { None } else { Some(headers) },
        })
    } else {
        None
    };

    let h2_opts = if network == "h2" {
        let h2_path = query_first(query, "path");
        let h2_host = query_first(query, "host");
        Some(H2Opts {
            host: if h2_host.is_empty() { None } else { Some(vec![h2_host]) },
            path: Some(if h2_path.is_empty() {
                StringOrList::One("/".to_string())
            } else {
                StringOrList::One(h2_path)
            }),
            headers: None,
        })
    } else {
        None
    };

    let ws_opts = if network == "ws" || network == "httpupgrade" {
        let ws_path = query_first(query, "path");
        let ws_host = query_first(query, "host");
        let mut ws_headers = IndexMap::new();
        ws_headers.insert("User-Agent".to_string(), rand_user_agent().to_string());
        if !ws_host.is_empty() {
            ws_headers.insert("Host".to_string(), ws_host);
        }

        let mut max_early_data: Option<u32> = None;
        let mut early_data_header_name: Option<String> = None;
        let mut v2ray_http_upgrade_fast_open: Option<bool> = None;

        let early_data = query_first(query, "ed");
        if !early_data.is_empty() {
            if let Ok(ed_int) = early_data.parse::<u32>() {
                if network == "ws" {
                    max_early_data = Some(ed_int);
                    early_data_header_name = Some("Sec-WebSocket-Protocol".to_string());
                } else {
                    v2ray_http_upgrade_fast_open = Some(true);
                }
            }
        }

        let early_header = query_first(query, "eh");
        if !early_header.is_empty() {
            early_data_header_name = Some(early_header);
        }

        Some(WsOpts {
            path: if ws_path.is_empty() { None } else { Some(ws_path) },
            headers: Some(ws_headers),
            max_early_data,
            early_data_header_name,
            v2ray_http_upgrade: None,
            v2ray_http_upgrade_fast_open,
        })
    } else {
        None
    };

    let grpc_opts = if network == "grpc" {
        let service_name = query_first(query, "serviceName");
        Some(GrpcOpts {
            grpc_service_name: if service_name.is_empty() { None } else { Some(service_name) },
            grpc_user_agent: None,
        })
    } else {
        None
    };

    let xhttp_opts = if network == "xhttp" {
        let xhttp_path = query_first(query, "path");
        let xhttp_host = query_first(query, "host");
        let xhttp_mode = query_first(query, "mode");
        let extra_str = query_first(query, "extra");

        let mut extra_opts: IndexMap<String, serde_json::Value> = IndexMap::new();
        if !extra_str.is_empty() {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(&extra_str) {
                if let Some(obj) = parsed.as_object() {
                    extra_opts = obj
                        .iter()
                        .map(|(k, v)| (k.clone(), v.clone()))
                        .collect();
                }
            }
        }

        Some(parse_xhttp_opts(
            if xhttp_path.is_empty() { None } else { Some(xhttp_path) },
            if xhttp_host.is_empty() { None } else { Some(xhttp_host) },
            if xhttp_mode.is_empty() { None } else { Some(xhttp_mode) },
            &extra_opts,
        ))
    } else {
        None
    };

    Ok(VShareLinkResult {
        name,
        server: hostname.to_string(),
        port,
        uuid,
        tls,
        client_fingerprint,
        alpn,
        fingerprint,
        servername,
        reality_opts,
        packet_addr,
        xudp,
        network: network_opt,
        http_opts,
        h2_opts,
        ws_opts,
        grpc_opts,
        xhttp_opts,
        skip_cert_verify,
    })
}

/// Decode a VLESS URL where the host part might be base64-encoded.
fn decode_vless_host(uri: &str) -> Result<Url, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("vless: {e}")))?;

    let host = match url.host_str() {
        Some(h) => h,
        None => return Err(SubconvError::Parse("vless: missing host".into())),
    };

    if let Ok(decoded_host) = base64_decode_auto(host) {
        let username = url.username();
        let port = url.port().map(|p| format!(":{p}")).unwrap_or_default();
        let query = url.query().map(|q| format!("?{q}")).unwrap_or_default();
        let fragment = url.fragment().map(|f| format!("#{f}")).unwrap_or_default();
        let new_uri = format!("vless://{username}@{decoded_host}{port}{query}{fragment}");
        if let Ok(new_url) = Url::parse(&new_uri) {
            return Ok(new_url);
        }
    }

    Ok(url)
}

/// Build XhttpOpts from parsed parameters.
fn parse_xhttp_opts(
    path: Option<String>,
    host: Option<String>,
    mode: Option<String>,
    extra: &IndexMap<String, serde_json::Value>,
) -> XhttpOpts {
    let no_grpc_header = extra
        .get("noGRPCHeader")
        .and_then(|v| v.as_bool())
        .filter(|&b| b);

    let x_padding_bytes = extra
        .get("xPaddingBytes")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let x_padding_obfs_mode = extra.get("xPaddingObfsMode").and_then(|v| v.as_bool());

    let x_padding_key = str_from_extra(extra, "xPaddingKey");
    let x_padding_header = str_from_extra(extra, "xPaddingHeader");
    let x_padding_placement = str_from_extra(extra, "xPaddingPlacement");
    let x_padding_method = str_from_extra(extra, "xPaddingMethod");
    let uplink_http_method = str_from_extra(extra, "uplinkHttpMethod");
    let session_placement = str_from_extra(extra, "sessionPlacement");
    let session_key = str_from_extra(extra, "sessionKey");
    let seq_placement = str_from_extra(extra, "seqPlacement");
    let seq_key = str_from_extra(extra, "seqKey");
    let uplink_data_placement = str_from_extra(extra, "uplinkDataPlacement");
    let uplink_data_key = str_from_extra(extra, "uplinkDataKey");

    let uplink_chunk_size = int_from_extra(extra, "uplinkChunkSize");
    let sc_max_each_post_bytes = int_from_extra(extra, "scMaxEachPostBytes");
    let sc_min_posts_interval_ms = int_from_extra(extra, "scMinPostsIntervalMs");

    let reuse_settings = extra
        .get("xmux")
        .and_then(|v| v.as_object())
        .map(xmux_to_reuse_settings);

    let download_settings = extra
        .get("downloadSettings")
        .and_then(|v| v.as_object())
        .map(parse_xhttp_download_settings);

    XhttpOpts {
        path,
        host,
        mode,
        headers: None,
        no_grpc_header,
        x_padding_bytes,
        x_padding_obfs_mode,
        x_padding_key,
        x_padding_header,
        x_padding_placement,
        x_padding_method,
        uplink_http_method,
        session_placement,
        session_key,
        seq_placement,
        seq_key,
        uplink_data_placement,
        uplink_data_key,
        uplink_chunk_size,
        sc_max_each_post_bytes,
        sc_min_posts_interval_ms,
        reuse_settings,
        download_settings,
    }
}

fn str_from_extra(extra: &IndexMap<String, serde_json::Value>, key: &str) -> Option<String> {
    extra
        .get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn int_from_extra(extra: &IndexMap<String, serde_json::Value>, key: &str) -> Option<u32> {
    extra.get(key).and_then(|v| {
        v.as_u64()
            .map(|n| n as u32)
            .or_else(|| v.as_f64().map(|f| f as u32))
    })
}

fn xmux_to_reuse_settings(xmux: &serde_json::Map<String, serde_json::Value>) -> XhttpReuseSettings {
    let max_connections = xmux
        .get("maxConnections")
        .and_then(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
        .or_else(|| xmux.get("maxConnections").and_then(|v| v.as_f64().map(|f| (f as u64).to_string())));

    let max_concurrency = xmux
        .get("maxConcurrency")
        .and_then(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
        .or_else(|| xmux.get("maxConcurrency").and_then(|v| v.as_f64().map(|f| (f as u64).to_string())));

    let c_max_reuse_times = xmux
        .get("cMaxReuseTimes")
        .and_then(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
        .or_else(|| xmux.get("cMaxReuseTimes").and_then(|v| v.as_f64().map(|f| (f as u64).to_string())));

    let h_max_request_times = xmux
        .get("hMaxRequestTimes")
        .and_then(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
        .or_else(|| xmux.get("hMaxRequestTimes").and_then(|v| v.as_f64().map(|f| (f as u64).to_string())));

    let h_max_reusable_secs = xmux
        .get("hMaxReusableSecs")
        .and_then(|v| v.as_str().filter(|s| !s.is_empty()).map(|s| s.to_string()))
        .or_else(|| xmux.get("hMaxReusableSecs").and_then(|v| v.as_f64().map(|f| (f as u64).to_string())));

    let h_keep_alive_period = xmux
        .get("hKeepAlivePeriod")
        .and_then(|v| v.as_u64().map(|n| n as u32));

    XhttpReuseSettings {
        max_connections,
        max_concurrency,
        c_max_reuse_times,
        h_max_request_times,
        h_max_reusable_secs,
        h_keep_alive_period,
    }
}

fn parse_xhttp_download_settings(ds: &serde_json::Map<String, serde_json::Value>) -> XhttpDownloadSettings {
    let server = ds
        .get("address")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string());

    let port = ds.get("port").and_then(|v| v.as_u64()).map(|n| n as u16);

    let security = ds
        .get("security")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_lowercase();

    let (mut tls, mut servername, mut client_fingerprint, mut alpn, mut skip_cert_verify, mut reality_opts) =
        (None, None, None, None, None, None);

    if security == "tls" || security == "reality" {
        tls = Some(true);
        if let Some(tls_settings) = ds.get("tlsSettings").and_then(|v| v.as_object()) {
            servername = tls_settings
                .get("serverName")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            client_fingerprint = tls_settings
                .get("fingerprint")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string());
            alpn = tls_settings.get("alpn").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });
            skip_cert_verify = tls_settings
                .get("allowInsecure")
                .and_then(|v| v.as_bool())
                .filter(|&b| b);
        }
        if security == "reality" {
            if let Some(rs) = ds.get("realitySettings").and_then(|v| v.as_object()) {
                let public_key = rs
                    .get("publicKey")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                if !public_key.is_empty() {
                    let short_id = rs
                        .get("shortId")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    reality_opts = Some(RealityOpts {
                        public_key,
                        short_id,
                    });
                }
            }
        }
    }

    let (path, host, headers, no_grpc_header, x_padding_bytes) =
        if let Some(xhs) = ds.get("xhttpSettings").and_then(|v| v.as_object()) {
            (
                xhs.get("path")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                xhs.get("host")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
                xhs.get("headers").and_then(|v| serde_yaml::to_value(v).ok()),
                xhs.get("noGRPCHeader")
                    .and_then(|v| v.as_bool())
                    .filter(|&b| b),
                xhs.get("xPaddingBytes")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string()),
            )
        } else {
            (None, None, None, None, None)
        };

    XhttpDownloadSettings {
        path,
        host,
        headers,
        no_grpc_header,
        x_padding_bytes,
        x_padding_obfs_mode: None,
        x_padding_key: None,
        x_padding_header: None,
        x_padding_placement: None,
        x_padding_method: None,
        uplink_http_method: None,
        session_placement: None,
        session_key: None,
        seq_placement: None,
        seq_key: None,
        uplink_data_placement: None,
        uplink_data_key: None,
        uplink_chunk_size: None,
        sc_max_each_post_bytes: None,
        sc_min_posts_interval_ms: None,
        reuse_settings: None,
        server,
        port,
        tls,
        servername,
        client_fingerprint,
        alpn,
        skip_cert_verify,
        reality_opts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_vless_basic() {
        let mut reg = NameRegistry::new();
        let uri = "vless://12345678-1234-1234-1234-123456789abc@example.com:443?type=ws&security=tls&sni=example.com#TestVless";
        let result = parse_vless(uri, &mut reg).unwrap();
        if let Proxy::Vless(p) = result {
            assert_eq!(p.uuid, "12345678-1234-1234-1234-123456789abc");
            assert_eq!(p.server, "example.com");
            assert_eq!(p.port, 443);
            assert_eq!(p.name, "TestVless");
            assert_eq!(p.tls, Some(true));
            assert_eq!(p.servername, Some("example.com".to_string()));
        } else {
            panic!("Expected Vless proxy");
        }
    }

    #[test]
    fn test_parse_vless_with_reality() {
        let mut reg = NameRegistry::new();
        let uri = "vless://uuid@server:443?security=reality&pbk=pubkey&sid=shortid&fp=chrome&type=tcp#RealityTest";
        let result = parse_vless(uri, &mut reg).unwrap();
        if let Proxy::Vless(p) = result {
            assert_eq!(p.tls, Some(true));
            assert!(p.reality_opts.is_some());
            let ro = p.reality_opts.unwrap();
            assert_eq!(ro.public_key, "pubkey");
            assert_eq!(ro.short_id, Some("shortid".to_string()));
        } else {
            panic!("Expected Vless proxy");
        }
    }

    #[test]
    fn test_parse_vless_missing_host() {
        let mut reg = NameRegistry::new();
        let result = parse_vless("vless://uuid@?type=tcp", &mut reg);
        assert!(result.is_err());
    }
}
