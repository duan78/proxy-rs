//! Resource management utilities for preventing memory leaks

use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use parking_lot::RwLock;

/// Resource manager for tracking and cleaning up resources
#[derive(Debug)]
pub struct ResourceManager<T> {
    resources: Arc<RwLock<HashMap<String, ResourceEntry<T>>>>,
    max_resources: usize,
    cleanup_interval: Duration,
    last_cleanup: Arc<Mutex<Instant>>,
}

#[derive(Debug)]
struct ResourceEntry<T> {
    resource: T,
    created_at: Instant,
    last_accessed: Instant,
    access_count: u64,
}

impl<T> ResourceManager<T> {
    pub fn new(max_resources: usize, cleanup_interval: Duration) -> Self {
        Self {
            resources: Arc::new(RwLock::new(HashMap::new())),
            max_resources,
            cleanup_interval,
            last_cleanup: Arc::new(Mutex::new(Instant::now())),
        }
    }

    /// Add a resource to the manager
    pub async fn add_resource(&self, key: String, resource: T) -> Result<(), &'static str> {
        let mut resources = self.resources.write();

        if resources.len() >= self.max_resources {
            self.cleanup_old_resources(&mut resources);
        }

        let now = Instant::now();
        resources.insert(key, ResourceEntry {
            resource,
            created_at: now,
            last_accessed: now,
            access_count: 0,
        });

