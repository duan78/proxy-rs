//! API Data Models - Request/Response structures for REST API

use crate::proxy::Proxy;
use crate::config::DynamicConfig;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::{DateTime, Utc};

/// Proxy information for API responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyInfo {
    pub id: String,
    pub host: String,
    pub port: u16,
    pub protocols: Vec<String>,
    pub country: String,
    pub anonymity_level: String,
    pub response_time_ms: Option<u64>,
    pub success_rate: f64,
    pub last_checked: Option<DateTime<Utc>>,
    pub is_working: bool,
    pub dnsbl_safe: bool,
    pub created_at: DateTime<Utc>,
    pub tags: Vec<String>,
}

impl From<Proxy> for ProxyInfo {
    fn from(proxy: Proxy) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            host: proxy.addr.ip().to_string(),
            port: proxy.addr.port(),
            protocols: proxy.expected_types.clone(),
            country: proxy.geo.iso_code.clone(),
            anonymity_level: "Anonymous".to_string(), // Could be enhanced
            response_time_ms: None, // Would need to be tracked
            success_rate: 0.0, // Would need to be calculated
            last_checked: None,
            is_working: true, // Would need to be tracked
            dnsbl_safe: true, // Would need to be tracked
            created_at: Utc::now(),
            tags: vec![],
        }
    }
}

/// Proxy creation request
#[derive(Debug, Deserialize)]
pub struct CreateProxyRequest {
    pub host: String,
    pub port: u16,
    pub protocols: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
}

/// Proxy update request
#[derive(Debug, Deserialize)]
pub struct UpdateProxyRequest {
    pub protocols: Option<Vec<String>>,
    pub tags: Option<Vec<String>>,
    pub notes: Option<String>,
}

/// Proxy filters for listing
#[derive(Debug, Deserialize)]
pub struct ProxyFilters {
    pub protocols: Option<Vec<String>>,
    pub countries: Option<Vec<String>>,
    pub is_working: Option<bool>,
    pub dnsbl_safe: Option<bool>,
    pub min_success_rate: Option<f64>,
    pub max_response_time_ms: Option<u64>,
    pub tags: Option<Vec<String>>,
}

/// Configuration update request
#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    pub section: String,
    pub config: serde_json::Value,
}

/// Metrics information
#[derive(Debug, Serialize, Deserialize)]
pub struct MetricsInfo {
    pub total_proxies: u64,
    pub working_proxies: u64,
    pub success_rate: f64,
    pub average_response_time_ms: f64,
    pub requests_per_second: f64,
    pub uptime_seconds: u64,
    pub memory_usage_mb: f64,
    pub cpu_usage_percent: f64,
    pub active_connections: u64,
    pub last_updated: DateTime<Utc>,
}

/// Health check response
#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheck {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub timestamp: DateTime<Utc>,
    pub checks: HashMap<String, HealthCheckResult>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub status: String,
    pub message: Option<String>,
    pub response_time_ms: Option<u64>,
}

/// Authentication request
#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

/// Authentication response
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub expires_in: u64,
    pub token_type: String,
}

/// API Error types
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
    pub message: String,
    pub details: Option<HashMap<String, String>>,
    pub timestamp: DateTime<Utc>,
}

impl ApiError {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: None,
            timestamp: Utc::now(),
        }
    }

    pub fn with_details(error: impl Into<String>, message: impl Into<String>, details: HashMap<String, String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
            details: Some(details),
            timestamp: Utc::now(),
        }
    }
}

/// Statistics summary
#[derive(Debug, Serialize)]
pub struct StatsSummary {
    pub total_proxies: u64,
    pub by_country: HashMap<String, u64>,
    pub by_protocol: HashMap<String, u64>,
    pub by_anonymity: HashMap<String, u64>,
    pub performance: PerformanceStats,
}

#[derive(Debug, Serialize)]
pub struct PerformanceStats {
    pub avg_response_time_ms: f64,
    pub success_rate: f64,
    pub requests_per_minute: f64,
    pub error_rate: f64,
}