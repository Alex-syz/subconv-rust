//! HTTP and SOCKS5 share link parser.
//!
//! Formats: `http://user:pass@host:port#name`, `https://...`,
//! `socks5://user:pass@host:port#name`, `socks5h://...`

use url::Url;

use super::proxy::{HttpSocksProxy, Proxy};
use super::registry::NameRegistry;
use super::util::{base64_decode_auto, url_decode};
use crate::SubconvError;

/// Parse an HTTP share link into a Proxy.
pub fn parse_http(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    parse_http_socks(uri, registry, "http")
}

/// Parse a SOCKS5 share link into a Proxy.
pub fn parse_socks5(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    parse_http_socks(uri, registry, "socks5")
}

fn parse_http_socks(
    uri: &str,
    registry: &mut NameRegistry,
    default_type: &str,
) -> Result<Proxy, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("http/socks: {e}")))?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("http/socks: missing host".into()))?;

    let scheme = url.scheme().to_lowercase();
    let proxy_type = match scheme.as_str() {
        "socks" | "socks5" | "socks5h" => "socks5",
        "http" | "https" => "http",
        _ => default_type,
    };

    let port = match url.port() {
        Some(p) => p,
        None if scheme == "https" => 443,
        _ => return Err(SubconvError::Parse("http/socks: missing port".into())),
    };

    let (username, password) = parse_credentials(&url)?;

    let name = registry.register(&url_decode(url.fragment().unwrap_or(&format!("{server}:{port}"))));

    let tls = if scheme == "https" { Some(true) } else { None };

    let proxy = HttpSocksProxy {
        name,
        server: server.to_string(),
        port,
        username,
        password,
        tls,
        skip_cert_verify: Some(true),
    };

    if proxy_type == "socks5" {
        Ok(Proxy::Socks5(proxy))
    } else {
        Ok(Proxy::Http(proxy))
    }
}

fn parse_credentials(url: &Url) -> Result<(Option<String>, Option<String>), SubconvError> {
    if !url.username().is_empty() {
        let username = Some(url_decode(url.username()));
        let password = url.password().map(|p| p.to_string());
        return Ok((username, password));
    }

    let original_str = url.to_string();
    let scheme_prefix = format!("{}://", url.scheme());

    if let Some(after_scheme) = original_str.strip_prefix(&scheme_prefix) {
        if let Some(at_pos) = after_scheme.find('@') {
            let raw_userinfo = &after_scheme[..at_pos];
            let decoded_raw = url_decode(raw_userinfo);
            if let Ok(fully_decoded) = base64_decode_auto(&decoded_raw) {
                return Ok(parse_userinfo_str(&fully_decoded));
            }
            return Ok(parse_userinfo_str(&decoded_raw));
        }
    }

    Ok((None, None))
}


fn parse_userinfo_str(s: &str) -> (Option<String>, Option<String>) {
    if let Some(colon_pos) = s.find(':') {
        let username = s[..colon_pos].to_string();
        let password = s[colon_pos + 1..].to_string();
        if username.is_empty() {
            (None, None)
        } else if password.is_empty() {
            (Some(username), None)
        } else {
            (Some(username), Some(password))
        }
    } else if s.is_empty() {
        (None, None)
    } else {
        (Some(s.to_string()), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_http_basic() {
        let mut reg = NameRegistry::new();
        let uri = "http://user:pass@server:8080#TestHTTP";
        let result = parse_http(uri, &mut reg).unwrap();
        if let Proxy::Http(p) = result {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 8080);
            assert_eq!(p.name, "TestHTTP");
            assert_eq!(p.username, Some("user".to_string()));
            assert_eq!(p.password, Some("pass".to_string()));
            assert_eq!(p.tls, None);
        } else {
            panic!("Expected Http proxy");
        }
    }

    #[test]
    fn test_parse_https() {
        let mut reg = NameRegistry::new();
        let uri = "https://user:pass@server#TestHTTPS";
        let result = parse_http(uri, &mut reg).unwrap();
        if let Proxy::Http(p) = result {
            assert_eq!(p.tls, Some(true));
            assert_eq!(p.port, 443);
        } else {
            panic!("Expected Http proxy");
        }
    }

    #[test]
    fn test_parse_socks5() {
        let mut reg = NameRegistry::new();
        let uri = "socks5://user:pass@server:1080#TestSocks5";
        let result = parse_socks5(uri, &mut reg).unwrap();
        if let Proxy::Socks5(p) = result {
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 1080);
            assert_eq!(p.name, "TestSocks5");
        } else {
            panic!("Expected Socks5 proxy");
        }
    }

    #[test]
    fn test_parse_http_missing_port() {
        let mut reg = NameRegistry::new();
        let result = parse_http("http://server", &mut reg);
        assert!(result.is_err());
    }
}
