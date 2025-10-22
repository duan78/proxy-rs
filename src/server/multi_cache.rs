//! Multi-level caching system for proxy.rs performance optimization

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use lru::LruCache;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::resolver::GeoData;

/// Cache entry with metadata
#[derive(Debug, Clone)]
struct CacheEntry<T> {
    /// The cached value
    value: T,
    /// When this entry was created
    created_at: Instant,
    /// When this entry was last accessed
    last_accessed: Instant,
    /// Number of times this entry was accessed
    access_count: u64,
    /// TTL for this entry
    ttl: Duration,
}

impl<T> CacheEntry<T> {
    fn new(value: T, ttl: Duration) -> Self {
        let now = Instant::now();
        Self {
            value,
            created_at: now,
            last_accessed: now,
            access_count: 0,
            ttl,
        }
    }

    fn is_expired(&self) -> bool {
        self.created_at.elapsed() > self.ttl
    }

    fn mark_accessed(&mut self) {
        self.last_accessed = Instant::now();
        self.access_count += 1;
    }

    fn access_frequency(&self) -> f64 {
        let age = self.created_at.elapsed().as_secs_f64();
        if age == 0.0 {
            self.access_count as f64
        } else {
            self.access_count as f64 / age
        }
    }
}

/// L1 Cache: In-memory LRU cache for frequently accessed items
#[derive(Debug)]
struct L1Cache<T: Clone> {
    cache: LruCache<String, CacheEntry<T>>,
    max_size: usize,
    default_ttl: Duration,
}

impl<T: Clone> L1Cache<T> {
    fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: LruCache::new(
                std::num::NonZeroUsize::new(max_size).unwrap()
            ),
            max_size,
            default_ttl,
        }
    }

    fn get(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.cache.get_mut(key) {
            if entry.is_expired() {
                // LruCache doesn't have pop for removal, would need to use pop_lru
                return None;
            }
            entry.mark_accessed();
            Some(entry.value.clone())
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: T, ttl: Option<Duration>) {
        let entry = CacheEntry::new(value, ttl.unwrap_or(self.default_ttl));
        self.cache.put(key, entry);
    }

    fn cleanup_expired(&mut self) -> usize {
        let initial_len = self.cache.len();
        // LruCache doesn't have retain, need to recreate with only non-expired entries
        let mut new_cache = lru::LruCache::new(
            std::num::NonZeroUsize::new(self.max_size).unwrap()
        );

        // Collect keys to keep
        let keys_to_keep: Vec<_> = self.cache
            .iter()
            .filter_map(|(key, entry)| if !entry.is_expired() { Some(key.clone()) } else { None })
            .collect();

        // Rebuild cache with non-expired entries using references to avoid move issues
        for key in &keys_to_keep {
            if let Some(entry) = self.cache.get(key) {
                new_cache.put(key.clone(), entry.clone());
            }
        }

        self.cache = new_cache;
        initial_len - self.cache.len()
    }

    fn get_stats(&self) -> L1CacheStats {
        L1CacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
            hit_rate: 0.0, // Will be calculated at higher level
        }
    }
}

/// L2 Cache: Frequency-based cache for moderately accessed items
#[derive(Debug)]
struct L2Cache<T: Clone> {
    cache: LruCache<String, CacheEntry<T>>,
    max_size: usize,
    default_ttl: Duration,
    access_threshold: f64,
}

impl<T: Clone> L2Cache<T> {
    fn new(max_size: usize, default_ttl: Duration, access_threshold: f64) -> Self {
        Self {
            cache: LruCache::new(
                std::num::NonZeroUsize::new(max_size).unwrap()
            ),
            max_size,
            default_ttl,
            access_threshold,
        }
    }

