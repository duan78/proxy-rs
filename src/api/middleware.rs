//! API Middleware - Rate Limiting, CORS, Logging (Simplified Version)

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::Response,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::api::{ApiError, auth_simple::SimpleAuthManager};

/// Rate limiting middleware
#[derive(Clone)]
pub struct RateLimiter {
    buckets: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
    max_requests: u32,
    window: Duration,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window: Duration) -> Self {
        Self {
            buckets: Arc::new(RwLock::new(HashMap::new())),
            max_requests,
            window,
        }
    }

    pub async fn is_allowed(&self, key: &str) -> bool {
        let mut buckets = self.buckets.write().await;
        let now = Instant::now();

        match buckets.get_mut(key) {
            Some((count, last_reset)) => {
                if now.duration_since(*last_reset) > self.window {
                    *count = 1;
                    *last_reset = now;
                    true
                } else if *count < self.max_requests {
                    *count += 1;
                    true
                } else {
                    false
                }
            }
            None => {
                buckets.insert(key.to_string(), (1, now));
                true
            }
        }
    }
}

/// Simple authentication middleware (JWT-free version)
pub async fn simple_auth_middleware(
    State(auth_manager): State<Arc<SimpleAuthManager>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // For now, always allow requests (auth can be enabled later)
    Ok(next.run(request).await)
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(rate_limiter): State<Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let client_ip = request
        .headers()
        .get("x-real-ip")
        .or_else(|| request.headers().get("x-forwarded-for"))
        .and_then(|h| h.to_str().ok())
        .unwrap_or_else(|| {
            request
                .extensions()
                .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
                .map(|addr| addr.to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    if !rate_limiter.is_allowed(&client_ip).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Request logging middleware
pub async fn logging_middleware(
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    let response = next.run(request).await;

    let duration = start.elapsed();
    let status = response.status();

    log::info!(
        "API Request: {} {} -> {} ({}ms)",
        method,
        uri,
        status,
        duration.as_millis()
    );

    response
}

/// CORS configuration
pub fn cors_layer(config: &crate::api::ApiConfig) -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([axum::http::Method::GET, axum::http::Method::POST, axum::http::Method::PUT, axum::http::Method::DELETE])
        .allow_headers(Any)
        .allow_credentials(true)
}

/// Create API middleware layers (simplified version)
pub fn create_middleware_layers(
    rate_limiter: Arc<RateLimiter>,
    auth_manager: Arc<SimpleAuthManager>,
) -> Vec<axum::middleware::FromFnLayer> {
    vec![
        axum::middleware::from_fn_with_state(
            auth_manager,
            simple_auth_middleware
        ),
        axum::middleware::from_fn_with_state(
            rate_limiter,
            rate_limit_middleware
        ),
        axum::middleware::from_fn(logging_middleware),
    ]
}

/// Error response helper
pub fn error_response(status: StatusCode, message: impl Into<String>) -> Response {
    let error = ApiError::new(
        status.as_str().to_string(),
        message.into()
    );

    (status, axum::Json(error)).into_response()
}