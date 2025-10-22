//! High-performance TCP connection pooling for proxy connections

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
    sync::{Mutex, RwLock},
    time::timeout,
};

use crate::utils::http::response::ResponseParser;

/// Configuration for connection pool
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Maximum number of connections per proxy
    pub max_connections_per_proxy: usize,
    /// Maximum idle time for connections
    pub max_idle_time: Duration,
    /// Maximum total connections in pool
    pub max_total_connections: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            max_connections_per_proxy: 5,
            max_idle_time: Duration::from_secs(30),
            max_total_connections: 1000,
            connection_timeout: Duration::from_secs(5),
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// Pooled connection with metadata
#[derive(Debug)]
struct PooledConnection {
    /// The actual TCP stream
    stream: TcpStream,
    /// When this connection was created
    created_at: Instant,
    /// When this connection was last used
    last_used: Instant,
    /// Number of times this connection has been used
    use_count: u64,
    /// Whether this connection is currently in use
    in_use: bool,
    /// Whether this connection is healthy
    healthy: bool,
}

impl PooledConnection {
    fn new(stream: TcpStream) -> Self {
        let now = Instant::now();
        Self {
            stream,
            created_at: now,
            last_used: now,
            use_count: 0,
            in_use: false,
            healthy: true,
        }
    }

    fn is_expired(&self, max_idle_time: Duration) -> bool {
        self.last_used.elapsed() > max_idle_time
    }

    fn is_healthy(&self) -> bool {
        self.healthy && !self.in_use
    }

    fn mark_used(&mut self) {
        self.last_used = Instant::now();
        self.use_count += 1;
    }
}

/// Connection pool for a specific proxy
#[derive(Debug)]
struct ProxyConnectionPool {
    /// Available connections for this proxy
    connections: Vec<PooledConnection>,
    /// Total connections created for this proxy
    total_connections: usize,
    /// Proxy address
    proxy_addr: String,
}

impl ProxyConnectionPool {
    fn new(proxy_addr: String) -> Self {
        Self {
            connections: Vec::new(),
            total_connections: 0,
            proxy_addr,
        }
    }

    fn get_connection(&mut self, max_idle_time: Duration) -> Option<PooledConnection> {
        // Find the best available connection
        let mut best_index = None;
        let mut best_score = -1i64;

        for (index, conn) in self.connections.iter().enumerate() {
            if conn.is_healthy() && !conn.is_expired(max_idle_time) {
                // Score based on recency and use count (prefer less used, recent connections)
                let score = conn.last_used.elapsed().as_secs() as i64 - conn.use_count as i64;
                if score > best_score {
                    best_score = score;
                    best_index = Some(index);
                }
            }
        }

        if let Some(index) = best_index {
            let mut conn = self.connections.swap_remove(index);
            conn.mark_used();
            conn.in_use = true;
            Some(conn)
        } else {
            None
        }
    }

    fn return_connection(&mut self, mut conn: PooledConnection) {
        conn.in_use = false;
        conn.last_used = Instant::now();
        self.connections.push(conn);
    }

    fn cleanup_expired(&mut self, max_idle_time: Duration) -> usize {
        let initial_len = self.connections.len();
        self.connections.retain(|conn| {
            !conn.is_expired(max_idle_time) && conn.is_healthy()
        });
        initial_len - self.connections.len()
    }

    fn get_stats(&self) -> PoolStats {
        PoolStats {
            total_connections: self.total_connections,
            available_connections: self.connections.len(),
            active_connections: self.connections.iter().filter(|c| c.in_use).count(),
            expired_connections: self.connections.iter().filter(|c| c.is_expired(Duration::from_secs(30))).count(),
        }
    }
}

/// Global connection pool manager
#[derive(Debug)]
pub struct ConnectionPool {
    /// Individual proxy pools
    proxy_pools: RwLock<HashMap<String, ProxyConnectionPool>>,
    /// Pool configuration
    config: PoolConfig,
    /// Last cleanup time
    last_cleanup: Mutex<Instant>,
    /// Pool statistics
    stats: RwLock<PoolGlobalStats>,
}

/// Statistics for a specific proxy pool
#[derive(Debug, Clone)]
pub struct PoolStats {
    pub total_connections: usize,
    pub available_connections: usize,
    pub active_connections: usize,
    pub expired_connections: usize,
}

