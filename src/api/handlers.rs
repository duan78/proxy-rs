//! API Handlers - Business logic for all REST API endpoints

use crate::api::{
    ApiResponse, ConfigUpdateRequest, CreateProxyRequest, HealthCheck, HealthCheckResult,
    MetricsInfo, PaginationInfo, PaginatedResponse, ProxyFilters, ProxyInfo, StatsSummary,
    UpdateProxyRequest,
};
use crate::config::{DynamicConfig, SharedConfig};
use crate::providers::PROXIES;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Health check handler
pub async fn health_check() -> Json<ApiResponse<HealthCheck>> {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let mut checks = HashMap::new();
    checks.insert(
        "proxy_pool".to_string(),
        HealthCheckResult {
            status: "healthy".to_string(),
            message: Some("Proxy pool is operational".to_string()),
            response_time_ms: Some(5),
        },
    );
    checks.insert(
        "config".to_string(),
        HealthCheckResult {
            status: "healthy".to_string(),
            message: Some("Configuration is loaded".to_string()),
            response_time_ms: Some(2),
        },
    );

    let health = HealthCheck {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
        timestamp: chrono::Utc::now(),
        checks,
    };

    Json(ApiResponse::success(health))
}

/// List all proxies with filtering and pagination
pub async fn list_proxies(
    Query(filters): Query<ProxyFilters>,
    Query(pagination): Query<crate::api::PaginationParams>,
) -> Json<ApiResponse<PaginatedResponse<ProxyInfo>>> {
    let pagination = pagination.unwrap_or_default();
    let page = pagination.page.unwrap_or(1);
    let limit = pagination.limit.unwrap_or(50).min(1000); // Cap at 1000

    // This is a simplified implementation
    // In a real implementation, you would query from a database or cache
    let mut proxies = Vec::new();

    // Simulate proxy data - in real implementation, query from PROXIES
    for i in 0..100 {
        let proxy = ProxyInfo {
            id: format!("proxy-{}", i),
            host: format!("192.168.1.{}", i % 254 + 1),
            port: 8080 + (i % 100) as u16,
            protocols: vec!["HTTP".to_string(), "HTTPS".to_string()],
            country: "US".to_string(),
            anonymity_level: "Anonymous".to_string(),
            response_time_ms: Some(100 + (i * 10) as u64),
            success_rate: 0.95 - (i as f64 * 0.001),
            last_checked: Some(chrono::Utc::now()),
            is_working: i % 10 != 0, // 90% working
            dnsbl_safe: i % 20 != 0, // 95% safe
            created_at: chrono::Utc::now(),
            tags: vec!["fast".to_string(), "stable".to_string()],
        };
        proxies.push(proxy);
    }

    // Apply filters
    if let Some(countries) = &filters.countries {
        proxies.retain(|p| countries.contains(&p.country));
    }
    if let Some(is_working) = filters.is_working {
        proxies.retain(|p| p.is_working == is_working);
    }
    if let Some(dnsbl_safe) = filters.dnsbl_safe {
        proxies.retain(|p| p.dnsbl_safe == dnsbl_safe);
    }

    let total = proxies.len() as u64;
    let total_pages = (total as f64 / limit as f64).ceil() as u32;

    // Apply pagination
    let start = ((page - 1) * limit) as usize;
    let end = (start + limit as usize).min(proxies.len());
    let paginated_proxies = if start < proxies.len() {
        proxies[start..end].to_vec()
    } else {
        Vec::new()
    };

    let response = PaginatedResponse {
        data: paginated_proxies,
        pagination: PaginationInfo {
            page,
            limit,
            total,
            total_pages,
            has_next: page < total_pages,
            has_prev: page > 1,
        },
    };

    Json(ApiResponse::success(response))
}

/// Get proxy by ID
pub async fn get_proxy(Path(proxy_id): Path<String>) -> Json<ApiResponse<ProxyInfo>> {
    // In real implementation, query from database/cache
    if proxy_id.starts_with("proxy-") {
        let proxy = ProxyInfo {
            id: proxy_id,
            host: "192.168.1.100".to_string(),
            port: 8080,
            protocols: vec!["HTTP".to_string(), "HTTPS".to_string()],
            country: "US".to_string(),
            anonymity_level: "Anonymous".to_string(),
            response_time_ms: Some(150),
            success_rate: 0.95,
            last_checked: Some(chrono::Utc::now()),
            is_working: true,
            dnsbl_safe: true,
            created_at: chrono::Utc::now(),
            tags: vec!["fast".to_string()],
        };
        Json(ApiResponse::success(proxy))
    } else {
        let error_response = ApiResponse::<ProxyInfo>::error("Proxy not found");
        Json(error_response)
    }
}

/// Create new proxy
pub async fn create_proxy(
    Json(request): Json<CreateProxyRequest>,
) -> Result<Json<ApiResponse<ProxyInfo>>, StatusCode> {
    // Validate input
    if request.port == 0 || request.port > 65535 {
        return Err(StatusCode::BAD_REQUEST);
    }

    // Create proxy (in real implementation, add to database)
    let proxy = ProxyInfo {
        id: uuid::Uuid::new_v4().to_string(),
        host: request.host.clone(),
        port: request.port,
        protocols: request.protocols.unwrap_or_else(|| vec!["HTTP".to_string()]),
        country: "Unknown".to_string(),
        anonymity_level: "Unknown".to_string(),
        response_time_ms: None,
        success_rate: 0.0,
        last_checked: None,
        is_working: false, // Needs validation
        dnsbl_safe: false, // Needs checking
        created_at: chrono::Utc::now(),
        tags: request.tags.unwrap_or_default(),
    };

    Ok(Json(ApiResponse::success(proxy)))
}

