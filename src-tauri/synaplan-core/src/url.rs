//! Pairing-address validation. The Synaplan address must be HTTPS (or HTTP only
//! for a loopback host, for local development). The address is *pinned*: the
//! HTTP client that uses it refuses cross-host redirects (see [`crate::pairing`]
//! and [`crate::messages`]).

use thiserror::Error;
use url::Url;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum UrlError {
    #[error("Enter a Synaplan address.")]
    Empty,
    #[error("That does not look like a valid web address.")]
    Malformed,
    #[error("Use an https address.")]
    InsecureScheme,
    #[error("Enter only the address, without a path — for example https://web.synaplan.com")]
    HasPathOrQuery,
}

/// Validate and normalise a pairing address into `scheme://host[:port]`.
///
/// - A bare host (`web.synaplan.com`) is assumed to be `https://`.
/// - `http://` is accepted only for loopback (`localhost`, `127.0.0.1`, `::1`).
/// - Any path, query, or fragment beyond `/` is rejected so a copied deep link
///   cannot silently point pairing at the wrong endpoint.
pub fn validate_base_url(input: &str) -> Result<String, UrlError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(UrlError::Empty);
    }

    let with_scheme = if trimmed.contains("://") {
        trimmed.to_string()
    } else {
        format!("https://{trimmed}")
    };

    let url = Url::parse(&with_scheme).map_err(|_| UrlError::Malformed)?;

    let host = url.host_str().ok_or(UrlError::Malformed)?;
    if host.is_empty() {
        return Err(UrlError::Malformed);
    }

    match url.scheme() {
        "https" => {}
        "http" => {
            if !is_loopback_host(host) {
                return Err(UrlError::InsecureScheme);
            }
        }
        _ => return Err(UrlError::InsecureScheme),
    }

    if !url.path().is_empty() && url.path() != "/" {
        return Err(UrlError::HasPathOrQuery);
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(UrlError::HasPathOrQuery);
    }

    // Rebuild a clean origin (drops any trailing slash, userinfo, etc.).
    let mut normalized = format!("{}://{}", url.scheme(), host);
    if let Some(port) = url.port() {
        normalized.push_str(&format!(":{port}"));
    }
    Ok(normalized)
}

fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    // `url` wraps IPv6 hosts in brackets in host_str(); strip them for parsing.
    let bare = host.trim_start_matches('[').trim_end_matches(']');
    match bare.parse::<std::net::IpAddr>() {
        Ok(ip) => ip.is_loopback(),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_https() {
        assert_eq!(
            validate_base_url("https://web.synaplan.com").unwrap(),
            "https://web.synaplan.com"
        );
    }

    #[test]
    fn assumes_https_for_bare_host() {
        assert_eq!(
            validate_base_url("web.synaplan.com").unwrap(),
            "https://web.synaplan.com"
        );
    }

    #[test]
    fn strips_trailing_slash() {
        assert_eq!(
            validate_base_url("https://web.synaplan.com/").unwrap(),
            "https://web.synaplan.com"
        );
    }

    #[test]
    fn keeps_explicit_port() {
        assert_eq!(
            validate_base_url("http://localhost:8000").unwrap(),
            "http://localhost:8000"
        );
    }

    #[test]
    fn allows_http_only_for_loopback() {
        assert!(validate_base_url("http://localhost").is_ok());
        assert!(validate_base_url("http://127.0.0.1:8000").is_ok());
        assert_eq!(
            validate_base_url("http://example.com"),
            Err(UrlError::InsecureScheme)
        );
    }

    #[test]
    fn rejects_empty_and_junk() {
        assert_eq!(validate_base_url("   "), Err(UrlError::Empty));
        assert_eq!(validate_base_url("ftp://x"), Err(UrlError::InsecureScheme));
    }

    #[test]
    fn rejects_path_and_query() {
        assert_eq!(
            validate_base_url("https://web.synaplan.com/api/v1/desktop/pair"),
            Err(UrlError::HasPathOrQuery)
        );
        assert_eq!(
            validate_base_url("https://web.synaplan.com/?token=abc"),
            Err(UrlError::HasPathOrQuery)
        );
    }
}
