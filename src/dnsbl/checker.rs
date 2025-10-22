//! Main DNSBL checker that orchestrates the DNSBL checking process

use std::time::Instant;

use crate::dnsbl::{
    DnsblCacheManager, DnsblCheckResults, DnsblClient, DnsblConfig, DnsblList, DnsblLists,
};

/// Main DNSBL checker that coordinates all DNSBL operations
#[derive(Debug)]
pub struct DnsblChecker {
    /// DNS client for performing queries
    client: DnsblClient,
    /// Cache manager for storing results
    cache_manager: DnsblCacheManager,
    /// Available DNSBL lists
    lists: DnsblLists,
    /// Configuration
    config: DnsblConfig,
}

// Ensure DnsblChecker is Send and Clone
unsafe impl Send for DnsblChecker {}
unsafe impl Sync for DnsblChecker {}

impl Clone for DnsblChecker {
    fn clone(&self) -> Self {
        let config = self.config.clone();
        Self {
            client: self.client.clone(),
            cache_manager: DnsblCacheManager::new(config.clone()),
            lists: self.lists.clone(),
            config,
        }
    }
}

impl DnsblChecker {
    /// Create new DNSBL checker with default configuration
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::with_config(DnsblConfig::default()).await
    }
    
    /// Create new DNSBL checker with custom configuration
    pub async fn with_config(config: DnsblConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = DnsblClient::with_optimized_config(
            std::time::Duration::from_secs(config.timeout_secs),
            true, // Use fast DNS servers for optimal performance
        ).await?;
        
        let cache_manager = DnsblCacheManager::new(config.clone());
        let lists = DnsblLists::new();
        
        Ok(Self {
            client,
            cache_manager,
            lists,
            config,
        })
    }
    
    /// Check a single IP address against configured DNSBL lists
    pub async fn check_ip(&mut self, ip: &str) -> Result<DnsblCheckResults, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        
        log::info!("Starting DNSBL check for IP: {}", ip);
        
        // Check cache first
        if let Some(cached_results) = self.cache_manager.get(ip) {
            log::debug!("DNSBL check for {} completed from cache", ip);
            return Ok(cached_results);
        }
        
        // Perform checks with early termination - clone needed for borrow checker
        let results = self.check_ip_with_early_termination(ip).await;
        
        // Create results object
        let mut check_results = DnsblCheckResults::new(ip.to_string());
        for result in results {
            check_results.add_result(result);
        }
        
        // Determine if malicious based on threshold
        check_results.update_malicious_status(self.config.malicious_threshold);
        
        // Cache the results
        self.cache_manager.put(ip.to_string(), check_results.clone());
        
        let total_time = start_time.elapsed();
        log::info!(
            "DNSBL check for {} completed in {}ms - Listed: {}/{}, Malicious: {}",
            ip,
            total_time.as_millis(),
            check_results.listed_count,
            check_results.total_checked,
            check_results.is_malicious
        );
        
        Ok(check_results)
    }
    
    /// Check IP with early termination for performance optimization
    async fn check_ip_with_early_termination(
        &mut self,
        ip: &str,
    ) -> Vec<crate::dnsbl::DnsblResult> {
        // Get the lists to check, sorted by priority for optimal performance
        let specific_lists = self.config.specific_lists.clone();
        let excluded_lists = self.config.excluded_lists.clone();
        let lists = self.lists.get_lists_by_priority(
            &specific_lists,
            &excluded_lists,
        );

        if lists.is_empty() {
            log::warn!("No DNSBL lists configured for checking");
            return Vec::new();
        }
        use futures_util::stream::{FuturesUnordered, StreamExt};
        
        let mut results = Vec::new();
        let mut listed_count = 0;
        let mut futures = FuturesUnordered::new();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.max_concurrent));
        let threshold = self.config.malicious_threshold;

        // Create tasks for each list in priority order
        for (index, list) in lists.iter().enumerate() {
            let permit = if let Ok(permit) = semaphore.clone().acquire_owned().await {
                permit
            } else {
                log::error!("Failed to acquire semaphore permit");
                continue;
            };
            let client = self.client.clone();
            let ip = ip.to_string();
            let list = (*list).clone(); // Dereference and clone

            futures.push(tokio::spawn(async move {
                let _permit = permit;
                let result = client.check_ip_against_list(&ip, &list).await;
                (index, result)
            }));
        }
        
        // Process results as they complete
        while let Some(result) = futures.next().await {
            match result {
                Ok((index, Ok(dnsbl_result))) => {
                    if dnsbl_result.listed {
                        listed_count += 1;
                        log::debug!("IP {} listed in {} (priority: {})", ip, dnsbl_result.list_name, index);
                        
                        // Early termination: if we've reached the threshold, we can stop
                        if listed_count >= threshold {
                            log::info!("Early termination for IP {} - threshold reached ({}/{})", 
                                ip, listed_count, threshold);
                            
                            // Add the current result and break
                            results.push(dnsbl_result);
                            break;
                        }
                    }
                    results.push(dnsbl_result);
                }
                Ok((_, Err(e))) => {
                    log::warn!("DNSBL query failed: {}", e);
                    // Create a failed result but don't count towards threshold
                    results.push(crate::dnsbl::DnsblResult {
                        list_name: "unknown".to_string(),
                        listed: false,
                        reason: Some(format!("Query failed: {}", e)),
                        response_time_ms: 0,
                    });
                }
                Err(e) => {
                    log::warn!("DNSBL task failed: {}", e);
                }
            }
        }
        
        // Sort results back to original order for consistency
        results.sort_by_key(|r| r.list_name.clone());
        results
    }
    
    /// Check multiple IP addresses
    pub async fn check_ips(&mut self, ips: &[String]) -> Vec<DnsblCheckResults> {
        use futures_util::stream::{FuturesUnordered, StreamExt};
        
        let mut futures = FuturesUnordered::new();
        
        // Create tasks for each IP
        for ip in ips {
            let mut checker = self.clone();
            let ip = ip.clone();
            
            futures.push(tokio::spawn(async move {
                checker.check_ip(&ip).await
            }));
        }
        
        // Collect results
        let mut results = Vec::new();
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(check_results)) => results.push(check_results),
                Ok(Err(e)) => {
                    log::error!("DNSBL check failed: {}", e);
                }
                Err(e) => {
                    log::error!("DNSBL task failed: {}", e);
                }
            }
        }
        
        results
    }
    
    /// Check if an IP is malicious (convenience method)
    pub async fn is_ip_malicious(&mut self, ip: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let results = self.check_ip(ip).await?;
        Ok(results.is_malicious)
    }
    
    /// Get statistics about DNSBL lists
    pub fn get_lists_stats(&self) -> crate::dnsbl::lists::DnsblStats {
        self.lists.get_stats()
    }
    
    /// Get cache statistics
    pub fn get_cache_stats(&self) -> &crate::dnsbl::cache::CacheStats {
        self.cache_manager.get_stats()
    }
    
    /// Force cache cleanup
    pub fn cleanup_cache(&mut self) {
        self.cache_manager.force_cleanup();
    }
    
    /// Test DNSBL connectivity
    pub async fn test_connectivity(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        self.client.test_connectivity().await
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &DnsblConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: DnsblConfig) {
        self.config = config.clone();
        // Note: We don't update the client or cache manager here as it would require
        // recreating them. In a production system, you might want to handle this better.
    }
    
    /// Get available DNSBL lists
    pub fn get_available_lists(&self) -> Vec<&DnsblList> {
        self.lists.get_all()
    }
    
    /// Get enabled DNSBL lists based on current configuration
    pub fn get_enabled_lists(&self) -> Vec<&DnsblList> {
        self.lists.filter_lists(
            &self.config.specific_lists,
            &self.config.excluded_lists,
        )
    }
    
    /// Validate IP format for DNSBL checking
    pub fn validate_ip_format(ip: &str) -> bool {
        // Use the existing validation function
        crate::dnsbl::lists::ip_to_dnsbl_format(ip).is_ok()
    }
    
    /// Get recommended configuration for different use cases
    pub fn get_recommended_config(use_case: DnsblUseCase) -> DnsblConfig {
        match use_case {
            DnsblUseCase::Strict => DnsblConfig {
                enabled: true,
                timeout_secs: 10,
                max_concurrent: 5,
                cache_ttl_secs: 7200, // 2 hours
                malicious_threshold: 1, // Any listing = malicious
                specific_lists: vec![
                    "zen".to_string(),
                    "spamcop".to_string(),
                    "dronebl".to_string(),
                ],
                excluded_lists: vec![],
            },
            DnsblUseCase::Balanced => DnsblConfig {
                enabled: true,
                timeout_secs: 5,
                max_concurrent: 10,
                cache_ttl_secs: 3600, // 1 hour
                malicious_threshold: 2, // 2+ listings = malicious
                specific_lists: vec![],
                excluded_lists: vec!["pbl".to_string()], // Exclude policy lists
            },
            DnsblUseCase::Performance => DnsblConfig {
                enabled: true,
                timeout_secs: 3,
                max_concurrent: 20,
                cache_ttl_secs: 1800, // 30 minutes
                malicious_threshold: 3, // 3+ listings = malicious
                specific_lists: vec![
                    "zen".to_string(),
                    "barracuda".to_string(),
                ],
                excluded_lists: vec![],
            },
            DnsblUseCase::Testing => DnsblConfig {
                enabled: true,
                timeout_secs: 1,
                max_concurrent: 1,
                cache_ttl_secs: 60, // 1 minute
                malicious_threshold: 1,
                specific_lists: vec!["zen".to_string()],
                excluded_lists: vec![],
            },
        }
    }
}