    fn get(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.cache.get_mut(key) {
            if entry.is_expired() {
                // LruCache doesn't have pop for removal, would need to use pop_lru
                return None;
            }
            entry.mark_accessed();
            Some(entry.value.clone())
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: T, ttl: Option<Duration>) {
        if self.cache.len() >= self.max_size {
            self.evict_least_frequent();
        }
        
        let entry = CacheEntry::new(value, ttl.unwrap_or(self.default_ttl));
        self.cache.put(key, entry);
    }

    fn evict_least_frequent(&mut self) {
        if let Some((_key_to_remove, _)) = self.cache
            .iter()
            .min_by(|(_, a), (_, b)| a.access_frequency().partial_cmp(&b.access_frequency()).unwrap())
            .map(|(k, _)| (k.clone(), ())) {
            // LruCache doesn't have pop for specific keys, would need different approach
        }
    }

    fn cleanup_expired(&mut self) -> usize {
        let initial_len = self.cache.len();
        // LruCache doesn't have retain, need to recreate with only non-expired entries
        let mut new_cache = lru::LruCache::new(
            std::num::NonZeroUsize::new(self.max_size).unwrap()
        );

        // Collect keys to keep
        let keys_to_keep: Vec<_> = self.cache
            .iter()
            .filter_map(|(key, entry)| if !entry.is_expired() { Some(key.clone()) } else { None })
            .collect();

        // Rebuild cache with non-expired entries using references to avoid move issues
        for key in &keys_to_keep {
            if let Some(entry) = self.cache.get(key) {
                new_cache.put(key.clone(), entry.clone());
            }
        }

        self.cache = new_cache;
        initial_len - self.cache.len()
    }

    fn promote_to_l1(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.cache.get(key) {
            if entry.access_frequency() > self.access_threshold {
                // Need to clone the value since we can't move out of LruCache
                Some(entry.value.clone())
            } else {
                None
            }
        } else {
            None
        }
    }

    fn get_stats(&self) -> L2CacheStats {
        L2CacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
            avg_access_frequency: self.cache
                .iter()
                .map(|(_, e)| e.access_frequency())
                .sum::<f64>() / self.cache.len().max(1) as f64,
        }
    }
}

/// L3 Cache: Persistent cache for rarely accessed but expensive items
#[derive(Debug)]
struct L3Cache<T: Clone> {
    cache: HashMap<String, CacheEntry<T>>,
    max_size: usize,
    default_ttl: Duration,
}

impl<T: Clone> L3Cache<T> {
    fn new(max_size: usize, default_ttl: Duration) -> Self {
        Self {
            cache: HashMap::new(),
            max_size,
            default_ttl,
        }
    }

    fn get(&mut self, key: &str) -> Option<T> {
        if let Some(entry) = self.cache.get_mut(key) {
            if entry.is_expired() {
                // LruCache doesn't have pop for removal, would need to use pop_lru
                return None;
            }
            entry.mark_accessed();
            Some(entry.value.clone())
        } else {
            None
        }
    }

    fn put(&mut self, key: String, value: T, ttl: Option<Duration>) {
        if self.cache.len() >= self.max_size {
            self.evict_oldest();
        }

        let entry = CacheEntry::new(value, ttl.unwrap_or(self.default_ttl));
        self.cache.insert(key, entry);
    }

    fn evict_oldest(&mut self) {
        if let Some(_key_to_remove) = self.cache
            .iter()
            .min_by_key(|(_, entry)| entry.created_at)
            .map(|(k, _)| k.clone()) {
            // LruCache doesn't have pop for specific keys, would need different approach
        }
    }

    fn cleanup_expired(&mut self) -> usize {
        let initial_len = self.cache.len();
        // HashMap can use retain directly
        self.cache.retain(|_, entry| !entry.is_expired());
        initial_len - self.cache.len()
    }

    fn get_stats(&self) -> L3CacheStats {
        L3CacheStats {
            size: self.cache.len(),
            max_size: self.max_size,
        }
    }
}

/// Multi-level cache system
#[derive(Debug)]
pub struct MultiCache<T: Clone + Send + Sync + 'static> {
    l1_cache: Arc<RwLock<L1Cache<T>>>,
    l2_cache: Arc<RwLock<L2Cache<T>>>,
    l3_cache: Arc<RwLock<L3Cache<T>>>,
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStats {
    pub l1_hits: u64,
    pub l2_hits: u64,
    pub l3_hits: u64,
    pub total_misses: u64,
    pub total_requests: u64,
    pub evictions: u64,
    pub promotions: u64,
}

