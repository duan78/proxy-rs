//! API Routes - Define all REST API endpoints (simplified working version)

use crate::api::{
    handlers::*,
    middleware::{cors_layer},
};
use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use std::sync::Arc;

/// Create the main API router
pub fn create_api_router(
    config: Arc<crate::api::ApiConfig>,
    shared_config: crate::config::SharedConfig,
) -> Router {
    let auth_manager = Arc::new(crate::api::auth_simple::SimpleAuthManager::new());
    let rate_limiter = Arc::new(crate::api::middleware::RateLimiter::new(
        config.rate_limit,
        std::time::Duration::from_secs(60),
    ));

    // API routes
    let api_router = Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))
        .route("/stats", get(get_stats))

        // Proxy CRUD endpoints
        .route("/proxies", get(list_proxies).post(create_proxy))
        .route("/proxies/:id", get(get_proxy))

        // Configuration endpoints
        .route("/config", get(get_config).post(update_config))

        // Apply middleware
        .layer(middleware::from_fn_with_state(
            auth_manager,
            crate::api::middleware::simple_auth_middleware
        ))
        .layer(middleware::from_fn_with_state(
            rate_limiter,
            crate::api::middleware::rate_limit_middleware
        ))
        .layer(middleware::from_fn(crate::api::middleware::logging_middleware))
        .layer(cors_layer(&config))
        .with_state(shared_config);

    // Create main router with API and docs
    Router::new()
        .nest("/api/v1", api_router)
        .nest("/", create_docs_router())
        .route("/", get(root_info))
}

/// Create API documentation router (simplified)
pub fn create_docs_router() -> Router {
    Router::new()
        .route("/docs", get(swagger_ui))
        .route("/docs/openapi.json", get(openapi_json))
}

/// Swagger UI handler
async fn swagger_ui() -> axum::response::Html<String> {
    let html = include_str!("swagger.html");
    axum::response::Html(html.to_string())
}

/// OpenAPI JSON specification (simplified)
async fn openapi_json() -> axum::response::Json<serde_json::Value> {
    let openapi_spec = serde_json::json!({
        "openapi": "3.0.0",
        "info": {
            "title": "Proxy.rs API",
            "description": "High-performance proxy management API",
            "version": env!("CARGO_PKG_VERSION")
        },
        "servers": [
            {
                "url": "http://localhost:3000",
                "description": "Development server"
            }
        ],
        "paths": {
            "/health": {
                "get": {
                    "summary": "Health check",
                    "tags": ["Health"],
                    "responses": {
                        "200": {
                            "description": "Healthy response"
                        }
                    }
                }
            },
            "/proxies": {
                "get": {
                    "summary": "List proxies",
                    "tags": ["Proxies"],
                    "responses": {
                        "200": {
                            "description": "List of proxies"
                        }
                    }
                }
            }
        }
    });
    axum::response::Json(openapi_spec)
}

/// Root endpoint handler
async fn root_info() -> axum::response::Json<serde_json::Value> {
    axum::response::Json(serde_json::json!({
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
            "Simple authentication",
            "Rate limiting",
            "CORS support",
            "OpenAPI documentation"
        ]
    }))
}