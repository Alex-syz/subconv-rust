//! Core proxy type definitions.
//!
//! The `Proxy` enum uses custom `Serialize`/`Deserialize` implementations
//! so that the `type` field acts as the YAML discriminant, producing flat
//! output matching mihomo's proxy format.
//!
//! An `Unknown` variant allows transparent pass-through of unrecognized
//! proxy types from Clash YAML subscriptions.

use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use super::models::{
    EchOpts, GrpcOpts, H2Opts, HttpOpts, RealityOpts, WsOpts, XhttpOpts,
};

// ── Default / predicate helpers for serde ───────────────────────────────────

fn default_true() -> bool {
    true
}
fn default_auto() -> String {
    "auto".into()
}

fn is_true(b: &bool) -> bool {
    *b
}
fn is_false(b: &bool) -> bool {
    !*b
}
fn is_zero_u32(n: &u32) -> bool {
    *n == 0
}
fn is_auto(s: &str) -> bool {
    s == "auto"
}

// ── Per-protocol proxy structs ──────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowsocksProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub password: String,
    pub cipher: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub plugin: Option<String>,
    #[serde(rename = "plugin-opts", skip_serializing_if = "Option::is_none")]
    pub plugin_opts: Option<IndexMap<String, serde_yaml::Value>>,
    #[serde(rename = "udp-over-tcp", skip_serializing_if = "Option::is_none")]
    pub udp_over_tcp: Option<bool>,
    #[serde(
        rename = "udp-over-tcp-version",
        skip_serializing_if = "Option::is_none"
    )]
    pub udp_over_tcp_version: Option<u32>,
    #[serde(rename = "client-fingerprint", skip_serializing_if = "Option::is_none")]
    pub client_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShadowsocksRProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub password: String,
    pub cipher: String,
    pub obfs: String,
    #[serde(rename = "obfs-param", skip_serializing_if = "Option::is_none")]
    pub obfs_param: Option<String>,
    pub protocol: String,
    #[serde(rename = "protocol-param", skip_serializing_if = "Option::is_none")]
    pub protocol_param: Option<String>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VmessProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub uuid: String,
    #[serde(rename = "alterId", default, skip_serializing_if = "is_zero_u32")]
    pub alter_id: u32,
    #[serde(default = "default_auto", skip_serializing_if = "is_auto")]
    pub cipher: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub tls: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(rename = "skip-cert-verify", default, skip_serializing_if = "is_false")]
    pub skip_cert_verify: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    #[serde(rename = "private-key", skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servername: Option<String>,
    #[serde(rename = "ech-opts", skip_serializing_if = "Option::is_none")]
    pub ech_opts: Option<EchOpts>,
    #[serde(rename = "reality-opts", skip_serializing_if = "Option::is_none")]
    pub reality_opts: Option<RealityOpts>,
    #[serde(rename = "http-opts", skip_serializing_if = "Option::is_none")]
    pub http_opts: Option<HttpOpts>,
    #[serde(rename = "h2-opts", skip_serializing_if = "Option::is_none")]
    pub h2_opts: Option<H2Opts>,
    #[serde(rename = "grpc-opts", skip_serializing_if = "Option::is_none")]
    pub grpc_opts: Option<GrpcOpts>,
    #[serde(rename = "ws-opts", skip_serializing_if = "Option::is_none")]
    pub ws_opts: Option<WsOpts>,
    #[serde(rename = "packet-addr", skip_serializing_if = "Option::is_none")]
    pub packet_addr: Option<bool>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub xudp: bool,
    #[serde(rename = "packet-encoding", skip_serializing_if = "Option::is_none")]
    pub packet_encoding: Option<String>,
    #[serde(rename = "global-padding", skip_serializing_if = "Option::is_none")]
    pub global_padding: Option<bool>,
    #[serde(
        rename = "authenticated-length",
        skip_serializing_if = "Option::is_none"
    )]
    pub authenticated_length: Option<bool>,
    #[serde(rename = "client-fingerprint", skip_serializing_if = "Option::is_none")]
    pub client_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VlessProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flow: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
    #[serde(rename = "packet-addr", skip_serializing_if = "Option::is_none")]
    pub packet_addr: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xudp: Option<bool>,
    #[serde(rename = "packet-encoding", skip_serializing_if = "Option::is_none")]
    pub packet_encoding: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encryption: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(rename = "ech-opts", skip_serializing_if = "Option::is_none")]
    pub ech_opts: Option<EchOpts>,
    #[serde(rename = "reality-opts", skip_serializing_if = "Option::is_none")]
    pub reality_opts: Option<RealityOpts>,
    #[serde(rename = "http-opts", skip_serializing_if = "Option::is_none")]
    pub http_opts: Option<HttpOpts>,
    #[serde(rename = "h2-opts", skip_serializing_if = "Option::is_none")]
    pub h2_opts: Option<H2Opts>,
    #[serde(rename = "grpc-opts", skip_serializing_if = "Option::is_none")]
    pub grpc_opts: Option<GrpcOpts>,
    #[serde(rename = "ws-opts", skip_serializing_if = "Option::is_none")]
    pub ws_opts: Option<WsOpts>,
    #[serde(rename = "xhttp-opts", skip_serializing_if = "Option::is_none")]
    pub xhttp_opts: Option<XhttpOpts>,
    #[serde(rename = "ws-headers", skip_serializing_if = "Option::is_none")]
    pub ws_headers: Option<IndexMap<String, String>>,
    #[serde(rename = "skip-cert-verify", skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub certificate: Option<String>,
    #[serde(rename = "private-key", skip_serializing_if = "Option::is_none")]
    pub private_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub servername: Option<String>,
    #[serde(rename = "client-fingerprint", skip_serializing_if = "Option::is_none")]
    pub client_fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrojanProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    pub password: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
    #[serde(rename = "skip-cert-verify", skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub network: Option<String>,
    #[serde(rename = "grpc-opts", skip_serializing_if = "Option::is_none")]
    pub grpc_opts: Option<GrpcOpts>,
    #[serde(rename = "ws-opts", skip_serializing_if = "Option::is_none")]
    pub ws_opts: Option<WsOpts>,
    #[serde(rename = "client-fingerprint", skip_serializing_if = "Option::is_none")]
    pub client_fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HysteriaProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfs: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(rename = "auth-str", skip_serializing_if = "Option::is_none")]
    pub auth_str: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub protocol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<String>,
    #[serde(rename = "skip-cert-verify", skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Hysteria2Proxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub obfs: Option<String>,
    #[serde(rename = "obfs-password", skip_serializing_if = "Option::is_none")]
    pub obfs_password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(rename = "skip-cert-verify", skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub down: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub up: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TuicProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    #[serde(
        rename = "congestion-controller",
        skip_serializing_if = "Option::is_none"
    )]
    pub congestion_controller: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpn: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(rename = "disable-sni", skip_serializing_if = "Option::is_none")]
    pub disable_sni: Option<bool>,
    #[serde(rename = "udp-relay-mode", skip_serializing_if = "Option::is_none")]
    pub udp_relay_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpSocksProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tls: Option<bool>,
    #[serde(rename = "skip-cert-verify", skip_serializing_if = "Option::is_none")]
    pub skip_cert_verify: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnytlsProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    pub password: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sni: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    #[serde(
        rename = "skip-cert-verify",
        default,
        skip_serializing_if = "is_false"
    )]
    pub skip_cert_verify: bool,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MieruProxy {
    pub name: String,
    pub server: String,
    pub transport: String,
    #[serde(default = "default_true", skip_serializing_if = "is_true")]
    pub udp: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(rename = "port-range", skip_serializing_if = "Option::is_none")]
    pub port_range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multiplexing: Option<String>,
    #[serde(rename = "handshake-mode", skip_serializing_if = "Option::is_none")]
    pub handshake_mode: Option<String>,
    #[serde(rename = "traffic-pattern", skip_serializing_if = "Option::is_none")]
    pub traffic_pattern: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramProxy {
    pub name: String,
    pub server: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}