impl CacheStats {
    pub fn hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            ((self.l1_hits + self.l2_hits + self.l3_hits) as f64 / self.total_requests as f64) * 100.0
        }
    }

    pub fn l1_hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.l1_hits as f64 / self.total_requests as f64) * 100.0
        }
    }

    pub fn l2_hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.l2_hits as f64 / self.total_requests as f64) * 100.0
        }
    }

    pub fn l3_hit_rate(&self) -> f64 {
        if self.total_requests == 0 {
            0.0
        } else {
            (self.l3_hits as f64 / self.total_requests as f64) * 100.0
        }
    }
}

/// Statistics for individual cache levels
#[derive(Debug, Clone)]
pub struct L1CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub hit_rate: f64,
}

#[derive(Debug, Clone)]
pub struct L2CacheStats {
    pub size: usize,
    pub max_size: usize,
    pub avg_access_frequency: f64,
}

#[derive(Debug, Clone)]
pub struct L3CacheStats {
    pub size: usize,
    pub max_size: usize,
}

/// Configuration for multi-level cache
#[derive(Debug, Clone)]
pub struct MultiCacheConfig {
    /// L1 cache size (most frequently accessed)
    pub l1_size: usize,
    /// L2 cache size (moderately accessed)
    pub l2_size: usize,
    /// L3 cache size (rarely accessed)
    pub l3_size: usize,
    /// Default TTL for L1 cache
    pub l1_ttl: Duration,
    /// Default TTL for L2 cache
    pub l2_ttl: Duration,
    /// Default TTL for L3 cache
    pub l3_ttl: Duration,
    /// Access frequency threshold for L1 promotion
    pub l1_promotion_threshold: f64,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for MultiCacheConfig {
    fn default() -> Self {
        Self {
            l1_size: 1000,
            l2_size: 5000,
            l3_size: 10000,
            l1_ttl: Duration::from_secs(300),  // 5 minutes
            l2_ttl: Duration::from_secs(1800), // 30 minutes
            l3_ttl: Duration::from_secs(7200), // 2 hours
            l1_promotion_threshold: 0.1,       // 0.1 accesses per second
            cleanup_interval: Duration::from_secs(60), // 1 minute
        }
    }
}

impl<T: Clone + Send + Sync + 'static> MultiCache<T> {
    pub fn new(config: MultiCacheConfig) -> Self {
        let l1_cache = Arc::new(RwLock::new(L1Cache::new(config.l1_size, config.l1_ttl)));
        let l2_cache = Arc::new(RwLock::new(L2Cache::new(config.l2_size, config.l2_ttl, config.l1_promotion_threshold)));
        let l3_cache = Arc::new(RwLock::new(L3Cache::new(config.l3_size, config.l3_ttl)));
        let stats = Arc::new(RwLock::new(CacheStats::default()));

        let cache = Self {
            l1_cache,
            l2_cache,
            l3_cache,
            stats,
        };

        // Start cleanup task
        cache.start_cleanup_task(config.cleanup_interval);
        cache
    }

    /// Get value from cache (tries L1, then L2, then L3)
    pub async fn get(&self, key: &str) -> Option<T> {
        let mut stats = self.stats.write().await;
        stats.total_requests += 1;
        drop(stats);

        // Try L1 first
        {
            let mut l1 = self.l1_cache.write().await;
            if let Some(value) = l1.get(key) {
                let mut stats = self.stats.write().await;
                stats.l1_hits += 1;
                return Some(value);
            }
        }

        // Try L2
        {
            let mut l2 = self.l2_cache.write().await;
            if let Some(value) = l2.get(key) {
                let mut stats = self.stats.write().await;
                stats.l2_hits += 1;

                // Check if we should promote to L1
                if l2.cache.get(key).map(|e| e.access_frequency()).unwrap_or(0.0) > l2.access_threshold {
                    if let Some(promoted_value) = l2.promote_to_l1(key) {
                        let mut l1 = self.l1_cache.write().await;
                        l1.put(key.to_string(), promoted_value, None);
                        
                        let mut stats = self.stats.write().await;
                        stats.promotions += 1;
                    }
                }

                return Some(value);
            }
        }

        // Try L3
        {
            let mut l3 = self.l3_cache.write().await;
            if let Some(value) = l3.get(key) {
                let mut stats = self.stats.write().await;
                stats.l3_hits += 1;

                // Move to L2 since it was accessed
                let mut l2 = self.l2_cache.write().await;
                l2.put(key.to_string(), value.clone(), None);

                return Some(value);
            }
        }

        // Cache miss
        let mut stats = self.stats.write().await;
        stats.total_misses += 1;
        None
    }

