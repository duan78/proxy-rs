//! Minimal API Routes - Working version without complex middleware

use crate::api::handlers_minimal::*;
use axum::{
    routing::get,
    Router,
};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

/// Create the main API router (minimal working version)
pub fn create_api_router(
    _config: Arc<crate::api::ApiConfig>,
    shared_config: crate::config::SharedConfig,
) -> Router {
    let api_router = Router::new()
        // Health and status endpoints
        .route("/health", get(health_check))
        .route("/metrics", get(get_metrics))

        // Proxy endpoints
        .route("/proxies", get(list_proxies).post(create_proxy))
        .route("/proxies/:id", get(get_proxy))

        // Configuration endpoints
        .route("/config", get(get_config).post(update_config))

        // Apply basic middleware
        .layer(cors_layer())
        .with_state(shared_config);

    // Create main router
    Router::new()
        .nest("/api/v1", api_router)
        .nest("/", create_docs_router())
        .route("/", get(root_info))
}

/// CORS configuration
fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::ACCEPT,
            axum::http::header::CONTENT_TYPE,
        ])
        .allow_credentials(false)
}

/// Create API documentation router
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

/// OpenAPI JSON specification (minimal)
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