// ── Type aliases for backward compatibility ──────────────────────────────────

pub type SsProxy = ShadowsocksProxy;
pub type SsrProxy = ShadowsocksRProxy;

// ── Proxy enum ──────────────────────────────────────────────────────────────

/// The top-level proxy type. Custom `Serialize`/`Deserialize` implementations
/// inject/read the `type` field as the YAML discriminant, producing flat output:
///
/// ```yaml
/// - type: ss
///   name: my-ss
///   server: 1.2.3.4
/// ```
///
/// The `Unknown` variant passes through unrecognized proxy types intact.
#[derive(Debug, Clone)]
pub enum Proxy {
    Ss(ShadowsocksProxy),
    Ssr(ShadowsocksRProxy),
    Vmess(Box<VmessProxy>),
    Vless(Box<VlessProxy>),
    Trojan(TrojanProxy),
    Hysteria(HysteriaProxy),
    Hysteria2(Hysteria2Proxy),
    Tuic(TuicProxy),
    Http(HttpSocksProxy),
    Socks5(HttpSocksProxy),
    Anytls(AnytlsProxy),
    Mieru(MieruProxy),
    Telegram(TelegramProxy),
    Unknown(serde_yaml::Value),
}

impl Serialize for Proxy {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::Error;

        let mut map = serde_yaml::Mapping::new();
        let (type_tag, val): (&str, serde_yaml::Value) = match self {
            Proxy::Ss(p) => ("ss", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Ssr(p) => ("ssr", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Vmess(p) => ("vmess", serde_yaml::to_value(&**p).map_err(S::Error::custom)?),
            Proxy::Vless(p) => ("vless", serde_yaml::to_value(&**p).map_err(S::Error::custom)?),
            Proxy::Trojan(p) => ("trojan", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Hysteria(p) => ("hysteria", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Hysteria2(p) => {
                ("hysteria2", serde_yaml::to_value(p).map_err(S::Error::custom)?)
            }
            Proxy::Tuic(p) => ("tuic", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Http(p) => ("http", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Socks5(p) => ("socks5", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Anytls(p) => ("anytls", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Mieru(p) => ("mieru", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Telegram(p) => ("telegram", serde_yaml::to_value(p).map_err(S::Error::custom)?),
            Proxy::Unknown(v) => return v.serialize(serializer),
        };

        map.insert(
            serde_yaml::Value::String("type".into()),
            serde_yaml::Value::String(type_tag.to_string()),
        );
        merge_mapping(&mut map, val);
        map.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Proxy {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde::de::Error;

        let raw = serde_yaml::Value::deserialize(deserializer)?;

        let mapping = match &raw {
            serde_yaml::Value::Mapping(m) => m,
            _ => return Err(D::Error::custom("proxy must be a YAML mapping")),
        };

        let type_val = mapping
            .get(serde_yaml::Value::String("type".into()))
            .ok_or_else(|| D::Error::custom("proxy missing `type` field"))?;

        let type_str = match type_val {
            serde_yaml::Value::String(s) => s.as_str(),
            _ => return Err(D::Error::custom("`type` must be a string")),
        };

        match type_str {
            "ss" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Ss)
                .map_err(D::Error::custom),
            "ssr" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Ssr)
                .map_err(D::Error::custom),
            "vmess" => serde_yaml::from_value(raw.clone())
                .map(|v| Proxy::Vmess(Box::new(v)))
                .map_err(D::Error::custom),
            "vless" => serde_yaml::from_value(raw.clone())
                .map(|v| Proxy::Vless(Box::new(v)))
                .map_err(D::Error::custom),
            "trojan" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Trojan)
                .map_err(D::Error::custom),
            "hysteria" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Hysteria)
                .map_err(D::Error::custom),
            "hysteria2" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Hysteria2)
                .map_err(D::Error::custom),
            "tuic" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Tuic)
                .map_err(D::Error::custom),
            "http" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Http)
                .map_err(D::Error::custom),
            "socks5" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Socks5)
                .map_err(D::Error::custom),
            "anytls" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Anytls)
                .map_err(D::Error::custom),
            "mieru" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Mieru)
                .map_err(D::Error::custom),
            "telegram" => serde_yaml::from_value(raw.clone())
                .map(Proxy::Telegram)
                .map_err(D::Error::custom),
            _ => Ok(Proxy::Unknown(raw)),
        }
    }
}

