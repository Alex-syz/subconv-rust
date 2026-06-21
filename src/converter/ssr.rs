//! ShadowsocksR share link parser.
//!
//! Format: `ssr://base64(server:port:protocol:method:obfs:base64(password)/?obfsparam=base64&protoparam=base64&remarks=base64)`

use super::proxy::{Proxy, ShadowsocksRProxy};
use super::registry::NameRegistry;
use super::util::base64_decode_auto;
use crate::SubconvError;

/// Parse a ShadowsocksR share link into a Proxy.
pub fn parse_ssr(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let body = uri
        .strip_prefix("ssr://")
        .ok_or_else(|| SubconvError::Parse("ssr: invalid scheme".into()))?;

    let decoded = base64_decode_auto(body).map_err(|_| {
        SubconvError::Parse("ssr: failed to decode base64 body".into())
    })?;

    // Split into main part and query part
    let (main_part, query_str) = if let Some(q_pos) = decoded.find("/?") {
        (&decoded[..q_pos], &decoded[q_pos + 2..])
    } else if let Some(q_pos) = decoded.find('?') {
        (&decoded[..q_pos], &decoded[q_pos + 1..])
    } else {
        (decoded.as_str(), "")
    };

    let parts: Vec<&str> = main_part.split(':').collect();
    if parts.len() < 6 {
        return Err(SubconvError::Parse(
            "ssr: invalid format, expected at least 6 colon-separated parts".into(),
        ));
    }

    let server = parts[0].to_string();
    let port: u16 = parts[1]
        .parse()
        .map_err(|_| SubconvError::Parse("ssr: invalid port".into()))?;
    let protocol = parts[2].to_string();
    let cipher = parts[3].to_string();
    let obfs = parts[4].to_string();

    // The password part may contain extra segments after the 6th colon
    let password_encoded = parts[5];
    let password = base64_decode_auto(password_encoded).map_err(|_| {
        SubconvError::Parse("ssr: failed to decode base64 password".into())
    })?;

    // Parse query parameters
    let mut obfs_param: Option<String> = None;
    let mut protocol_param: Option<String> = None;
    let mut remarks: Option<String> = None;

    for pair in query_str.split('&') {
        if pair.is_empty() {
            continue;
        }
        if let Some(eq_pos) = pair.find('=') {
            let key = &pair[..eq_pos];
            let value = &pair[eq_pos + 1..];
            match key {
                "obfsparam" => {
                    if !value.is_empty() {
                        obfs_param = base64_decode_auto(value).ok();
                    }
                }
                "protoparam" => {
                    if !value.is_empty() {
                        protocol_param = base64_decode_auto(value).ok();
                    }
                }
                "remarks" if !value.is_empty() => {
                    remarks = base64_decode_auto(value).ok();
                }
                _ => {}
            }
        }
    }

    let name = registry.register(remarks.as_deref().unwrap_or(&format!("{server}:{port}")));

    Ok(Proxy::Ssr(ShadowsocksRProxy {
        name,
        server,
        port,
        password,
        cipher,
        obfs,
        obfs_param,
        protocol,
        protocol_param,
        udp: true,
    }))
}


#[cfg(test)]
mod tests {
    use base64::Engine;
    use super::*;

    #[test]
    fn test_parse_ssr_basic() {
        let mut reg = NameRegistry::new();
        let inner = "server:8388:auth_aes128_md5:aes-256-cfb:tls:aGVsbG8=?obfsparam=&protoparam=&remarks=VGVzdA";
        let encoded = base64::engine::general_purpose::STANDARD.encode(inner);
        let uri = format!("ssr://{encoded}");
        let result = parse_ssr(&uri, &mut reg).unwrap();
        if let Proxy::Ssr(p) = result {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 8388);
            assert_eq!(p.protocol, "auth_aes128_md5");
            assert_eq!(p.cipher, "aes-256-cfb");
            assert_eq!(p.obfs, "tls");
            assert_eq!(p.password, "hello");
            assert_eq!(p.name, "Test");
        } else {
            panic!("Expected Ssr proxy");
        }
    }

    #[test]
    fn test_parse_ssr_invalid_scheme() {
        let mut reg = NameRegistry::new();
        let result = parse_ssr("http://example.com", &mut reg);
        assert!(result.is_err());
    }
}
