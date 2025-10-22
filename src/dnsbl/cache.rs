//! DNSBL result caching system

use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::dnsbl::{DnsblCheckResults, DnsblConfig};

/// Cache entry for DNSBL check results
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached results
    results: DnsblCheckResults,
    /// When this entry was created
    created_at: Instant,
    /// When this entry expires
    expires_at: Instant,
}

/// DNSBL result cache
#[derive(Debug, Clone)]
pub struct DnsblCache {
    /// Internal cache storage
    cache: HashMap<String, CacheEntry>,
    /// Default TTL for cache entries
    default_ttl: Duration,
    /// Maximum number of entries in cache
    max_entries: usize,
    /// Cache statistics
    stats: CacheStats,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    /// Total number of cache hits
    pub hits: u64,
    /// Total number of cache misses
    pub misses: u64,
    /// Total number of entries added
    pub additions: u64,
    /// Total number of entries expired/evicted
    pub evictions: u64,
    /// Current cache size
    pub current_size: usize,
}

impl DnsblCache {
    /// Create new DNSBL cache with default settings
    pub fn new() -> Self {
        Self::with_settings(Duration::from_secs(3600), 10000) // 1 hour TTL, 10k max entries
    }
    
    /// Create new DNSBL cache with custom settings
    pub fn with_settings(ttl: Duration, max_entries: usize) -> Self {
        Self {
            cache: HashMap::new(),
            default_ttl: ttl,
            max_entries,
            stats: CacheStats::default(),
        }
    }
    
    /// Get cached results for an IP address
    pub fn get(&mut self, ip: &str) -> Option<DnsblCheckResults> {
        // Clean expired entries first
        self.cleanup_expired();
        
        if let Some(entry) = self.cache.get(ip) {
            if entry.expires_at > Instant::now() {
                self.stats.hits += 1;
                log::debug!("DNSBL cache hit for IP: {}", ip);
                return Some(entry.results.clone());
            } else {
                // Entry expired, remove it
                self.cache.remove(ip);
                self.stats.evictions += 1;
            }
        }
        
        self.stats.misses += 1;
        log::debug!("DNSBL cache miss for IP: {}", ip);
        None
    }
    
    /// Store DNSBL check results for an IP address
    pub fn put(&mut self, ip: String, results: DnsblCheckResults) {
        // Check if we need to evict entries
        if self.cache.len() >= self.max_entries {
            self.evict_lru();
        }
        
        let now = Instant::now();
        let entry = CacheEntry {
            results: results.clone(),
            created_at: now,
            expires_at: now + self.default_ttl,
        };
        
        self.cache.insert(ip.clone(), entry);
        self.stats.additions += 1;
        self.stats.current_size = self.cache.len();
        
        log::debug!("DNSBL cache stored for IP: {} (TTL: {:?})", ip, self.default_ttl);
    }
    
    /// Store results with custom TTL
    pub fn put_with_ttl(&mut self, ip: String, results: DnsblCheckResults, ttl: Duration) {
        // Check if we need to evict entries
        if self.cache.len() >= self.max_entries {
            self.evict_lru();
        }
        
        let now = Instant::now();
        let entry = CacheEntry {
            results: results.clone(),
            created_at: now,
            expires_at: now + ttl,
        };
        
        self.cache.insert(ip.clone(), entry);
        self.stats.additions += 1;
        self.stats.current_size = self.cache.len();
        
        log::debug!("DNSBL cache stored for IP: {} (custom TTL: {:?})", ip, ttl);
    }
    
    /// Remove entry for specific IP
    pub fn remove(&mut self, ip: &str) -> Option<DnsblCheckResults> {
        if let Some(entry) = self.cache.remove(ip) {
            self.stats.evictions += 1;
            self.stats.current_size = self.cache.len();
            log::debug!("DNSBL cache removed for IP: {}", ip);
            Some(entry.results)
        } else {
            None
        }
    }
    
    /// Clear all cache entries
    pub fn clear(&mut self) {
        let count = self.cache.len();
        self.cache.clear();
        self.stats.evictions += count as u64;
        self.stats.current_size = 0;
        log::debug!("DNSBL cache cleared ({} entries removed)", count);
    }
    