// ── Proxy accessor methods ──────────────────────────────────────────────────

impl Proxy {
    /// Return the display name of this proxy node.
    pub fn name(&self) -> &str {
        match self {
            Self::Ss(p) => &p.name,
            Self::Ssr(p) => &p.name,
            Self::Vmess(p) => &p.name,
            Self::Vless(p) => &p.name,
            Self::Trojan(p) => &p.name,
            Self::Hysteria(p) => &p.name,
            Self::Hysteria2(p) => &p.name,
            Self::Tuic(p) => &p.name,
            Self::Http(p) => &p.name,
            Self::Socks5(p) => &p.name,
            Self::Anytls(p) => &p.name,
            Self::Mieru(p) => &p.name,
            Self::Telegram(p) => &p.name,
            Self::Unknown(v) => v
                .get("name")
                .and_then(|n| n.as_str())
                .unwrap_or("unknown"),
        }
    }

    /// Set the name field on any Proxy variant.
    pub fn set_name(&mut self, name: &str) {
        match self {
            Self::Ss(p) => p.name = name.into(),
            Self::Ssr(p) => p.name = name.into(),
            Self::Vmess(p) => p.name = name.into(),
            Self::Vless(p) => p.name = name.into(),
            Self::Trojan(p) => p.name = name.into(),
            Self::Hysteria(p) => p.name = name.into(),
            Self::Hysteria2(p) => p.name = name.into(),
            Self::Tuic(p) => p.name = name.into(),
            Self::Http(p) => p.name = name.into(),
            Self::Socks5(p) => p.name = name.into(),
            Self::Anytls(p) => p.name = name.into(),
            Self::Mieru(p) => p.name = name.into(),
            Self::Telegram(p) => p.name = name.into(),
            Self::Unknown(v) => {
                if let serde_yaml::Value::Mapping(ref mut m) = v {
                    m.insert(
                        serde_yaml::Value::String("name".into()),
                        serde_yaml::Value::String(name.into()),
                    );
                }
            }
        }
    }
}