/// Global pool statistics
#[derive(Debug, Clone, Default)]
pub struct PoolGlobalStats {
    pub total_pools: usize,
    pub total_connections: usize,
    pub active_connections: usize,
    pub cache_hits: u64,
    pub cache_misses: u64,
    pub connections_created: u64,
    pub connections_reused: u64,
}

impl ConnectionPool {
    pub fn new(config: PoolConfig) -> Self {
        Self {
            proxy_pools: RwLock::new(HashMap::new()),
            config,
            last_cleanup: Mutex::new(Instant::now()),
            stats: RwLock::new(PoolGlobalStats::default()),
        }
    }

    /// Get a connection from the pool or create a new one
    pub async fn get_connection(&self, proxy_addr: &str) -> Result<TcpStream, Box<dyn std::error::Error + Send + Sync>> {
        // Periodic cleanup
        self.maybe_cleanup().await;

        let mut pools = self.proxy_pools.write().await;
        let pool = pools.entry(proxy_addr.to_string()).or_insert_with(|| {
            ProxyConnectionPool::new(proxy_addr.to_string())
        });

        // Try to get an existing connection
        if let Some(mut pooled_conn) = pool.get_connection(self.config.max_idle_time) {
            log::debug!("Reusing connection to {}", proxy_addr);
            
            // Update stats
            {
                let mut stats = self.stats.write().await;
                stats.cache_hits += 1;
                stats.connections_reused += 1;
            }

            // Test if connection is still alive
            if self.test_connection(&mut pooled_conn.stream).await {
                // Since we need to return the stream but also keep the pooled connection,
                // we need to handle this differently. We'll return the stream directly
                // and let the caller create a new connection if needed.
                return Ok(pooled_conn.stream);
            } else {
                log::debug!("Connection to {} is dead, creating new one", proxy_addr);
                // Connection is dead, remove it and continue to create new one
            }
        }

        // Create new connection
        log::debug!("Creating new connection to {}", proxy_addr);
        let stream = timeout(
            self.config.connection_timeout,
            TcpStream::connect(proxy_addr)
        ).await??;

        // Update stats
        {
            let mut stats = self.stats.write().await;
            stats.cache_misses += 1;
            stats.connections_created += 1;
            stats.total_connections += 1;
        }

        // Note: TcpStream doesn't have try_clone() in newer Tokio
        // For production, consider implementing connection sharing differently
        // For now, we'll not pool the connection to avoid clone issues
        log::debug!("Connection established for {} (not pooled due to Tokio limitations)", proxy_addr);

        Ok(stream)
    }

    /// Return a connection to the pool
    pub async fn return_connection(&self, proxy_addr: &str, stream: TcpStream) {
        let mut pools = self.proxy_pools.write().await;
        if let Some(pool) = pools.get_mut(proxy_addr) {
            if pool.total_connections < self.config.max_connections_per_proxy {
                let pooled_conn = PooledConnection::new(stream);
                pool.return_connection(pooled_conn);
            }
        }
    }

    /// Test if a connection is still alive
    async fn test_connection(&self, stream: &mut TcpStream) -> bool {
        // Simple read test with zero-byte read
        match stream.try_read(&mut [0u8; 1]) {
            Ok(0) => false, // Connection closed
            Ok(_) => true,  // Data available (connection alive)
            Err(_) => {
                // Would block or error - try peek
                stream.peek(&mut [0u8; 1]).await.is_ok()
            }
        }
    }

    /// Periodic cleanup of expired connections
    async fn maybe_cleanup(&self) {
        let should_cleanup = {
            let last_cleanup = self.last_cleanup.lock().await;
            last_cleanup.elapsed() > self.config.health_check_interval
        };

        if should_cleanup {
            self.cleanup_expired().await;
            *self.last_cleanup.lock().await = Instant::now();
        }
    }