    /// Clean up expired entries
    pub fn cleanup_expired(&mut self) {
        let now = Instant::now();
        let mut expired_keys = Vec::new();

        for (ip, entry) in &self.cache {
            if entry.expires_at <= now {
                expired_keys.push(ip.clone());
            }
        }

        let expired_count = expired_keys.len();
        for key in expired_keys {
            self.cache.remove(&key);
            self.stats.evictions += 1;
        }

        self.stats.current_size = self.cache.len();

        if expired_count > 0 {
            log::debug!("DNSBL cache cleanup: {} expired entries removed", expired_count);
        }
    }
    
    /// Evict least recently used entries
    fn evict_lru(&mut self) {
        if self.cache.is_empty() {
            return;
        }
        
        // Find the oldest entry
        let oldest_key = self
            .cache
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(key, _)| key.clone());
        
        if let Some(key) = oldest_key {
            self.cache.remove(&key);
            self.stats.evictions += 1;
            self.stats.current_size = self.cache.len();
            log::debug!("DNSBL cache evicted LRU entry: {}", key);
        }
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> &CacheStats {
        &self.stats
    }
    
    /// Reset cache statistics
    pub fn reset_stats(&mut self) {
        self.stats = CacheStats {
            current_size: self.cache.len(),
            ..Default::default()
        };
    }
    
    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.stats.hits + self.stats.misses;
        if total == 0 {
            0.0
        } else {
            (self.stats.hits as f64 / total as f64) * 100.0
        }
    }
    
    /// Get number of entries that will expire soon (within next 5 minutes)
    pub fn expiring_soon_count(&self) -> usize {
        let soon = Instant::now() + Duration::from_secs(300); // 5 minutes
        self.cache
            .values()
            .filter(|entry| entry.expires_at <= soon)
            .count()
    }
    
    /// Get cache memory usage estimate (in bytes)
    pub fn memory_usage_estimate(&self) -> usize {
        // Rough estimate: each entry ~1KB average
        self.cache.len() * 1024
    }
    
    /// Optimize cache by removing entries with low hit rate
    pub fn optimize_by_hit_rate(&mut self, keep_ratio: f64) {
        if self.cache.len() <= 10 {
            return; // Too small to optimize
        }

        // Collect keys to remove first to avoid borrow issues
        let keys_to_remove: Vec<_> = self.cache
            .keys()
            .cloned()
            .collect();

        // Calculate how many to keep
        let keep_count = (self.cache.len() as f64 * keep_ratio) as usize;
        let remove_count = self.cache.len() - keep_count;

        // Remove oldest entries
        for key in keys_to_remove.into_iter().take(remove_count) {
            self.cache.remove(key.as_str());
            self.stats.evictions += 1;
        }

        self.stats.current_size = self.cache.len();
        log::debug!(
            "DNSBL cache optimized: kept {} entries, removed {} entries",
            keep_count,
            remove_count
        );
    }
}

impl Default for DnsblCache {
    fn default() -> Self {
        Self::new()
    }
}

/// DNSBL cache manager that handles cache lifecycle
#[derive(Clone, Debug)]
pub struct DnsblCacheManager {
    cache: DnsblCache,
    config: DnsblConfig,
    cleanup_interval: Duration,
    last_cleanup: Instant,
}

impl DnsblCacheManager {
    /// Create new cache manager
    pub fn new(config: DnsblConfig) -> Self {
        let cache = DnsblCache::with_settings(
            Duration::from_secs(config.cache_ttl_secs),
            10000, // Default max entries
        );
        
        Self {
            cache,
            config,
            cleanup_interval: Duration::from_secs(300), // Cleanup every 5 minutes
            last_cleanup: Instant::now(),
        }
    }
    
    /// Get cached results, with automatic cleanup
    pub fn get(&mut self, ip: &str) -> Option<DnsblCheckResults> {
        self.maybe_cleanup();
        self.cache.get(ip)
    }
    
    /// Store results in cache
    pub fn put(&mut self, ip: String, results: DnsblCheckResults) {
        self.maybe_cleanup();
        
        // Use custom TTL based on results
        let ttl = if results.is_malicious {
            // Cache malicious results longer
            Duration::from_secs(self.config.cache_ttl_secs * 2)
        } else {
            Duration::from_secs(self.config.cache_ttl_secs)
        };
        
        self.cache.put_with_ttl(ip, results, ttl);
    }
    
    /// Perform cleanup if needed
    fn maybe_cleanup(&mut self) {
        if self.last_cleanup.elapsed() >= self.cleanup_interval {
            self.cache.cleanup_expired();
            self.last_cleanup = Instant::now();
        }
    }
    
