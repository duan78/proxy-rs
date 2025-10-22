//! Error handling utilities for proxy.rs

use thiserror::Error;

/// Custom error type for proxy.rs operations
#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("Network error: {0}")]
    Network(#[from] std::io::Error),

    #[error("DNS resolution failed: {0}")]
    DnsResolution(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("TLS error: {0}")]
    Tls(String),

    #[error("Connection timeout")]
    Timeout,

    #[error("Invalid proxy format: {0}")]
    InvalidFormat(String),

    #[error("DNSBL check failed: {0}")]
    DnsblError(String),

    #[error("Cache error: {0}")]
    CacheError(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Resource exhausted: {0}")]
    ResourceExhausted(String),
}

pub type ProxyResult<T> = Result<T, ProxyError>;

/// Trait for safe error handling
pub trait SafeUnwrap<T> {
    /// Safely unwrap with context
    fn safe_unwrap(self, context: &str) -> ProxyResult<T>;
}

impl<T> SafeUnwrap<T> for Option<T> {
    fn safe_unwrap(self, context: &str) -> ProxyResult<T> {
        self.ok_or_else(|| ProxyError::Config(format!("Expected value in {}", context)))
    }
}

impl<T, E> SafeUnwrap<T> for Result<T, E>
where
    E: Into<ProxyError>
{
    fn safe_unwrap(self, context: &str) -> ProxyResult<T> {
        self.map_err(|e| {
            let proxy_err: ProxyError = e.into();
            ProxyError::Config(format!("{}: {}", context, proxy_err))
        })
    }
}

/// Macro for logging errors safely
#[macro_export]
macro_rules! log_error {
    ($result:expr, $context:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                log::error!("{}: {}", $context, e);
                return Err(e);
            }
        }
    };
}

/// Macro for handling errors with fallback
#[macro_export]
macro_rules! handle_error {
    ($result:expr, $fallback:expr) => {
        match $result {
            Ok(value) => value,
            Err(e) => {
                log::warn!("Operation failed, using fallback: {}", e);
                $fallback
            }
        }
    };
}