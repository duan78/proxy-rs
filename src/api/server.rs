//! API Server - Main REST API server (minimal working version)

use crate::api::{ApiConfig, routes_minimal::*};
use crate::config::SharedConfig;
use axum::{
    extract::Json,
    routing::get,
    Router,
};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;

/// High-performance API server
pub struct ApiServer {
    config: Arc<ApiConfig>,
    shared_config: SharedConfig,
    app: Router,
}

impl ApiServer {
    /// Create new API server instance
    pub fn new(config: ApiConfig, shared_config: SharedConfig) -> Self {
        let config = Arc::new(config);
        let app = Self::create_app(config.clone(), shared_config.clone());

        Self {
            config,
            shared_config,
            app,
        }
    }

    /// Create Axum application with all routes and middleware
    fn create_app(config: Arc<ApiConfig>, shared_config: SharedConfig) -> Router {
        // API routes
        let api_router = create_api_router(config.clone(), shared_config);

        // Combine all routers
        Router::new()
            .nest("/api/v1", api_router)
            .nest("/", create_docs_router())
            // Root endpoint with API info
            .route("/", get(root_info))
    }

    /// Start the API server
    pub async fn start(self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.config.port));
        let listener = TcpListener::bind(addr).await?;

        log::info!(
            "🚀 API Server starting on http://{}:{}/api/v1",
            self.config.host,
            self.config.port
        );
        log::info!("📚 API Documentation: http://{}:{}/docs", self.config.host, self.config.port);
        log::info!("🔑 Authentication: {}", if self.config.enable_auth { "Enabled" } else { "Disabled" });
        log::info!("⚡ Rate limiting: {} requests/minute", self.config.rate_limit);

        axum::serve(listener, self.app).await?;

        Ok(())
    }

    /// Get server configuration
    pub fn config(&self) -> &ApiConfig {
        &self.config
    }

    /// Get shared configuration reference
    pub fn shared_config(&self) -> &SharedConfig {
        &self.shared_config
    }
}

/// Root endpoint handler
async fn root_info() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "name": "Proxy.rs API",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "High-performance proxy management API",
        "endpoints": {
            "api": "/api/v1",
            "docs": "/docs",
            "health": "/api/v1/health",
            "proxies": "/api/v1/proxies",
            "config": "/api/v1/config",
            "metrics": "/api/v1/metrics"
        },
        "features": [
            "High-performance async/await",
            "Hot-reload configuration",
            "Real-time metrics",
            "JWT authentication",
            "Rate limiting",
            "CORS support",
            "OpenAPI documentation"
        ]
    }))
}

/// 404 Not Found handler
async fn not_found_handler() -> Json<crate::api::ApiResponse<String>> {
    Json(crate::api::ApiResponse::error("Endpoint not found"))
}

/// Health check for the API server itself
pub async fn api_health_check() -> Json<crate::api::ApiResponse<serde_json::Value>> {
    let uptime = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let health = serde_json::json!({
        "status": "healthy",
        "service": "proxy-rs-api",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": uptime,
        "timestamp": chrono::Utc::now(),
        "features": {
            "hot_reload": true,
            "authentication": true,
            "rate_limiting": true,
            "metrics": true,
            "documentation": true
        }
    });

    Json(crate::api::ApiResponse::success(health))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DynamicConfig;

    #[tokio::test]
    async fn test_api_server_creation() {
        let config = ApiConfig::default();
        let shared_config = Arc::new(parking_lot::RwLock::new(DynamicConfig::new()));

        let server = ApiServer::new(config, shared_config);

        assert_eq!(server.config().port, 3000);
        assert!(!server.config().enable_auth);
    }

    #[tokio::test]
    async fn test_root_info() {
        let response = root_info().await;
        let info = response.0;

        assert_eq!(info["name"], "Proxy.rs API");
        assert_eq!(info["version"], env!("CARGO_PKG_VERSION"));
        assert!(info["endpoints"]["api"].is_string());
        assert!(info["features"].is_array());
    }
}

/// Create and start API server with default configuration
pub async fn start_default_api_server(shared_config: SharedConfig) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = ApiConfig::default();
    let server = ApiServer::new(config, shared_config);
    server.start().await
}

/// Create and start API server with custom configuration
pub async fn start_api_server_with_config(
    config: ApiConfig,
    shared_config: SharedConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let server = ApiServer::new(config, shared_config);
    server.start().await
}