    /// Get cache statistics
    pub fn get_stats(&self) -> &CacheStats {
        self.cache.get_stats()
    }
    
    /// Force cleanup
    pub fn force_cleanup(&mut self) {
        self.cache.cleanup_expired();
        self.last_cleanup = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dnsbl::{DnsblResult, DnsblCheckResults};
    
    fn create_test_results(ip: &str, listed: bool) -> DnsblCheckResults {
        let mut results = DnsblCheckResults::new(ip.to_string());
        results.add_result(DnsblResult {
            list_name: "test".to_string(),
            listed,
            reason: None,
            response_time_ms: 100,
        });
        results.update_malicious_status(1);
        results
    }
    
    #[test]
    fn test_cache_basic_operations() {
        let mut cache = DnsblCache::new();
        
        // Test miss
        assert!(cache.get("192.168.1.1").is_none());
        assert_eq!(cache.stats.misses, 1);
        
        // Test put
        let results = create_test_results("192.168.1.1", false);
        cache.put("192.168.1.1".to_string(), results.clone());
        assert_eq!(cache.stats.additions, 1);
        
        // Test hit
        let cached = cache.get("192.168.1.1").expect("Failed to get cached result");
        assert_eq!(cached.ip, "192.168.1.1");
        assert_eq!(cache.stats.hits, 1);
    }
    
    #[test]
    fn test_cache_ttl() {
        let mut cache = DnsblCache::with_settings(Duration::from_millis(100), 100);
        
        let results = create_test_results("192.168.1.1", false);
        cache.put("192.168.1.1".to_string(), results);
        
        // Should be available immediately
        assert!(cache.get("192.168.1.1").is_some());
        
        // Wait for expiration
        std::thread::sleep(Duration::from_millis(150));
        
        // Should be expired now
        assert!(cache.get("192.168.1.1").is_none());
        assert_eq!(cache.stats.evictions, 1);
    }
    
    #[test]
    fn test_cache_max_entries() {
        let mut cache = DnsblCache::with_settings(Duration::from_secs(3600), 2);
        
        // Add 3 entries (max is 2)
        cache.put("192.168.1.1".to_string(), create_test_results("192.168.1.1", false));
        cache.put("192.168.1.2".to_string(), create_test_results("192.168.1.2", false));
        cache.put("192.168.1.3".to_string(), create_test_results("192.168.1.3", false));
        
        // Should have evicted one entry
        assert_eq!(cache.cache.len(), 2);
        assert_eq!(cache.stats.evictions, 1);
    }
    
    #[test]
    fn test_cache_stats() {
        let mut cache = DnsblCache::new();
        
        // Initial stats
        assert_eq!(cache.hit_rate(), 0.0);
        
        // Add some data
        cache.put("192.168.1.1".to_string(), create_test_results("192.168.1.1", false));
        
        // Generate hits and misses
        cache.get("192.168.1.1"); // hit
        cache.get("192.168.1.2"); // miss
        
        assert_eq!(cache.stats.hits, 1);
        assert_eq!(cache.stats.misses, 1);
        assert_eq!(cache.hit_rate(), 50.0);
    }
    
    #[test]
    fn test_cache_cleanup() {
        let mut cache = DnsblCache::with_settings(Duration::from_millis(50), 100);
        
        // Add entries
        cache.put("192.168.1.1".to_string(), create_test_results("192.168.1.1", false));
        cache.put("192.168.1.2".to_string(), create_test_results("192.168.1.2", false));
        
        assert_eq!(cache.cache.len(), 2);
        
        // Wait for expiration and cleanup
        std::thread::sleep(Duration::from_millis(100));
        cache.cleanup_expired();
        
        assert_eq!(cache.cache.len(), 0);
    }
    
    #[test]
    fn test_cache_manager() {
        let config = DnsblConfig {
            cache_ttl_secs: 1,
            ..Default::default()
        };
        let mut manager = DnsblCacheManager::new(config);
        
        // Test basic operations
        let results = create_test_results("192.168.1.1", false);
        manager.put("192.168.1.1".to_string(), results);
        
        let cached = manager.get("192.168.1.1").expect("Failed to get cached result from manager");
        assert_eq!(cached.ip, "192.168.1.1");
        
        // Test stats
        let stats = manager.get_stats();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.additions, 1);
    }
}