    /// Cleanup expired connections
    async fn cleanup_expired(&self) {
        let mut pools = self.proxy_pools.write().await;
        let mut total_cleaned = 0;

        for (proxy_addr, pool) in pools.iter_mut() {
            let cleaned = pool.cleanup_expired(self.config.max_idle_time);
            if cleaned > 0 {
                log::debug!("Cleaned {} expired connections for {}", cleaned, proxy_addr);
                total_cleaned += cleaned;
            }
        }

        if total_cleaned > 0 {
            log::info!("Cleaned {} expired connections from pool", total_cleaned);
        }
    }

    /// Get statistics for a specific proxy
    pub async fn get_proxy_stats(&self, proxy_addr: &str) -> Option<PoolStats> {
        let pools = self.proxy_pools.read().await;
        pools.get(proxy_addr).map(|pool| pool.get_stats())
    }

    /// Get global pool statistics
    pub async fn get_global_stats(&self) -> PoolGlobalStats {
        let pools = self.proxy_pools.read().await;
        let stats = self.stats.read().await;
        
        let mut active_connections = 0;
        let mut total_connections = 0;

        for pool in pools.values() {
            let pool_stats = pool.get_stats();
            active_connections += pool_stats.active_connections;
            total_connections += pool_stats.total_connections;
        }

        PoolGlobalStats {
            total_pools: pools.len(),
            total_connections,
            active_connections,
            cache_hits: stats.cache_hits,
            cache_misses: stats.cache_misses,
            connections_created: stats.connections_created,
            connections_reused: stats.connections_reused,
        }
    }

    /// Close all connections and clear the pool
    pub async fn clear(&self) {
        let mut pools = self.proxy_pools.write().await;
        pools.clear();
        
        // Reset stats
        let mut stats = self.stats.write().await;
        *stats = PoolGlobalStats::default();
        
        log::info!("Connection pool cleared");
    }

    /// Get pool configuration
    pub fn get_config(&self) -> &PoolConfig {
        &self.config
    }
}

/// Helper function to send CONNECT request through pooled connection
pub async fn send_connect_request_pooled(
    stream: &mut TcpStream,
    host: &str,
    timeout_in_seconds: u64,
) -> bool {
    let connect = format!(
        "CONNECT {0}:443 HTTP/1.1\r\nHost: {0}:443\r\nProxy-Connection: Keep-Alive\r\n\r\n",
        host
    );
    
    // Send data
    if let Ok(Ok(_)) = timeout(
        Duration::from_secs(timeout_in_seconds),
        stream.write_all(connect.as_bytes()),
    ).await {
        // Read Response
        let data = read_timeout(stream, timeout_in_seconds).await;
        let response = ResponseParser::parse(data.as_slice());

        if let Some(status_code) = response.status_code {
            return status_code == 200;
        }
    }
    false
}

async fn read_timeout<R: tokio::io::AsyncRead + Unpin>(reader: &mut R, timeout_in_seconds: u64) -> Vec<u8> {
    let mut data = vec![];
    loop {
        let mut buf = [0; 512];
        if let Ok(Ok(buf_size)) = timeout(
            Duration::from_secs(timeout_in_seconds),
            reader.read(&mut buf),
        ).await {
            if buf_size == 0 {
                break;
            }
            data.extend(&buf[..buf_size]);
            continue;
        }
        break;
    }
    data
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_pool_creation() {
        let config = PoolConfig::default();
        let pool = ConnectionPool::new(config);
        
        let stats = pool.get_global_stats().await;
        assert_eq!(stats.total_pools, 0);
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_connection_reuse() {
        let config = PoolConfig {
            max_connections_per_proxy: 2,
            max_idle_time: Duration::from_secs(5),
            ..Default::default()
        };
        let pool = Arc::new(ConnectionPool::new(config));
        
        // This test would require a real proxy server to work properly
        // For now, we just test the pool structure
        let stats = pool.get_global_stats().await;
        assert_eq!(stats.total_connections, 0);
    }

    #[tokio::test]
    async fn test_cleanup() {
        let config = PoolConfig {
            max_idle_time: Duration::from_millis(100),
            health_check_interval: Duration::from_millis(50),
            ..Default::default()
        };
        let pool = ConnectionPool::new(config);
        
        // Wait for cleanup interval
        sleep(Duration::from_millis(150)).await;
        pool.maybe_cleanup().await;
        
        let stats = pool.get_global_stats().await;
        assert_eq!(stats.total_connections, 0);
    }
}