        Ok(())
    }

    /// Get a resource by key
    pub async fn get_resource(&self, key: &str) -> Option<T>
    where
        T: Clone,
    {
        let mut resources = self.resources.write();
        if let Some(entry) = resources.get_mut(key) {
            entry.last_accessed = Instant::now();
            entry.access_count += 1;
            Some(entry.resource.clone())
        } else {
            None
        }
    }

    /// Remove a resource
    pub async fn remove_resource(&self, key: &str) -> Option<T> {
        let mut resources = self.resources.write();
        resources.remove(key).map(|entry| entry.resource)
    }

    /// Periodic cleanup of old resources
    pub async fn periodic_cleanup(&self) {
        let mut last_cleanup = self.last_cleanup.lock().await;
        if last_cleanup.elapsed() > self.cleanup_interval {
            let mut resources = self.resources.write();
            self.cleanup_old_resources(&mut resources);
            *last_cleanup = Instant::now();
        }
    }

    /// Clean up old resources based on LRU and age
    fn cleanup_old_resources(&self, resources: &mut HashMap<String, ResourceEntry<T>>) {
        if resources.len() <= self.max_resources / 2 {
            return;
        }

        // Sort resources by last accessed time and remove oldest 25%
        let mut entries: Vec<_> = resources.iter().collect();
        entries.sort_by_key(|(_, entry)| entry.last_accessed);

        let to_remove = resources.len() / 4; // Remove 25%
        let keys_to_remove: Vec<String> = entries.iter()
            .take(to_remove)
            .map(|(key, _)| (*key).to_string())
            .collect();

        drop(entries); // Release immutable borrow

        for key in keys_to_remove {
            resources.remove(&key);
        }
    }

    /// Get resource statistics
    pub fn get_stats(&self) -> ResourceStats {
        let resources = self.resources.read();
        let total_resources = resources.len();
        let total_accesses: u64 = resources.values()
            .map(|entry| entry.access_count)
            .sum();

        let oldest_resource = resources.values()
            .map(|entry| entry.created_at)
            .min();

        ResourceStats {
            total_resources,
            total_accesses,
            oldest_resource_age: oldest_resource.map(|time| time.elapsed()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ResourceStats {
    pub total_resources: usize,
    pub total_accesses: u64,
    pub oldest_resource_age: Option<Duration>,
}

/// Connection pool manager with automatic cleanup
#[derive(Debug)]
pub struct ConnectionPoolManager {
    active_connections: Arc<RwLock<HashMap<String, ConnectionInfo>>>,
    max_connections: usize,
    connection_timeout: Duration,
}

#[derive(Debug)]
struct ConnectionInfo {
    created_at: Instant,
    last_used: Instant,
    usage_count: u64,
}

impl ConnectionPoolManager {
    pub fn new(max_connections: usize, connection_timeout: Duration) -> Self {
        Self {
            active_connections: Arc::new(RwLock::new(HashMap::new())),
            max_connections,
            connection_timeout,
        }
    }

    /// Register a new connection
    pub fn register_connection(&self, connection_id: String) -> Result<(), &'static str> {
        let mut connections = self.active_connections.write();

        if connections.len() >= self.max_connections {
            self.cleanup_expired_connections();
        }

        let now = Instant::now();
        connections.insert(connection_id, ConnectionInfo {
            created_at: now,
            last_used: now,
            usage_count: 0,
        });

        Ok(())
    }

    /// Update connection usage
    pub fn update_connection_usage(&self, connection_id: &str) {
        let mut connections = self.active_connections.write();
        if let Some(conn_info) = connections.get_mut(connection_id) {
            conn_info.last_used = Instant::now();
            conn_info.usage_count += 1;
        }
    }

    /// Remove a connection
    pub fn remove_connection(&self, connection_id: &str) {
        let mut connections = self.active_connections.write();
        connections.remove(connection_id);
    }

    /// Clean up expired connections
    pub fn cleanup_expired_connections(&self) {
        let mut connections = self.active_connections.write();

        let now = Instant::now();
        let expired_connections: Vec<String> = connections
            .iter()
            .filter(|(_, info)| now.duration_since(info.last_used) > self.connection_timeout)
            .map(|(id, _)| id.clone())
            .collect();

        for id in expired_connections {
            connections.remove(&id);
        }
    }

    /// Get connection statistics
    pub fn get_connection_stats(&self) -> ConnectionStats {
        let connections = self.active_connections.read();
        ConnectionStats {
            active_connections: connections.len(),
            total_usage: connections.values().map(|info| info.usage_count).sum(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub active_connections: usize,
    pub total_usage: u64,
}

/// Global resource manager singleton
use once_cell::sync::Lazy;
use crate::utils::error::{ProxyError, ProxyResult};

pub static GLOBAL_RESOURCE_MANAGER: Lazy<ResourceManager<Arc<Semaphore>>> =
    Lazy::new(|| ResourceManager::new(1000, Duration::from_secs(60)));

pub static CONNECTION_MANAGER: Lazy<ConnectionPoolManager> =
    Lazy::new(|| ConnectionPoolManager::new(10000, Duration::from_secs(300)));

/// Initialize global resource managers
pub fn init_resource_managers() {
    // Force initialization of lazy static variables
    Lazy::force(&GLOBAL_RESOURCE_MANAGER);
    Lazy::force(&CONNECTION_MANAGER);
}

/// Get a semaphore for resource limiting
pub async fn get_resource_semaphore(name: &str) -> ProxyResult<Arc<Semaphore>> {
    GLOBAL_RESOURCE_MANAGER
        .get_resource(name)
        .await
        .ok_or_else(|| ProxyError::ResourceExhausted(format!("Resource semaphore '{}' not found", name)))
}

/// Create and register a resource semaphore
pub async fn create_resource_semaphore(name: String, permits: usize) -> ProxyResult<()> {
    let semaphore = Arc::new(Semaphore::new(permits));
    GLOBAL_RESOURCE_MANAGER
        .add_resource(name, semaphore)
        .await
        .map_err(|_| ProxyError::ResourceExhausted("Failed to create resource semaphore".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_resource_manager() {
        let manager = ResourceManager::new(10, Duration::from_millis(100));

        assert!(manager.add_resource("test".to_string(), 42).await.is_ok());
        assert_eq!(manager.get_resource("test").await, Some(42));

        let stats = manager.get_stats();
        assert_eq!(stats.total_resources, 1);
        assert_eq!(stats.total_accesses, 1);
    }

    #[tokio::test]
    async fn test_connection_manager() {
        let manager = ConnectionPoolManager::new(5, Duration::from_millis(100));

        assert!(manager.register_connection("conn1".to_string()).is_ok());
        manager.update_connection_usage("conn1");

        let stats = manager.get_connection_stats();
        assert_eq!(stats.active_connections, 1);
        assert_eq!(stats.total_usage, 1);

        manager.remove_connection("conn1");
        let stats = manager.get_connection_stats();
        assert_eq!(stats.active_connections, 0);
    }
}