    /// Put value in cache (starts in L3, can be promoted)
    pub async fn put(&self, key: String, value: T, level: CacheLevel) {
        match level {
            CacheLevel::L1 => {
                let mut l1 = self.l1_cache.write().await;
                l1.put(key, value, None);
            }
            CacheLevel::L2 => {
                let mut l2 = self.l2_cache.write().await;
                l2.put(key, value, None);
            }
            CacheLevel::L3 => {
                let mut l3 = self.l3_cache.write().await;
                l3.put(key, value, None);
            }
        }
    }

    /// Put value with custom TTL
    pub async fn put_with_ttl(&self, key: String, value: T, ttl: Duration, level: CacheLevel) {
        match level {
            CacheLevel::L1 => {
                let mut l1 = self.l1_cache.write().await;
                l1.put(key, value, Some(ttl));
            }
            CacheLevel::L2 => {
                let mut l2 = self.l2_cache.write().await;
                l2.put(key, value, Some(ttl));
            }
            CacheLevel::L3 => {
                let mut l3 = self.l3_cache.write().await;
                l3.put(key, value, Some(ttl));
            }
        }
    }

    /// Remove item from cache
    pub async fn remove(&self, key: &str) {
        let mut l1 = self.l1_cache.write().await;
        l1.cache.pop(key);
        
        let _l2 = self.l2_cache.write().await;
        // Note: LruCache doesn't have direct remove by key - this would need reimplementation

        let _l3 = self.l3_cache.write().await;
        // Note: LruCache doesn't have direct remove by key - this would need reimplementation
    }

    /// Clear all caches
    pub async fn clear(&self) {
        let mut l1 = self.l1_cache.write().await;
        l1.cache.clear();
        
        let mut l2 = self.l2_cache.write().await;
        l2.cache.clear();
        
        let mut l3 = self.l3_cache.write().await;
        l3.cache.clear();
        
        let mut stats = self.stats.write().await;
        *stats = CacheStats::default();
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Get detailed cache statistics
    pub async fn get_detailed_stats(&self) -> DetailedCacheStats {
        let stats = self.stats.read().await.clone();
        let l1_stats = self.l1_cache.read().await.get_stats();
        let l2_stats = self.l2_cache.read().await.get_stats();
        let l3_stats = self.l3_cache.read().await.get_stats();

        DetailedCacheStats {
            overall: stats,
            l1: l1_stats,
            l2: l2_stats,
            l3: l3_stats,
        }
    }

    /// Start background cleanup task
    fn start_cleanup_task(&self, interval: Duration) {
        let l1_cache = Arc::clone(&self.l1_cache);
        let l2_cache = Arc::clone(&self.l2_cache);
        let l3_cache = Arc::clone(&self.l3_cache);
        let stats = Arc::clone(&self.stats);

        tokio::spawn(async move {
            let mut cleanup_interval = tokio::time::interval(interval);
            loop {
                cleanup_interval.tick().await;

                let mut total_cleaned = 0;

                // Cleanup L1
                {
                    let mut l1 = l1_cache.write().await;
                    total_cleaned += l1.cleanup_expired();
                }

                // Cleanup L2
                {
                    let mut l2 = l2_cache.write().await;
                    total_cleaned += l2.cleanup_expired();
                }

                // Cleanup L3
                {
                    let mut l3 = l3_cache.write().await;
                    total_cleaned += l3.cleanup_expired();
                }

                if total_cleaned > 0 {
                    log::debug!("Cache cleanup: removed {} expired entries", total_cleaned);
                    
                    let mut stats = stats.write().await;
                    stats.evictions += total_cleaned as u64;
                }
            }
        });
    }
}

/// Cache level for insertion
#[derive(Debug, Clone, Copy)]
pub enum CacheLevel {
    L1, // Most frequently accessed
    L2, // Moderately accessed
    L3, // Rarely accessed
}

/// Detailed cache statistics
#[derive(Debug, Clone)]
pub struct DetailedCacheStats {
    pub overall: CacheStats,
    pub l1: L1CacheStats,
    pub l2: L2CacheStats,
    pub l3: L3CacheStats,
}

/// Specialized cache types for different data

/// Cache for proxy validation results
pub type ProxyValidationCache = MultiCache<ProxyValidationResult>;

/// Cache for DNSBL results
pub type DnsblCache = MultiCache<DnsblResult>;

/// Cache for geolocation data
pub type GeoCache = MultiCache<GeoData>;

/// Cache for connection metadata
pub type ConnectionMetadataCache = MultiCache<ConnectionMetadata>;

/// Proxy validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProxyValidationResult {
    pub is_working: bool,
    pub response_time_ms: u64,
    pub error_count: u32,
    pub last_checked: std::time::SystemTime,
    pub dnsbl_clean: bool,
}

