//! URL Shortening Service Module
//!
//! Provides core functionality for creating and managing short URLs, including
//! URL validation/normalization.

use thiserror::Error;
use url::Url;

/// Maximum allowed URL length
const MAX_URL_LENGTH: usize = 2048;
/// Example disallowed domains (can be expanded/configured)
const DISALLOWED_DOMAINS: &[&str] = &["localhost", "127.0.0.1", "::1"];

pub struct UrlService;

impl UrlService {
    pub fn new_dummy() -> Self {
        UrlService
    }

    /// Validate a URL string for format, protocol, length, and disallowed domains
    pub fn validate_url(&self, url_str: &str) -> Result<(), UrlServiceError> {
        let trimmed = url_str.trim();
        if trimmed.is_empty() {
            return Err(UrlServiceError::InvalidUrl("URL is empty".into()));
        }
        if trimmed.len() > MAX_URL_LENGTH {
            return Err(UrlServiceError::InvalidUrl("URL is too long".into()));
        }
        let parsed = Url::parse(trimmed).map_err(|_| UrlServiceError::InvalidUrl("Malformed URL".into()))?;
        let scheme = parsed.scheme().to_ascii_lowercase();
        if scheme != "http" && scheme != "https" {
            return Err(UrlServiceError::InvalidUrl("URL must use http or https".into()));
        }
        let host = parsed.host_str().unwrap_or("").to_ascii_lowercase();
        for &bad in DISALLOWED_DOMAINS {
            if host == bad {
                return Err(UrlServiceError::InvalidUrl("Disallowed domain".into()));
            }
        }
        Ok(())
    }

    /// Normalize a URL string (lowercase scheme/host, remove default ports, trailing slash, etc.)
    pub fn normalize_url(&self, url_str: &str) -> Result<String, UrlServiceError> {
        let mut parsed = Url::parse(url_str).map_err(|_| UrlServiceError::InvalidUrl("Malformed URL".into()))?;
        // Lowercase scheme and host
        let scheme = parsed.scheme().to_ascii_lowercase();
        let host = parsed.host_str().map(|h| h.to_ascii_lowercase());
        // Remove default ports
        if (scheme == "http" && parsed.port_or_known_default() == Some(80)) ||
           (scheme == "https" && parsed.port_or_known_default() == Some(443)) {
            parsed.set_port(None).ok();
        }
        // Remove trailing slash (except for root)
        let mut path = parsed.path().to_string();
        if path != "/" && path.ends_with('/') {
            path.pop();
            parsed.set_path(&path);
        }
        // Rebuild URL with normalized scheme/host
        let mut normalized = parsed.to_string();
        if let Some(h) = host {
            normalized = normalized.replace(parsed.host_str().unwrap_or(""), &h);
        }
        normalized = normalized.replace(parsed.scheme(), &scheme);
        Ok(normalized)
    }
}

#[derive(Debug, Error)]
pub enum UrlServiceError {
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_url_valid() {
        let service = UrlService;
        assert!(service.validate_url("https://example.com").is_ok());
        assert!(service.validate_url("http://example.com/path?query=1").is_ok());
    }

    #[test]
    fn test_validate_url_invalid() {
        let service = UrlService;
        assert!(service.validate_url("").is_err());
        assert!(service.validate_url("ftp://example.com").is_err());
        assert!(service.validate_url("localhost").is_err());
        let long_url = "http://".to_string() + &"a".repeat(MAX_URL_LENGTH + 1);
        assert!(service.validate_url(&long_url).is_err());
    }

    #[test]
    fn test_normalize_url() {
        let service = UrlService;
        let url = "HTTP://EXAMPLE.COM:80/path/";
        let norm = service.normalize_url(url).unwrap();
        assert!(norm.starts_with("http://example.com/path"));
    }
}
