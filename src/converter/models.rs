//! Transport-layer option structs for mihomo proxy definitions.
//!
//! Each struct derives serde traits and uses `#[serde(rename, skip_serializing_if)]`
//! to produce clean YAML that matches mihomo's expected format.

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};

/// A value that can be either a single string or a list of strings.
///
/// Used for fields like `H2Opts::path` which accept both forms.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StringOrList {
    One(String),
    Many(Vec<String>),
}

// ── HTTP transport ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, Vec<String>>>,
}

// ── HTTP/2 transport ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H2Opts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<StringOrList>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, Vec<String>>>,
}

// ── gRPC transport ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrpcOpts {
    #[serde(rename = "grpc-service-name", skip_serializing_if = "Option::is_none")]
    pub grpc_service_name: Option<String>,
    #[serde(rename = "grpc-user-agent", skip_serializing_if = "Option::is_none")]
    pub grpc_user_agent: Option<String>,
}

// ── WebSocket transport ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
    #[serde(rename = "max-early-data", skip_serializing_if = "Option::is_none")]
    pub max_early_data: Option<u32>,
    #[serde(rename = "early-data-header-name", skip_serializing_if = "Option::is_none")]
    pub early_data_header_name: Option<String>,
    #[serde(rename = "v2ray-http-upgrade", skip_serializing_if = "Option::is_none")]
    pub v2ray_http_upgrade: Option<bool>,
    #[serde(
        rename = "v2ray-http-upgrade-fast-open",
        skip_serializing_if = "Option::is_none"
    )]
    pub v2ray_http_upgrade_fast_open: Option<bool>,
}

// ── Reality ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealityOpts {
    #[serde(rename = "public-key")]
    pub public_key: String,
    #[serde(rename = "short-id", skip_serializing_if = "Option::is_none")]
    pub short_id: Option<String>,
}

// ── ECH ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<String>,
    #[serde(rename = "query-server-name", skip_serializing_if = "Option::is_none")]
    pub query_server_name: Option<String>,
}

