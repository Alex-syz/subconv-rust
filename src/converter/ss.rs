//! Shadowsocks share link parser.
//!
//! Format: `ss://base64(cipher:password)@host:port#name`
//!     or: `ss://base64(cipher:password@host:port)#name`

use indexmap::IndexMap;
use url::Url;

use super::proxy::{Proxy, ShadowsocksProxy};
use super::registry::NameRegistry;
use super::util::{base64_decode_auto, url_decode};
use crate::SubconvError;

/// Parse a Shadowsocks share link into a Proxy.
pub fn parse_ss(uri: &str, registry: &mut NameRegistry) -> Result<Proxy, SubconvError> {
    let url = parse_ss_url(uri)?;
    let query: IndexMap<String, String> = url.query_pairs().into_owned().collect();

    let (cipher, password) = parse_userinfo(&url)?;

    let server = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("shadowsocks: missing host".into()))?
        .to_string();

    let port = url
        .port()
        .ok_or_else(|| SubconvError::Parse("shadowsocks: missing port".into()))?;

    let udp_over_tcp = if query.get("udp-over-tcp").map(|v| v.as_str()) == Some("true")
        || query.get("uot").map(|v| v.as_str()) == Some("1")
    {
        Some(true)
    } else {
        None
    };

    let (plugin, plugin_opts) = if let Some(plugin_str) = query.get("plugin") {
        parse_plugin(plugin_str)?
    } else {
        (None, None)
    };

    let name = registry.register(&url_decode(url.fragment().unwrap_or("")));

    Ok(Proxy::Ss(ShadowsocksProxy {
        name,
        server,
        port,
        password,
        cipher,
        udp: true,
        plugin,
        plugin_opts,
        udp_over_tcp,
        udp_over_tcp_version: None,
        client_fingerprint: None,
    }))
}

/// Parse the SS URL, handling the case where the host part is base64-encoded.
fn parse_ss_url(uri: &str) -> Result<Url, SubconvError> {
    let url = Url::parse(uri).map_err(|e| SubconvError::Parse(format!("shadowsocks: {e}")))?;

    if url.scheme() != "ss" {
        return Err(SubconvError::Parse("shadowsocks: invalid scheme".into()));
    }

    // If port is present, the URL is already in standard form
    if url.port().is_some() {
        return Ok(url);
    }

    // Otherwise, the host part might be base64-encoded
    let host_part = url
        .host_str()
        .ok_or_else(|| SubconvError::Parse("shadowsocks: missing host".into()))?;

    let decoded = base64_decode_auto(host_part).map_err(|_| {
        SubconvError::Parse("shadowsocks: failed to decode base64 host".into())
    })?;

    let reparsed = Url::parse(&format!("ss://{decoded}")).map_err(|e| {
        SubconvError::Parse(format!("shadowsocks: failed to reparse decoded URL: {e}"))
    })?;

    if reparsed.host_str().is_none() {
        return Err(SubconvError::Parse("shadowsocks: missing host after decode".into()));
    }

    // Carry over query and fragment from the original URL
    let mut result = reparsed;
    if let Some(q) = url.query() {
        result.set_query(Some(q));
    }
    if let Some(f) = url.fragment() {
        result.set_fragment(Some(f));
    }

    Ok(result)
}

/// Extract cipher and password from the URL userinfo.
fn parse_userinfo(url: &Url) -> Result<(String, String), SubconvError> {
    // Python code only uses standard userinfo when BOTH username AND password are present.
    // If only username is present (no password), it falls through to base64 decode.
    let username = url.username();
    if !username.is_empty() && url.password().is_some() {
        let password = url.password().unwrap_or("");
        return Ok((url_decode(username), password.to_string()));
    }

    // Fall back to base64-encoded userinfo in the netloc
    let original_str = url.to_string();

    // Extract the part between "ss://" and the host
    if let Some(after_scheme) = original_str.strip_prefix("ss://") {
        if let Some(at_pos) = after_scheme.find('@') {
            let raw_userinfo = &after_scheme[..at_pos];
            let decoded = base64_decode_auto(raw_userinfo).map_err(|_| {
                SubconvError::Parse("shadowsocks: invalid base64 userinfo".into())
            })?;

            if let Some(colon_pos) = decoded.find(':') {
                let cipher = decoded[..colon_pos].to_string();
                let password = decoded[colon_pos + 1..].to_string();
                return Ok((cipher, password));
            }
            return Err(SubconvError::Parse(
                "shadowsocks: invalid credentials format".into(),
            ));
        }
    }

    Err(SubconvError::Parse(
        "shadowsocks: missing userinfo".into(),
    ))
}


