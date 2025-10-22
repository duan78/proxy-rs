//! Simplified Authentication Module - Basic API key authentication (Windows ARM64 compatible)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use chrono::Utc;

/// Simple API key authentication (JWT-free version)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_used: Option<chrono::DateTime<chrono::Utc>>,
    pub permissions: Vec<String>,
}

/// Simple authentication manager
#[derive(Clone)]
pub struct SimpleAuthManager {
    api_keys: Arc<tokio::sync::RwLock<HashMap<String, ApiKey>>>,
    default_key: String,
}

impl SimpleAuthManager {
    pub fn new() -> Self {
        let mut keys = HashMap::new();

        // Add default API key
        let default_key = ApiKey {
            key: "proxy-rs-api-key-default".to_string(),
            name: "Default API Key".to_string(),
            created_at: Utc::now(),
            last_used: None,
            permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        };
        keys.insert(default_key.key.clone(), default_key.clone());

        Self {
            api_keys: Arc::new(tokio::sync::RwLock::new(keys)),
            default_key: default_key.key,
        }
    }

    pub fn default_key(&self) -> &str {
        &self.default_key
    }

    pub async fn validate_key(&self, key: &str) -> bool {
        let keys = self.api_keys.read().await;
        keys.contains_key(key)
    }

    pub async fn update_last_used(&self, key: &str) {
        let mut keys = self.api_keys.write().await;
        if let Some(api_key) = keys.get_mut(key) {
            api_key.last_used = Some(Utc::now());
        }
    }

    pub async fn create_key(&self, name: String, permissions: Vec<String>) -> String {
        let uuid_str = uuid::Uuid::new_v4().to_string();
        let new_key = format!("proxy-rs-{}", &uuid_str[..8]);

        let api_key = ApiKey {
            key: new_key.clone(),
            name,
            created_at: Utc::now(),
            last_used: None,
            permissions,
        };

        let mut keys = self.api_keys.write().await;
        keys.insert(new_key.clone(), api_key);

        new_key
    }
}

/// Simple authentication request
#[derive(Debug, Deserialize)]
pub struct SimpleAuthRequest {
    pub api_key: String,
}

/// Simple authentication response
#[derive(Debug, Serialize)]
pub struct SimpleAuthResponse {
    pub authenticated: bool,
    pub key_info: Option<ApiKey>,
    pub message: String,
}

/// Check API key in headers
pub fn extract_api_key_from_headers(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(auth_header) = headers.get("authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                return Some(auth_str[7..].to_string());
            } else if auth_str.starts_with("ApiKey ") {
                return Some(auth_str[8..].to_string());
            }
        }
    }

    // Try X-API-Key header
    if let Some(api_key_header) = headers.get("x-api-key") {
        if let Ok(api_key) = api_key_header.to_str() {
            return Some(api_key.to_string());
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_key() {
        let auth = SimpleAuthManager::new();
        assert_eq!(auth.default_key(), "proxy-rs-api-key-default");
    }

    #[tokio::test]
    async fn test_key_validation() {
        let auth = SimpleAuthManager::new();
        assert!(auth.validate_key("proxy-rs-api-key-default").await);
        assert!(!auth.validate_key("invalid-key").await);
    }

    #[tokio::test]
    async fn test_create_key() {
        let auth = SimpleAuthManager::new();
        let key = auth.create_key("test key".to_string(), vec!["read".to_string()]).await;
        assert!(auth.validate_key(&key).await);
    }
}