/// DNSBL use cases for recommended configurations
#[derive(Debug, Clone, Copy)]
pub enum DnsblUseCase {
    /// Strict security checking (slowest, most thorough)
    Strict,
    /// Balanced approach between security and performance
    Balanced,
    /// Performance-focused (fastest, less thorough)
    Performance,
    /// Testing/development (minimal lists, fast)
    Testing,
}

/// DNSBL checking result summary
#[derive(Debug, Clone)]
pub struct DnsblSummary {
    /// Total IPs checked
    pub total_ips: usize,
    /// IPs found in DNSBLs
    pub listed_ips: usize,
    /// IPs considered malicious
    pub malicious_ips: usize,
    /// Total DNSBL lists checked
    pub total_lists_checked: usize,
    /// Average response time per IP
    pub avg_response_time_ms: f64,
    /// Cache hit rate
    pub cache_hit_rate: f64,
}

impl DnsblChecker {
    /// Generate summary from multiple check results
    pub fn generate_summary(results: &[DnsblCheckResults], cache_hit_rate: f64) -> DnsblSummary {
        let total_ips = results.len();
        let listed_ips = results.iter().filter(|r| r.listed_count > 0).count();
        let malicious_ips = results.iter().filter(|r| r.is_malicious).count();
        
        let total_response_time: u64 = results.iter().map(|r| r.total_time_ms).sum();
        let avg_response_time_ms = if total_ips > 0 {
            total_response_time as f64 / total_ips as f64
        } else {
            0.0
        };
        
        let total_lists_checked = results.iter().map(|r| r.total_checked).sum();
        
        DnsblSummary {
            total_ips,
            listed_ips,
            malicious_ips,
            total_lists_checked,
            avg_response_time_ms,
            cache_hit_rate,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_dnsbl_checker_creation() {
        let checker = DnsblChecker::new().await;
        assert!(checker.is_ok());
    }
    
    #[tokio::test]
    async fn test_dnsbl_checker_with_config() {
        let config = DnsblConfig {
            enabled: true,
            timeout_secs: 3,
            max_concurrent: 5,
            cache_ttl_secs: 1800,
            malicious_threshold: 2,
            specific_lists: vec![],
            excluded_lists: vec![],
        };
        
        let checker = DnsblChecker::with_config(config).await;
        assert!(checker.is_ok());
    }
    
    #[tokio::test]
    async fn test_ip_validation() {
        assert!(DnsblChecker::validate_ip_format("192.168.1.1"));
        assert!(DnsblChecker::validate_ip_format("8.8.8.8"));
        assert!(!DnsblChecker::validate_ip_format("invalid"));
        assert!(!DnsblChecker::validate_ip_format("2001:db8::1"));
    }
    
    #[tokio::test]
    async fn test_recommended_configs() {
        let strict_config = DnsblChecker::get_recommended_config(DnsblUseCase::Strict);
        assert_eq!(strict_config.malicious_threshold, 1);
        assert!(strict_config.specific_lists.contains(&"zen".to_string()));
        
        let performance_config = DnsblChecker::get_recommended_config(DnsblUseCase::Performance);
        assert_eq!(performance_config.malicious_threshold, 3);
        assert_eq!(performance_config.max_concurrent, 20);
        
        let testing_config = DnsblChecker::get_recommended_config(DnsblUseCase::Testing);
        assert_eq!(testing_config.cache_ttl_secs, 60);
        assert_eq!(testing_config.specific_lists.len(), 1);
    }
    
    #[tokio::test]
    async fn test_connectivity() {
        let checker = DnsblChecker::new().await.expect("Failed to create DNSBL checker");
        let connectivity = checker.test_connectivity().await.expect("Failed to test connectivity");
        log::info!("DNSBL connectivity: {}", connectivity);
        // Don't assert as this depends on network environment
    }
    
    #[tokio::test]
    async fn test_lists_stats() {
        let checker = DnsblChecker::new().await.expect("Failed to create DNSBL checker");
        let stats = checker.get_lists_stats();
        assert!(stats.total_lists > 0);
        assert!(stats.default_enabled > 0);
    }
    
    #[tokio::test]
    async fn test_enabled_lists() {
        let mut config = DnsblConfig::default();
        config.specific_lists = vec!["zen".to_string(), "spamcop".to_string()];
        
        let checker = DnsblChecker::with_config(config).await.expect("Failed to create DNSBL checker with config");
        let enabled_lists = checker.get_enabled_lists();
        
        assert_eq!(enabled_lists.len(), 2);
        assert!(enabled_lists.iter().any(|l| l.id == "zen"));
        assert!(enabled_lists.iter().any(|l| l.id == "spamcop"));
    }
    
    #[tokio::test]
    async fn test_summary_generation() {
        let mut results = Vec::new();
        
        // Create test results
        let mut result1 = DnsblCheckResults::new("192.168.1.1".to_string());
        result1.listed_count = 0;
        result1.total_checked = 5;
        result1.total_time_ms = 1000;
        result1.is_malicious = false;
        
        let mut result2 = DnsblCheckResults::new("192.168.1.2".to_string());
        result2.listed_count = 2;
        result2.total_checked = 5;
        result2.total_time_ms = 1500;
        result2.is_malicious = true;
        
        results.push(result1);
        results.push(result2);
        
        let summary = DnsblChecker::generate_summary(&results, 75.0);
        
        assert_eq!(summary.total_ips, 2);
        assert_eq!(summary.listed_ips, 1);
        assert_eq!(summary.malicious_ips, 1);
        assert_eq!(summary.total_lists_checked, 10);
        assert_eq!(summary.avg_response_time_ms, 1250.0);
        assert_eq!(summary.cache_hit_rate, 75.0);
    }
    
    #[tokio::test]
    async fn test_check_known_clean_ip() {
        let mut checker = DnsblChecker::new().await.expect("Failed to create DNSBL checker");
        
        // Use Google's DNS as a known clean IP
        let result = checker.check_ip("8.8.8.8").await.expect("Failed to check IP");
        
        assert_eq!(result.ip, "8.8.8.8");
        assert!(!result.is_malicious); // Should not be malicious
        assert!(result.total_checked > 0); // Should have checked some lists
    }
}
