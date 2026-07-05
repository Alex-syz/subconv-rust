//! Shared utility functions for share-link conversion.
//!
//! Base64 decoding with auto-padding, URL decoding, boolean parsing,
//! and random User-Agent generation.

use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use base64::Engine;
use indexmap::IndexMap;

use crate::error::SubconvError;

/// Decode standard base64, automatically adding missing padding.
pub fn base64_decode_std(s: &str) -> Result<Vec<u8>, SubconvError> {
    let padded = pad_base64(s);
    STANDARD
        .decode(padded)
        .map_err(|e| SubconvError::Parse(format!("base64 decode failed: {e}")))
}

/// Decode URL-safe base64 (no padding), automatically adding missing padding.
pub fn base64_decode_url(s: &str) -> Result<Vec<u8>, SubconvError> {
    let padded = pad_base64(s);
    URL_SAFE
        .decode(padded)
        .map_err(|e| SubconvError::Parse(format!("url-safe base64 decode failed: {e}")))
}

/// Decode standard base64 to a UTF-8 string.
pub fn base64_decode_std_string(s: &str) -> Result<String, SubconvError> {
    let bytes = base64_decode_std(s)?;
    String::from_utf8(bytes)
        .map_err(|e| SubconvError::Parse(format!("base64 result is not valid UTF-8: {e}")))
}

/// Decode URL-safe base64 to a UTF-8 string.
pub fn base64_decode_url_string(s: &str) -> Result<String, SubconvError> {
    let bytes = base64_decode_url(s)?;
    String::from_utf8(bytes)
        .map_err(|e| SubconvError::Parse(format!("url-safe base64 result is not valid UTF-8: {e}")))
}

/// URL-decode a percent-encoded string.
pub fn url_decode(s: &str) -> String {
    urlencoding::decode(s)
        .map(|cow| cow.into_owned())
        .unwrap_or_else(|_| s.to_string())
}

/// Parse a string as a boolean. Returns `true` for "1", "true", "yes"
/// (case-insensitive), `false` for everything else.
pub fn parse_bool(s: &str) -> bool {
    matches!(s.to_ascii_lowercase().as_str(), "1" | "true" | "yes")
}

/// Parse an optional string as a boolean. Returns  for "1", "true",
/// "yes" (case-insensitive),  for None or everything else.
pub fn parse_bool_opt(s: Option<&String>) -> bool {
    s.map(|v| parse_bool(v)).unwrap_or(false)
}

/// Return a random desktop User-Agent string.
pub fn rand_user_agent() -> &'static str {
    use rand::Rng;
    let agents: &[&str] = &[
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:133.0) Gecko/20100101 Firefox/133.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:133.0) Gecko/20100101 Firefox/133.0",
        "Mozilla/5.0 (X11; Linux x86_64; rv:133.0) Gecko/20100101 Firefox/133.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.2 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Edg/131.0.0.0",
    ];
    let idx = rand::rng().random_range(0..agents.len());
    agents[idx]
}

/// Add `=` padding so the length is a multiple of 4.
fn pad_base64(s: &str) -> String {
    let rem = s.len() % 4;
    if rem == 0 {
        s.to_string()
    } else {
        let mut out = s.to_string();
        out.push_str(&"=".repeat(4 - rem));
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_std_round_trip() {
        let original = b"hello world";
        let encoded = STANDARD.encode(original);
        let decoded = base64_decode_std(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn base64_std_auto_padding() {
        // "a" encodes to "YQ==" — without padding it's "YQ"
        let decoded = base64_decode_std("YQ").unwrap();
        assert_eq!(decoded, b"a");
    }

    #[test]
    fn base64_url_round_trip() {
        let original = b"\xff\xfe\xfd";
        let encoded = URL_SAFE.encode(original);
        let decoded = base64_decode_url(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn base64_url_auto_padding() {
        // URL-safe base64 of bytes [0xfb, 0xff, 0xfe] is "-__-" (no padding needed, length % 4 == 0)
        let decoded = base64_decode_url("-__-").unwrap();
        assert_eq!(decoded, vec![0xfb, 0xff, 0xfe]);
    }

    #[test]
    fn base64_url_needs_padding() {
        // "_w" is URL-safe base64 of [0xff] — needs 2 padding chars
        let decoded = base64_decode_url("_w").unwrap();
        assert_eq!(decoded, vec![0xff]);
    }

    #[test]
    fn base64_decode_std_string_utf8() {
        let encoded = STANDARD.encode("test string");
        let decoded = base64_decode_std_string(&encoded).unwrap();
        assert_eq!(decoded, "test string");
    }

    #[test]
    fn url_decode_percent() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("%E4%BD%A0%E5%A5%BD"), "你好");
    }

    #[test]
    fn url_decode_passthrough() {
        assert_eq!(url_decode("no-encoding"), "no-encoding");
    }

    #[test]
    fn parse_bool_true_variants() {
        assert!(parse_bool("1"));
        assert!(parse_bool("true"));
        assert!(parse_bool("True"));
        assert!(parse_bool("TRUE"));
        assert!(parse_bool("yes"));
        assert!(parse_bool("YES"));
    }

    #[test]
    fn parse_bool_false_variants() {
        assert!(!parse_bool("0"));
        assert!(!parse_bool("false"));
        assert!(!parse_bool("no"));
        assert!(!parse_bool(""));
        assert!(!parse_bool("random"));
    }

    #[test]
    fn rand_user_agent_returns_valid() {
        let ua = rand_user_agent();
        assert!(ua.starts_with("Mozilla/5.0"));
    }

    #[test]
    fn pad_base64_no_padding_needed() {
        assert_eq!(pad_base64("AAAA"), "AAAA");
    }

    #[test]
    fn pad_base64_adds_padding() {
        assert_eq!(pad_base64("YQ"), "YQ==");
        assert_eq!(pad_base64("YWI"), "YWI=");
    }
}

/// Try standard base64 first, then URL-safe base64. Returns a UTF-8 string.
/// Convenience wrapper used by the parser modules.
pub fn base64_decode_auto(s: &str) -> Result<String, SubconvError> {
    base64_decode_std_string(s)
        .or_else(|_| base64_decode_url_string(s))
}

/// Collect URL query pairs into a map supporting multiple values per key.
pub fn query_pairs_multi(url: &url::Url) -> IndexMap<String, Vec<String>> {
    url.query_pairs()
        .fold(IndexMap::new(), |mut map, (k, v)| {
            map.entry(k.into_owned()).or_default().push(v.into_owned());
            map
        })
}

/// Get the first value for a key from a multi-value query map.
/// Returns `None` if the key is absent or its first value is empty.
pub fn query_first(map: &IndexMap<String, Vec<String>>, key: &str) -> String {
    map.get(key)
        .and_then(|v| v.first())
        .cloned()
        .unwrap_or_default()
}
