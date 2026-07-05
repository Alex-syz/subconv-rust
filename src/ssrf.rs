//! SSRF (Server-Side Request Forgery) protection.
//!
//! This module provides utilities to validate that URLs accessed by the server
//! (e.g., for fetching remote templates or rule files) do not point to internal
//! network addresses.

use std::net::{Ipv4Addr, Ipv6Addr};

use url::Url;

use crate::SubconvError;

/// Check if a URL string points to a private/internal network address.
///
/// Returns `true` if the URL should be blocked to prevent SSRF attacks.
///
/// # Checks performed
/// - Scheme must be `http` or `https`
/// - Host cannot be `localhost`, `127.0.0.1`, or `::1`
/// - Host cannot match RFC 1918 private ranges (10.x, 172.16-31.x, 192.168.x)
/// - Host cannot match link-local ranges (169.254.x, fe80::)
/// - Host cannot be `0.0.0.0`
pub fn is_private_url(url_str: &str) -> bool {
    validate_remote_url(url_str).is_err()
}

/// Validate that a remote URL is safe to access.
///
/// Returns the parsed `Url` if safe, or an error describing why it was rejected.
pub fn validate_remote_url(url_str: &str) -> Result<Url, SubconvError> {
    let url =
        Url::parse(url_str).map_err(|e| SubconvError::InvalidUrl(format!("invalid URL: {e}")))?;

    // Scheme must be http or https.
    match url.scheme() {
        "http" | "https" => {}
        other => {
            return Err(SubconvError::InvalidUrl(format!(
                "unsupported scheme: {other}"
            )));
        }
    }

    // Extract host via url::Host which correctly handles IPv6 brackets.
    let host = url
        .host()
        .ok_or_else(|| SubconvError::InvalidUrl("URL has no host".into()))?;

    match host {
        url::Host::Domain(domain) => {
            // Block localhost by name.
            if domain.eq_ignore_ascii_case("localhost") {
                return Err(SubconvError::Forbidden("URL points to localhost".into()));
            }
            // Other domain names are allowed through.
            // DNS rebinding attacks are mitigated by reqwest's redirect policy.
        }
        url::Host::Ipv4(ip) => {
            if is_private_ipv4(&ip) {
                return Err(SubconvError::Forbidden(format!(
                    "URL points to private IP: {ip}"
                )));
            }
        }
        url::Host::Ipv6(ip) => {
            if is_private_ipv6(&ip) {
                return Err(SubconvError::Forbidden(format!(
                    "URL points to private IP: {ip}"
                )));
            }
        }
    }

    Ok(url)
}

/// Check if an IPv4 address is private/internal.
///
/// Covers:
/// - Loopback: 127.0.0.0/8
/// - Link-local: 169.254.0.0/16
/// - RFC 1918 private: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16
/// - Unspecified: 0.0.0.0
fn is_private_ipv4(ip: &Ipv4Addr) -> bool {
    let octets = ip.octets();

    // 0.0.0.0 (unspecified)
    if ip.is_unspecified() {
        return true;
    }

    // 127.0.0.0/8 (loopback)
    if octets[0] == 127 {
        return true;
    }

    // 10.0.0.0/8 (private)
    if octets[0] == 10 {
        return true;
    }

    // 172.16.0.0/12 (private: 172.16.x.x - 172.31.x.x)
    if octets[0] == 172 && (16..=31).contains(&octets[1]) {
        return true;
    }

    // 192.168.0.0/16 (private)
    if octets[0] == 192 && octets[1] == 168 {
        return true;
    }

    // 169.254.0.0/16 (link-local)
    if octets[0] == 169 && octets[1] == 254 {
        return true;
    }

    false
}

