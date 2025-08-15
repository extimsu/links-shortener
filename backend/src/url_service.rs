//! URL Shortening Service Module
//!
//! Provides core functionality for creating and managing short URLs, including
//! short code generation, storage in PostgreSQL, and Redis caching integration.

use sqlx::PgPool;
use redis::Client as RedisClient;
use thiserror::Error;
use url::Url;

/// Default short code length
const DEFAULT_CODE_LENGTH: usize = 6;
/// Base62 character set (0-9, a-z, A-Z)
const BASE62_CHARSET: &[u8] = b"0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
/// Maximum allowed URL length
const MAX_URL_LENGTH: usize = 2048;
/// Example disallowed domains (can be expanded/configured)
const DISALLOWED_DOMAINS: &[&str] = &["localhost", "127.0.0.1", "::1"];

/// Service for URL shortening operations.
pub struct UrlService {
    pub db_pool: PgPool,
    pub redis_client: RedisClient,
}

impl UrlService {
    /// Creates a new UrlService instance.
    pub fn new(db_pool: PgPool, redis_client: RedisClient) -> Self {
        Self { db_pool, redis_client }
    }

    pub fn new_dummy() -> Self {
        let db_pool = sqlx::PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let redis_client = redis::Client::open("redis://127.0.0.1/").unwrap();
        UrlService::new(db_pool, redis_client)
    }

    /// Generate a random base62 short code of the given length
    pub fn generate_short_code(&self, length: usize) -> String {
        use rand::{thread_rng, Rng};
        let mut rng = thread_rng();
        (0..length)
            .map(|_| {
                let idx = rng.gen_range(0..BASE62_CHARSET.len());
                BASE62_CHARSET[idx] as char
            })
            .collect()
    }

    /// Check if a short code is unique in the database (stub, to be implemented)
    pub async fn is_code_unique(&self, _code: &str) -> Result<bool, UrlServiceError> {
        // TODO: Query PostgreSQL for existence of the code
        Ok(true)
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
        let mut normalized = parsed.to_string(); // Use to_string() instead of into_string()
        if let Some(h) = host {
            normalized = normalized.replace(parsed.host_str().unwrap_or(""), &h);
        }
        normalized = normalized.replace(parsed.scheme(), &scheme);
        Ok(normalized)
    }

    /// Extract the domain from a URL string
    pub fn extract_domain(&self, url_str: &str) -> Result<String, UrlServiceError> {
        let parsed = Url::parse(url_str).map_err(|_| UrlServiceError::InvalidUrl("Malformed URL".into()))?;
        Ok(parsed.host_str().unwrap_or("").to_string())
    }
}

/// Errors that can occur during URL shortening operations.
#[derive(Debug, Error)]
pub enum UrlServiceError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("Redis error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
    #[error("Other error: {0}")]
    Other(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_short_code_length() {
        let dummy_pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let dummy_redis = RedisClient::open("redis://127.0.0.1/").unwrap();
        let service = UrlService::new(dummy_pool, dummy_redis);
        let code = service.generate_short_code(DEFAULT_CODE_LENGTH);
        assert_eq!(code.len(), DEFAULT_CODE_LENGTH);
    }

    #[tokio::test]
    async fn test_generate_short_code_charset() {
        let dummy_pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let dummy_redis = RedisClient::open("redis://127.0.0.1/").unwrap();
        let service = UrlService::new(dummy_pool, dummy_redis);
        let code = service.generate_short_code(DEFAULT_CODE_LENGTH);
        for c in code.chars() {
            assert!(BASE62_CHARSET.contains(&(c as u8)), "Invalid char: {}", c);
        }
    }

    #[tokio::test]
    async fn test_validate_url_valid() {
        let dummy_pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let dummy_redis = RedisClient::open("redis://127.0.0.1/").unwrap();
        let service = UrlService::new(dummy_pool, dummy_redis);
        assert!(service.validate_url("https://example.com").is_ok());
        assert!(service.validate_url("http://example.com/path?query=1").is_ok());
    }

    #[tokio::test]
    async fn test_validate_url_invalid() {
        let dummy_pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let dummy_redis = RedisClient::open("redis://127.0.0.1/").unwrap();
        let service = UrlService::new(dummy_pool, dummy_redis);
        assert!(service.validate_url("").is_err());
        assert!(service.validate_url("ftp://example.com").is_err());
        assert!(service.validate_url("localhost").is_err());
        let long_url = "http://".to_string() + &"a".repeat(MAX_URL_LENGTH + 1);
        assert!(service.validate_url(&long_url).is_err());
    }

    #[tokio::test]
    async fn test_normalize_url() {
        let dummy_pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let dummy_redis = RedisClient::open("redis://127.0.0.1/").unwrap();
        let service = UrlService::new(dummy_pool, dummy_redis);
        let url = "HTTP://EXAMPLE.COM:80/path/";
        let norm = service.normalize_url(url).unwrap();
        assert!(norm.starts_with("http://example.com/path"));
    }

    #[tokio::test]
    async fn test_extract_domain() {
        let dummy_pool = PgPool::connect_lazy("postgres://user:pass@localhost/db").unwrap();
        let dummy_redis = RedisClient::open("redis://127.0.0.1/").unwrap();
        let service = UrlService::new(dummy_pool, dummy_redis);
        let url = "https://sub.example.com/path";
        let domain = service.extract_domain(url).unwrap();
        assert_eq!(domain, "sub.example.com");
    }
}