// ── XHTTP transport ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XhttpReuseSettings {
    #[serde(rename = "max-connections", skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<String>,
    #[serde(rename = "max-concurrency", skip_serializing_if = "Option::is_none")]
    pub max_concurrency: Option<String>,
    #[serde(rename = "c-max-reuse-times", skip_serializing_if = "Option::is_none")]
    pub c_max_reuse_times: Option<String>,
    #[serde(rename = "h-max-request-times", skip_serializing_if = "Option::is_none")]
    pub h_max_request_times: Option<String>,
    #[serde(rename = "h-max-reusable-secs", skip_serializing_if = "Option::is_none")]
    pub h_max_reusable_secs: Option<String>,
    #[serde(rename = "h-keep-alive-period", skip_serializing_if = "Option::is_none")]
    pub h_keep_alive_period: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XhttpDownloadSettings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<serde_yaml::Value>,
    #[serde(rename = "no-grpc-header", skip_serializing_if = "Option::is_none")]
    pub no_grpc_header: Option<bool>,
    #[serde(rename = "x-padding-bytes", skip_serializing_if = "Option::is_none")]
    pub x_padding_bytes: Option<String>,
    #[serde(rename = "x-padding-obfs-mode", skip_serializing_if = "Option::is_none")]
    pub x_padding_obfs_mode: Option<bool>,
    #[serde(rename = "x-padding-key", skip_serializing_if = "Option::is_none")]
    pub x_padding_key: Option<String>,
    #[serde(rename = "x-padding-header", skip_serializing_if = "Option::is_none")]
    pub x_padding_header: Option<String>,
    #[serde(rename = "x-padding-placement", skip_serializing_if = "Option::is_none")]
    pub x_padding_placement: Option<String>,
    #[serde(rename = "x-padding-method", skip_serializing_if = "Option::is_none")]
    pub x_padding_method: Option<String>,
    #[serde(rename = "uplink-http-method", skip_serializing_if = "Option::is_none")]
    pub uplink_http_method: Option<String>,
    #[serde(rename = "session-placement", skip_serializing_if = "Option::is_none")]
    pub session_placement: Option<String>,
    #[serde(rename = "session-key", skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(rename = "seq-placement", skip_serializing_if = "Option::is_none")]
    pub seq_placement: Option<String>,
    #[serde(rename = "seq-key", skip_serializing_if = "Option::is_none")]
    pub seq_key: Option<String>,
    #[serde(
        rename = "uplink-data-placement",
        skip_serializing_if = "Option::is_none"
    )]
    pub uplink_data_placement: Option<String>,
    #[serde(rename = "uplink-data-key", skip_serializing_if = "Option::is_none")]
    pub uplink_data_key: Option<String>,
    #[serde(rename = "uplink-chunk-size", skip_serializing_if = "Option::is_none")]
    pub uplink_chunk_size: Option<u32>,
    #[serde(
        rename = "sc-max-each-post-bytes",
        skip_serializing_if = "Option::is_none"
    )]
    pub sc_max_each_post_bytes: Option<u32>,
    #[serde(
        rename = "sc-min-posts-interval-ms",
        skip_serializing_if = "Option::is_none"
    )]
    pub sc_min_posts_interval_ms: Option<u32>,
    #[serde(rename = "reuse-settings", skip_serializing_if = "Option::is_none")]
    pub reuse_settings: Option<XhttpReuseSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servername: Option<String>,
    #[serde(rename = "client-fingerprint", skip_serializing_if = "Option::is_none")]
    pub client_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(rename = "skip-cert-verify", skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
    #[serde(rename = "reality-opts", skip_serializing_if = "Option::is_none")]
    pub reality_opts: Option<RealityOpts>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct XhttpOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<IndexMap<String, String>>,
    #[serde(rename = "no-grpc-header", skip_serializing_if = "Option::is_none")]
    pub no_grpc_header: Option<bool>,
    #[serde(rename = "x-padding-bytes", skip_serializing_if = "Option::is_none")]
    pub x_padding_bytes: Option<String>,
    #[serde(rename = "x-padding-obfs-mode", skip_serializing_if = "Option::is_none")]
    pub x_padding_obfs_mode: Option<bool>,
    #[serde(rename = "x-padding-key", skip_serializing_if = "Option::is_none")]
    pub x_padding_key: Option<String>,
    #[serde(rename = "x-padding-header", skip_serializing_if = "Option::is_none")]
    pub x_padding_header: Option<String>,
    #[serde(rename = "x-padding-placement", skip_serializing_if = "Option::is_none")]
    pub x_padding_placement: Option<String>,
    #[serde(rename = "x-padding-method", skip_serializing_if = "Option::is_none")]
    pub x_padding_method: Option<String>,
    #[serde(rename = "uplink-http-method", skip_serializing_if = "Option::is_none")]
    pub uplink_http_method: Option<String>,
    #[serde(rename = "session-placement", skip_serializing_if = "Option::is_none")]
    pub session_placement: Option<String>,
    #[serde(rename = "session-key", skip_serializing_if = "Option::is_none")]
    pub session_key: Option<String>,
    #[serde(rename = "seq-placement", skip_serializing_if = "Option::is_none")]
    pub seq_placement: Option<String>,
    #[serde(rename = "seq-key", skip_serializing_if = "Option::is_none")]
    pub seq_key: Option<String>,
    #[serde(
        rename = "uplink-data-placement",
        skip_serializing_if = "Option::is_none"
    )]
    pub uplink_data_placement: Option<String>,
    #[serde(rename = "uplink-data-key", skip_serializing_if = "Option::is_none")]
    pub uplink_data_key: Option<String>,
    #[serde(rename = "uplink-chunk-size", skip_serializing_if = "Option::is_none")]
    pub uplink_chunk_size: Option<u32>,
    #[serde(
        rename = "sc-max-each-post-bytes",
        skip_serializing_if = "Option::is_none"
    )]
    pub sc_max_each_post_bytes: Option<u32>,
    #[serde(
        rename = "sc-min-posts-interval-ms",
        skip_serializing_if = "Option::is_none"
    )]
    pub sc_min_posts_interval_ms: Option<u32>,
    #[serde(rename = "reuse-settings", skip_serializing_if = "Option::is_none")]
    pub reuse_settings: Option<XhttpReuseSettings>,
    #[serde(rename = "download-settings", skip_serializing_if = "Option::is_none")]
    pub download_settings: Option<XhttpDownloadSettings>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_or_list_deserialize_single() {
        let v: StringOrList = serde_yaml::from_str("\"hello\"").unwrap();
        assert!(matches!(v, StringOrList::One(s) if s == "hello"));
    }

    #[test]
    fn string_or_list_deserialize_list() {
        let v: StringOrList = serde_yaml::from_str("[\"a\", \"b\"]").unwrap();
        assert!(matches!(v, StringOrList::Many(l) if l.len() == 2));
    }

    #[test]
    fn reality_opts_serializes_kebab() {
        let opts = RealityOpts {
            public_key: "pk123".into(),
            short_id: Some("ab".into()),
        };
        let yaml = serde_yaml::to_string(&opts).unwrap();
        assert!(yaml.contains("public-key: pk123"));
        assert!(yaml.contains("short-id: ab"));
        assert!(!yaml.contains("public_key"));
    }

    #[test]
    fn ws_opts_skips_none_fields() {
        let opts = WsOpts {
            path: Some("/ws".into()),
            headers: None,
            max_early_data: None,
            early_data_header_name: None,
            v2ray_http_upgrade: None,
            v2ray_http_upgrade_fast_open: None,
        };
        let yaml = serde_yaml::to_string(&opts).unwrap();
        assert!(yaml.contains("path: /ws"));
        assert!(!yaml.contains("headers"));
        assert!(!yaml.contains("max-early-data"));
    }

    #[test]
    fn grpc_opts_renames_fields() {
        let opts = GrpcOpts {
            grpc_service_name: Some("svc".into()),
            grpc_user_agent: None,
        };
        let yaml = serde_yaml::to_string(&opts).unwrap();
        assert!(yaml.contains("grpc-service-name: svc"));
        assert!(!yaml.contains("grpc-service_name"));
        assert!(!yaml.contains("grpc-user-agent"));
    }

    #[test]
    fn xhttp_opts_round_trip() {
        let opts = XhttpOpts {
            path: Some("/xhttp".into()),
            host: None,
            mode: Some("packet-up".into()),
            headers: None,
            no_grpc_header: None,
            x_padding_bytes: None,
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
            download_settings: None,
        };
        let yaml = serde_yaml::to_string(&opts).unwrap();
        let back: XhttpOpts = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(back.path, Some("/xhttp".into()));
        assert_eq!(back.mode, Some("packet-up".into()));
    }
}

// ── Shadowsocks plugin options ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObfsPluginOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct V2rayPluginOpts {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
}

/// Plugin options can be either obfs or v2ray-plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PluginOpts {
    Obfs(ObfsPluginOpts),
    V2ray(V2rayPluginOpts),
}