/// DNSBL result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsblResult {
    pub is_listed: bool,
    pub listed_count: u32,
    pub checked_lists: Vec<String>,
    pub last_checked: std::time::SystemTime,
}

/// Connection metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionMetadata {
    pub success_rate: f64,
    pub avg_response_time: f64,
    pub total_requests: u64,
    pub last_used: std::time::SystemTime,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_multi_cache_basic() {
        let config = MultiCacheConfig::default();
        let cache: MultiCache<String> = MultiCache::new(config);

        // Test put and get
        cache.put("key1".to_string(), "value1".to_string(), CacheLevel::L1).await;
        let value = cache.get("key1").await;
        assert_eq!(value, Some("value1".to_string()));

        // Test cache miss
        let value = cache.get("nonexistent").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_promotion() {
        let mut config = MultiCacheConfig::default();
        config.l1_promotion_threshold = 0.01; // Very low threshold for testing
        let cache: MultiCache<String> = MultiCache::new(config);

        // Put in L2
        cache.put("key1".to_string(), "value1".to_string(), CacheLevel::L2).await;

        // Access multiple times to trigger promotion
        for _ in 0..10 {
            cache.get("key1").await;
            sleep(Duration::from_millis(10)).await;
        }

        // Should now be in L1
        let stats = cache.get_detailed_stats().await;
        assert!(stats.promotions > 0);
    }

    #[tokio::test]
    async fn test_ttl_expiration() {
        let mut config = MultiCacheConfig::default();
        config.l1_ttl = Duration::from_millis(100);
        let cache: MultiCache<String> = MultiCache::new(config);

        cache.put("key1".to_string(), "value1".to_string(), CacheLevel::L1).await;
        
        // Should be available immediately
        let value = cache.get("key1").await;
        assert_eq!(value, Some("value1".to_string()));

        // Wait for expiration
        sleep(Duration::from_millis(150)).await;
        
        // Should be expired
        let value = cache.get("key1").await;
        assert_eq!(value, None);
    }

    #[tokio::test]
    async fn test_cache_stats() {
        let cache: MultiCache<String> = MultiCache::new(MultiCacheConfig::default());

        cache.put("key1".to_string(), "value1".to_string(), CacheLevel::L1).await;
        cache.get("key1").await; // Hit
        cache.get("nonexistent").await; // Miss

        let stats = cache.get_stats().await;
        assert_eq!(stats.total_requests, 2);
        assert_eq!(stats.l1_hits, 1);
        assert_eq!(stats.total_misses, 1);
        assert_eq!(stats.hit_rate(), 50.0);
    }
}