/// Parse the plugin string into plugin name and options.
#[allow(clippy::type_complexity)]
fn parse_plugin(
    plugin: &str,
) -> Result<(Option<String>, Option<IndexMap<String, serde_yaml::Value>>), SubconvError> {
    let segments: Vec<&str> = plugin.split(';').collect();
    if segments.is_empty() {
        return Ok((None, None));
    }

    let plugin_name = segments[0];
    let mut options: IndexMap<&str, &str> = IndexMap::new();
    for segment in &segments[1..] {
        if let Some(eq_pos) = segment.find('=') {
            options.insert(&segment[..eq_pos], &segment[eq_pos + 1..]);
        }
    }

    if plugin_name.contains("obfs") {
        let mut opts = IndexMap::new();
        if let Some(mode) = options.get("obfs") {
            opts.insert("mode".to_string(), serde_yaml::Value::String((*mode).to_string()));
        }
        if let Some(host) = options.get("obfs-host") {
            opts.insert("host".to_string(), serde_yaml::Value::String((*host).to_string()));
        }
        return Ok((Some("obfs".into()), Some(opts)));
    }

    if plugin_name.contains("v2ray-plugin") {
        let mut opts = IndexMap::new();
        if let Some(mode) = options.get("mode") {
            opts.insert("mode".to_string(), serde_yaml::Value::String((*mode).to_string()));
        }
        if let Some(host) = options.get("host") {
            opts.insert("host".to_string(), serde_yaml::Value::String((*host).to_string()));
        }
        if let Some(path) = options.get("path") {
            opts.insert("path".to_string(), serde_yaml::Value::String((*path).to_string()));
        }
        if plugin.contains("tls") {
            opts.insert("tls".to_string(), serde_yaml::Value::Bool(true));
        }
        return Ok((Some("v2ray-plugin".into()), Some(opts)));
    }

    Ok((None, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ss_standard() {
        let mut reg = NameRegistry::new();
        // base64("aes-256-gcm:password") = "YWVzLTI1Ni1nY206cGFzc3dvcmQ"
        let uri = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ@server:443#TestSS";
        let result = parse_ss(uri, &mut reg).unwrap();
        if let Proxy::Ss(p) = result {
            assert_eq!(p.cipher, "aes-256-gcm");
            assert_eq!(p.password, "password");
            assert_eq!(p.server, "server");
            assert_eq!(p.port, 443);
            assert_eq!(p.name, "TestSS");
        } else {
            panic!("Expected Ss proxy");
        }
    }

    #[test]
    fn test_parse_ss_with_plugin() {
        let mut reg = NameRegistry::new();
        let uri = "ss://YWVzLTI1Ni1nY206cGFzc3dvcmQ@server:443?plugin=obfs-local%3Bobfs%3Dhttp%3Bobfs-host%3Dexample.com#WithPlugin";
        let result = parse_ss(uri, &mut reg).unwrap();
        if let Proxy::Ss(p) = result {
            assert_eq!(p.plugin, Some("obfs".to_string()));
            assert!(p.plugin_opts.is_some());
        } else {
            panic!("Expected Ss proxy");
        }
    }

    #[test]
    fn test_parse_ss_invalid_scheme() {
        let mut reg = NameRegistry::new();
        let result = parse_ss("http://example.com", &mut reg);
        assert!(result.is_err());
    }
}