/// Merge a serde_yaml::Value (must be a Mapping) into the target mapping.
fn merge_mapping(target: &mut serde_yaml::Mapping, source: serde_yaml::Value) {
    if let serde_yaml::Value::Mapping(src) = source {
        for (k, v) in src {
            target.insert(k, v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ss_proxy_round_trip() {
        let proxy = Proxy::Ss(ShadowsocksProxy {
            name: "test-ss".into(),
            server: "1.2.3.4".into(),
            port: 8388,
            password: "pass".into(),
            cipher: "aes-256-gcm".into(),
            udp: true,
            plugin: None,
            plugin_opts: None,
            udp_over_tcp: None,
            udp_over_tcp_version: None,
            client_fingerprint: None,
        });
        let yaml = serde_yaml::to_string(&proxy).unwrap();
        assert!(yaml.contains("type: ss"));
        assert!(yaml.contains("name: test-ss"));
        assert!(yaml.contains("server: 1.2.3.4"));
        assert!(yaml.contains("port: 8388"));
        assert!(yaml.contains("cipher: aes-256-gcm"));
        // udp: true is skipped by skip_serializing_if = "is_true" (default value)
        assert!(!yaml.contains("udp"));
        assert!(!yaml.contains("plugin"));
        assert!(!yaml.contains("udp-over-tcp"));

        let back: Proxy = serde_yaml::from_str(&yaml).unwrap();
        assert!(matches!(back, Proxy::Ss(_)));
    }

    #[test]
    fn vmess_proxy_skips_defaults() {
        let proxy = Proxy::Vmess(Box::new(VmessProxy {
            name: "vm".into(),
            server: "5.6.7.8".into(),
            port: 443,
            uuid: "uuid-123".into(),
            alter_id: 0,
            cipher: "auto".into(),
            udp: true,
            network: None,
            tls: false,
            alpn: None,
            skip_cert_verify: false,
            fingerprint: None,
            certificate: None,
            private_key: None,
            servername: None,
            ech_opts: None,
            reality_opts: None,
            http_opts: None,
            h2_opts: None,
            grpc_opts: None,
            ws_opts: None,
            packet_addr: None,
            xudp: true,
            packet_encoding: None,
            global_padding: None,
            authenticated_length: None,
            client_fingerprint: None,
        }));
        let yaml = serde_yaml::to_string(&proxy).unwrap();
        assert!(yaml.contains("type: vmess"));
        assert!(!yaml.contains("alterId"));
        assert!(!yaml.contains("cipher"));
        assert!(!yaml.contains("tls: false"));
        assert!(!yaml.contains("skip-cert-verify"));
    }

    #[test]
    fn trojan_proxy_with_ws_opts() {
        let proxy = Proxy::Trojan(TrojanProxy {
            name: "tr".into(),
            server: "9.9.9.9".into(),
            port: 443,
            password: "trojan-pass".into(),
            udp: true,
            skip_cert_verify: None,
            sni: Some("example.com".into()),
            alpn: None,
            network: Some("ws".into()),
            grpc_opts: None,
            ws_opts: Some(WsOpts {
                path: Some("/ws".into()),
                headers: None,
                max_early_data: None,
                early_data_header_name: None,
                v2ray_http_upgrade: None,
                v2ray_http_upgrade_fast_open: None,
            }),
            client_fingerprint: Some("chrome".into()),
            fingerprint: None,
        });
        let yaml = serde_yaml::to_string(&proxy).unwrap();
        assert!(yaml.contains("type: trojan"));
        assert!(yaml.contains("sni: example.com"));
        assert!(yaml.contains("network: ws"));
        assert!(yaml.contains("ws-opts"));
        assert!(yaml.contains("path: /ws"));
        assert!(yaml.contains("client-fingerprint: chrome"));
    }

    #[test]
    fn unknown_proxy_pass_through() {
        let yaml = r#"
type: wireguard
name: wg1
server: 10.0.0.1
port: 51820
private-key: abc
"#;
        let proxy: Proxy = serde_yaml::from_str(yaml).unwrap();
        assert!(matches!(proxy, Proxy::Unknown(_)));

        let out = serde_yaml::to_string(&proxy).unwrap();
        assert!(out.contains("type: wireguard"));
        assert!(out.contains("name: wg1"));
    }

    #[test]
    fn hysteria2_proxy_fields() {
        let proxy = Proxy::Hysteria2(Hysteria2Proxy {
            name: "hy2".into(),
            server: "hy2.example.com".into(),
            port: 443,
            password: Some("auth".into()),
            obfs: Some("salamander".into()),
            obfs_password: Some("obfs-pass".into()),
            sni: Some("hy2.example.com".into()),
            skip_cert_verify: None,
            alpn: None,
            fingerprint: None,
            down: None,
            up: None,
        });
        let yaml = serde_yaml::to_string(&proxy).unwrap();
        assert!(yaml.contains("type: hysteria2"));
        assert!(yaml.contains("obfs: salamander"));
        assert!(yaml.contains("obfs-password: obfs-pass"));
    }

    #[test]
    fn proxy_name_extracts_name() {
        let proxy = Proxy::Ss(ShadowsocksProxy {
            name: "my-ss".into(),
            server: "1.1.1.1".into(),
            port: 8388,
            password: "p".into(),
            cipher: "c".into(),
            udp: true,
            plugin: None,
            plugin_opts: None,
            udp_over_tcp: None,
            udp_over_tcp_version: None,
            client_fingerprint: None,
        });
        assert_eq!(proxy.name(), "my-ss");
    }

    #[test]
    fn socks5_proxy_uses_http_socks_struct() {
        let proxy = Proxy::Socks5(HttpSocksProxy {
            name: "socks".into(),
            server: "127.0.0.1".into(),
            port: 1080,
            username: None,
            password: None,
            tls: None,
            skip_cert_verify: None,
        });
        let yaml = serde_yaml::to_string(&proxy).unwrap();
        assert!(yaml.contains("type: socks5"));
        assert!(yaml.contains("port: 1080"));
    }
}