/// Check if an IPv6 address is private/internal.
///
/// Covers:
/// - Loopback: ::1
/// - Link-local: fe80::/10
/// - Unique local: fc00::/7 (RFC 4193)
/// - Unspecified: ::
fn is_private_ipv6(ip: &Ipv6Addr) -> bool {
    // :: (unspecified)
    if ip.is_unspecified() {
        return true;
    }

    // ::1 (loopback)
    if ip.is_loopback() {
        return true;
    }

    let segments = ip.segments();

    // fe80::/10 (link-local)
    // fe80 = 0xfe80, mask with 0xffc0 gives fe80-febf
    if (segments[0] & 0xffc0) == 0xfe80 {
        return true;
    }

    // fc00::/7 (unique local / private)
    // fc00-fdff: first byte is 0xfc or 0xfd
    if (segments[0] & 0xfe00) == 0xfc00 {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_private_ipv4() {
        // Loopback
        assert!(is_private_ipv4(&Ipv4Addr::new(127, 0, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(127, 255, 255, 255)));

        // 10.x.x.x
        assert!(is_private_ipv4(&Ipv4Addr::new(10, 0, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(10, 255, 255, 255)));

        // 172.16-31.x.x
        assert!(is_private_ipv4(&Ipv4Addr::new(172, 16, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(172, 31, 255, 255)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(172, 15, 0, 1)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(172, 32, 0, 1)));

        // 192.168.x.x
        assert!(is_private_ipv4(&Ipv4Addr::new(192, 168, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(192, 168, 255, 255)));

        // Link-local
        assert!(is_private_ipv4(&Ipv4Addr::new(169, 254, 0, 1)));
        assert!(is_private_ipv4(&Ipv4Addr::new(169, 254, 255, 255)));

        // Unspecified
        assert!(is_private_ipv4(&Ipv4Addr::new(0, 0, 0, 0)));

        // Public IPs should pass
        assert!(!is_private_ipv4(&Ipv4Addr::new(8, 8, 8, 8)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(1, 1, 1, 1)));
        assert!(!is_private_ipv4(&Ipv4Addr::new(203, 0, 113, 1)));
    }

    #[test]
    fn test_private_ipv6() {
        // Loopback
        assert!(is_private_ipv6(&Ipv6Addr::LOCALHOST));

        // Link-local fe80::/10
        assert!(is_private_ipv6(&Ipv6Addr::new(0xfe80, 0, 0, 0, 0, 0, 0, 1)));
        assert!(is_private_ipv6(&Ipv6Addr::new(
            0xfebf, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
        )));

        // Unique local fc00::/7
        assert!(is_private_ipv6(&Ipv6Addr::new(0xfc00, 0, 0, 0, 0, 0, 0, 1)));
        assert!(is_private_ipv6(&Ipv6Addr::new(
            0xfdff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff, 0xffff
        )));

        // Unspecified
        assert!(is_private_ipv6(&Ipv6Addr::UNSPECIFIED));

        // Public IPv6 should pass
        assert!(!is_private_ipv6(&Ipv6Addr::new(
            0x2001, 0x4860, 0x4860, 0, 0, 0, 0, 0x8888
        ))); // Google DNS
    }

    #[test]
    fn test_validate_remote_url() {
        // Valid public URLs
        assert!(validate_remote_url("https://example.com/path").is_ok());
        assert!(validate_remote_url("http://8.8.8.8/dns-query").is_ok());

        // Invalid scheme
        assert!(validate_remote_url("ftp://example.com/file").is_err());
        assert!(validate_remote_url("file:///etc/passwd").is_err());

        // Localhost blocked
        assert!(validate_remote_url("http://localhost/admin").is_err());
        assert!(validate_remote_url("http://127.0.0.1/admin").is_err());
        assert!(validate_remote_url("http://[::1]/admin").is_err());

        // Private IPs blocked
        assert!(validate_remote_url("http://10.0.0.1/").is_err());
        assert!(validate_remote_url("http://172.16.0.1/").is_err());
        assert!(validate_remote_url("http://192.168.1.1/").is_err());
        assert!(validate_remote_url("http://169.254.1.1/").is_err());
        assert!(validate_remote_url("http://0.0.0.0/").is_err());

        // IPv6 private blocked
        assert!(validate_remote_url("http://[fe80::1]/").is_err());
        assert!(validate_remote_url("http://[fc00::1]/").is_err());

        // Malformed URL
        assert!(validate_remote_url("not a url").is_err());
    }

    #[test]
    fn test_is_private_url() {
        assert!(!is_private_url("https://example.com"));
        assert!(is_private_url("http://localhost"));
        assert!(is_private_url("http://127.0.0.1"));
        assert!(is_private_url("http://10.0.0.1"));
    }
}