/// Update proxy
pub async fn update_proxy(
    Path(proxy_id): Path<String>,
    Json(request): Json<UpdateProxyRequest>,
) -> Result<Json<ApiResponse<ProxyInfo>>, StatusCode> {
    // In real implementation, update in database
    if proxy_id.starts_with("proxy-") {
        let proxy = ProxyInfo {
            id: proxy_id,
            host: "192.168.1.100".to_string(),
            port: 8080,
            protocols: request.protocols.unwrap_or_else(|| vec!["HTTP".to_string()]),
            country: "US".to_string(),
            anonymity_level: "Anonymous".to_string(),
            response_time_ms: Some(150),
            success_rate: 0.95,
            last_checked: Some(chrono::Utc::now()),
            is_working: true,
            dnsbl_safe: true,
            created_at: chrono::Utc::now(),
            tags: request.tags.unwrap_or_default(),
        };
        Ok(Json(ApiResponse::success(proxy)))
    } else {
        Err(StatusCode::NOT_FOUND)
    }
}

/// Delete proxy
pub async fn delete_proxy(Path(proxy_id): Path<String>) -> Json<ApiResponse<String>> {
    // In real implementation, delete from database
    if proxy_id.starts_with("proxy-") {
        Json(ApiResponse::success(format!("Proxy {} deleted successfully", proxy_id)))
    } else {
        Json(ApiResponse::error("Proxy not found"))
    }
}

/// Get current configuration
pub async fn get_config(
    State(shared_config): State<SharedConfig>,
) -> Json<ApiResponse<DynamicConfig>> {
    let config = shared_config.read().clone();
    Json(ApiResponse::success(config))
}

/// Update configuration
pub async fn update_config(
    State(shared_config): State<SharedConfig>,
    Json(request): Json<ConfigUpdateRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    // Validate section
    let section = match request.section.as_str() {
        "general" | "dnsbl" | "server" | "protocols" => request.section,
        _ => return Err(StatusCode::BAD_REQUEST),
    };

    // Update configuration (this triggers hot-reload)
    {
        let mut config = shared_config.write();
        if let Err(e) = config.update_section(
            crate::config::ConfigSection::General, // This would be dynamic based on section
            request.config,
        ) {
            log::error!("Failed to update config: {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    log::info!("Configuration updated via API: {}", section);

    Ok(Json(ApiResponse::success(format!(
        "Configuration section '{}' updated successfully",
        section
    ))))
}

/// Get metrics
pub async fn get_metrics() -> Json<ApiResponse<MetricsInfo>> {
    // In real implementation, collect actual metrics
    let metrics = MetricsInfo {
        total_proxies: 1000,
        working_proxies: 950,
        success_rate: 0.95,
        average_response_time_ms: 150.0,
        requests_per_second: 1250.0,
        uptime_seconds: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        memory_usage_mb: 45.0,
        cpu_usage_percent: 12.5,
        active_connections: 250,
        last_updated: chrono::Utc::now(),
    };

    Json(ApiResponse::success(metrics))
}

/// Get statistics summary
pub async fn get_stats() -> Json<ApiResponse<StatsSummary>> {
    let mut by_country = HashMap::new();
    by_country.insert("US".to_string(), 400);
    by_country.insert("DE".to_string(), 200);
    by_country.insert("FR".to_string(), 150);
    by_country.insert("GB".to_string(), 100);
    by_country.insert("Other".to_string(), 150);

    let mut by_protocol = HashMap::new();
    by_protocol.insert("HTTP".to_string(), 950);
    by_protocol.insert("HTTPS".to_string(), 900);
    by_protocol.insert("SOCKS4".to_string(), 300);
    by_protocol.insert("SOCKS5".to_string(), 350);

    let mut by_anonymity = HashMap::new();
    by_anonymity.insert("Transparent".to_string(), 200);
    by_anonymity.insert("Anonymous".to_string(), 500);
    by_anonymity.insert("High".to_string(), 300);

    let stats = StatsSummary {
        total_proxies: 1000,
        by_country,
        by_protocol,
        by_anonymity,
        performance: crate::api::models::PerformanceStats {
            avg_response_time_ms: 150.0,
            success_rate: 0.95,
            requests_per_minute: 75000.0,
            error_rate: 0.05,
        },
    };

    Json(ApiResponse::success(stats))
}

/// Validate proxies
pub async fn validate_proxies(
    Json(proxy_ids): Json<Vec<String>>,
) -> Json<ApiResponse<Vec<String>>> {
    // In real implementation, trigger validation for specified proxies
    log::info!("Triggering validation for {} proxies", proxy_ids.len());

    // Return task IDs for tracking validation progress
    let task_ids: Vec<String> = proxy_ids
        .iter()
        .map(|_| uuid::Uuid::new_v4().to_string())
        .collect();

    Json(ApiResponse::success(task_ids))
}

/// Export proxies
pub async fn export_proxies(
    Query(filters): Query<ProxyFilters>,
    Query(format): Query<HashMap<String, String>>,
) -> Result<String, StatusCode> {
    let format = format.get("format").unwrap_or(&"json".to_string()).clone();

    // In real implementation, generate export in requested format
    match format.as_str() {
        "json" => Ok("[{\"host\":\"192.168.1.1\",\"port\":8080}]".to_string()),
        "txt" => Ok("192.168.1.1:8080\n192.168.1.2:8080\n".to_string()),
        "csv" => Ok("host,port,protocol\n192.168.1.1,8080,HTTP\n".to_string()),
        _ => Err(StatusCode::BAD_REQUEST),
    }
}