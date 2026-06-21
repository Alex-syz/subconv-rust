//! V2Ray share link converter — entry point and dispatcher.
//!
//! Converts a subscription string (base64-encoded or plain text) into a list
//! of strongly-typed Proxy values by dispatching to protocol-specific parsers.

pub mod anytls;
pub mod hysteria;
pub mod hysteria2;
pub mod http_socks;
pub mod mieru;
pub mod models;
pub mod proxy;
pub mod registry;
pub mod ss;
pub mod ssr;
pub mod telegram;
pub mod trojan;
pub mod tuic;
pub mod util;
pub mod vless;
pub mod vmess;

use proxy::Proxy;
use registry::NameRegistry;
use util::base64_decode_auto;
use crate::SubconvError;

/// Convert a subscription string into a list of Proxies.
///
/// The input may be base64-encoded or plain text. Each line is parsed
/// independently; malformed lines are silently skipped.
pub fn converts_v2ray(buf: &str, registry: &mut NameRegistry) -> Result<Vec<Proxy>, SubconvError> {
    // Try base64 decode first, fall back to raw text
    let data = base64_decode_auto(buf.trim()).unwrap_or_else(|_| buf.to_string());

    let mut proxies = Vec::new();

    for line in data.lines() {
        let line = line.trim_end_matches([' ', '\r']);
        if line.is_empty() {
            continue;
        }

        if !line.contains("://") {
            continue;
        }

        let (scheme, _) = line.split_once("://").unwrap_or(("", ""));
        let scheme = scheme.to_lowercase();

        match dispatch_parse(line, &scheme, registry) {
            Ok(mut results) => proxies.append(&mut results),
            Err(_) => continue,
        }
    }

    if proxies.is_empty() {
        return Err(SubconvError::Parse("No valid proxies found".into()));
    }

    Ok(proxies)
}


/// Dispatch a single line to the appropriate parser based on scheme.
fn dispatch_parse(
    line: &str,
    scheme: &str,
    registry: &mut NameRegistry,
) -> Result<Vec<Proxy>, SubconvError> {
    match scheme {
        "ss" => ss::parse_ss(line, registry).map(|p| vec![p]),
        "ssr" => ssr::parse_ssr(line, registry).map(|p| vec![p]),
        "vmess" => vmess::parse_vmess(line, registry).map(|p| vec![p]),
        "vless" => vless::parse_vless(line, registry).map(|p| vec![p]),
        "trojan" => trojan::parse_trojan(line, registry).map(|p| vec![p]),
        "hysteria" => hysteria::parse_hysteria(line, registry).map(|p| vec![p]),
        "hysteria2" | "hy2" => hysteria2::parse_hysteria2(line, registry).map(|p| vec![p]),
        "tuic" => tuic::parse_tuic(line, registry).map(|p| vec![p]),
        "socks" | "socks5" | "socks5h" => http_socks::parse_socks5(line, registry).map(|p| vec![p]),
        "http" | "https" => {
            if scheme == "https" && line.contains("t.me") {
                telegram::parse_telegram_https(line, registry).map(|p| vec![p])
            } else {
                http_socks::parse_http(line, registry).map(|p| vec![p])
            }
        }
        "tg" => telegram::parse_telegram(line, registry).map(|p| vec![p]),
        "anytls" => anytls::parse_anytls(line, registry).map(|p| vec![p]),
        "mierus" => mieru::parse_mieru(line, registry),
        _ => Err(SubconvError::Parse(format!("unsupported scheme: {scheme}"))),
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine;
    use super::*;
    #[test]
    fn test_converts_v2ray_empty() {
        let mut reg = NameRegistry::new();
        let result = converts_v2ray("", &mut reg);
        assert!(result.is_err());
    }

    #[test]
    fn test_converts_v2ray_no_proxies() {
        let mut reg = NameRegistry::new();
        let result = converts_v2ray("just some random text\nno urls here", &mut reg);
        assert!(result.is_err());
    }

    #[test]
    fn test_converts_v2ray_trojan() {
        let mut reg = NameRegistry::new();
        let input = "trojan://password123@server:443?sni=example.com#TestTrojan";
        let result = converts_v2ray(input, &mut reg).unwrap();
        assert_eq!(result.len(), 1);
        assert!(matches!(result[0], Proxy::Trojan(_)));
    }

    #[test]
    fn test_converts_v2ray_skips_malformed() {
        let mut reg = NameRegistry::new();
        let input = "trojan://password123@server:443?sni=example.com#Good\ninvalid-line\nss://baduri";
        let result = converts_v2ray(input, &mut reg).unwrap();
        assert!(!result.is_empty());
    }

    #[test]
    fn test_converts_v2ray_base64_encoded() {
        let mut reg = NameRegistry::new();
        let plain = "trojan://password123@server:443?sni=example.com#B64Test";
        let encoded = base64::engine::general_purpose::STANDARD.encode(plain);
        let result = converts_v2ray(&encoded, &mut reg).unwrap();
        assert_eq!(result.len(), 1);
    }
}
