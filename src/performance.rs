//! Performance monitoring and optimization utilities

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};

/// Performance metrics collector
#[derive(Debug, Clone)]
pub struct PerformanceMonitor {
    metrics: Arc<RwLock<PerformanceMetrics>>,
}

/// Performance metrics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// DNSBL check metrics
    pub dnsbl_metrics: DnsblMetrics,
    /// Proxy check metrics
    pub proxy_metrics: ProxyMetrics,
    /// Network metrics
    pub network_metrics: NetworkMetrics,
    /// Cache metrics
    pub cache_metrics: CacheMetrics,
    /// System metrics
    pub system_metrics: SystemMetrics,
}

/// DNSBL performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DnsblMetrics {
    /// Total DNSBL checks performed
    pub total_checks: u64,
    /// Successful DNSBL checks
    pub successful_checks: u64,
    /// Failed DNSBL checks
    pub failed_checks: u64,
    /// Total DNSBL check time (milliseconds)
    pub total_check_time_ms: u64,
    /// Average DNSBL check time
    pub avg_check_time_ms: f64,
    /// Fastest DNSBL check time
    pub fastest_check_ms: u64,
    /// Slowest DNSBL check time
    pub slowest_check_ms: u64,
    /// DNSBL cache hit rate
    pub cache_hit_rate: f64,
    /// DNSBL lists checked per IP (average)
    pub avg_lists_per_check: f64,
    /// Early termination count
    pub early_terminations: u64,
    /// Early termination rate
    pub early_termination_rate: f64,
}

/// Proxy check performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProxyMetrics {
    /// Total proxy checks performed
    pub total_checks: u64,
    /// Successful proxy checks
    pub successful_checks: u64,
    /// Failed proxy checks
    pub failed_checks: u64,
    /// Total proxy check time (milliseconds)
    pub total_check_time_ms: u64,
    /// Average proxy check time
    pub avg_check_time_ms: f64,
    /// Proxies rejected by DNSBL
    pub dnsbl_rejections: u64,
    /// DNSBL rejection rate
    pub dnsbl_rejection_rate: f64,
    /// Protocol success rates
    pub protocol_success_rates: HashMap<String, f64>,
}

/// Network performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkMetrics {
    /// Total network requests
    pub total_requests: u64,
    /// Successful network requests
    pub successful_requests: u64,
    /// Failed network requests
    pub failed_requests: u64,
    /// Average response time (milliseconds)
    pub avg_response_time_ms: f64,
    /// Connection pool utilization
    pub connection_pool_utilization: f64,
    /// DNS query times
    pub dns_query_times: Vec<u64>,
    /// Average DNS query time
    pub avg_dns_query_time_ms: f64,
}

/// Cache performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheMetrics {
    /// Total cache operations
    pub total_operations: u64,
    /// Cache hits
    pub cache_hits: u64,
    /// Cache misses
    pub cache_misses: u64,
    /// Cache hit rate
    pub hit_rate: f64,
    /// Cache evictions
    pub evictions: u64,
    /// Average cache retrieval time (microseconds)
    pub avg_retrieval_time_us: f64,
}

/// System performance metrics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemMetrics {
    /// Memory usage (bytes)
    pub memory_usage_bytes: u64,
    /// CPU usage percentage
    pub cpu_usage_percent: f64,
    /// Active connections
    pub active_connections: u64,
    /// Concurrent operations
    pub concurrent_operations: u64,
    /// Uptime (seconds)
    pub uptime_seconds: u64,
    /// Total async task execution time
    pub async_task_time_ms: f64,
    /// Number of completed async tasks
    pub async_tasks_completed: u64,
    /// Average async task execution time
    pub avg_async_task_time_ms: f64,
}

impl PerformanceMonitor {
    /// Create new performance monitor
    pub fn new() -> Self {
        Self {
            metrics: Arc::new(RwLock::new(PerformanceMetrics::default())),
        }
    }

    /// Record DNSBL check performance
    pub async fn record_dnsbl_check(&self, duration: Duration, lists_checked: usize, early_termination: bool) {
        let mut metrics = self.metrics.write().await;
        let dnsbl = &mut metrics.dnsbl_metrics;
        
        let duration_ms = duration.as_millis() as u64;
        
        dnsbl.total_checks += 1;
        dnsbl.total_check_time_ms += duration_ms;
        dnsbl.avg_check_time_ms = dnsbl.total_check_time_ms as f64 / dnsbl.total_checks as f64;
        
        if dnsbl.fastest_check_ms == 0 || duration_ms < dnsbl.fastest_check_ms {
            dnsbl.fastest_check_ms = duration_ms;
        }
        if duration_ms > dnsbl.slowest_check_ms {
            dnsbl.slowest_check_ms = duration_ms;
        }
        
        // Update average lists per check
        dnsbl.avg_lists_per_check = (dnsbl.avg_lists_per_check * (dnsbl.total_checks - 1) as f64 + lists_checked as f64) / dnsbl.total_checks as f64;
        
        if early_termination {
            dnsbl.early_terminations += 1;
        }
        dnsbl.early_termination_rate = dnsbl.early_terminations as f64 / dnsbl.total_checks as f64;
    }

    /// Record DNSBL check result
    pub async fn record_dnsbl_result(&self, success: bool, cache_hit: bool) {
        let mut metrics = self.metrics.write().await;
        let dnsbl = &mut metrics.dnsbl_metrics;
        
        if success {
            dnsbl.successful_checks += 1;
        } else {
            dnsbl.failed_checks += 1;
        }
        
        if cache_hit {
            // Update cache hit rate (simplified - in practice would need more sophisticated tracking)
            dnsbl.cache_hit_rate = (dnsbl.cache_hit_rate * 0.9) + (1.0 * 0.1); // Exponential moving average
        } else {
            dnsbl.cache_hit_rate = dnsbl.cache_hit_rate * 0.9; // Decay
        }
    }

    /// Record proxy check performance
    pub async fn record_proxy_check(&self, duration: Duration, success: bool, dnsbl_rejected: bool) {
        let mut metrics = self.metrics.write().await;
        let proxy = &mut metrics.proxy_metrics;
        
        let duration_ms = duration.as_millis() as u64;
        
        proxy.total_checks += 1;
        proxy.total_check_time_ms += duration_ms;
        proxy.avg_check_time_ms = proxy.total_check_time_ms as f64 / proxy.total_checks as f64;
        
        if success {
            proxy.successful_checks += 1;
        } else {
            proxy.failed_checks += 1;
        }
        
        if dnsbl_rejected {
            proxy.dnsbl_rejections += 1;
        }
        proxy.dnsbl_rejection_rate = proxy.dnsbl_rejections as f64 / proxy.total_checks as f64;
    }

    /// Record network request performance
    pub async fn record_network_request(&self, duration: Duration, success: bool) {
        let mut metrics = self.metrics.write().await;
        let network = &mut metrics.network_metrics;
        
        let duration_ms = duration.as_millis() as u64;
        
        network.total_requests += 1;
        if success {
            network.successful_requests += 1;
        } else {
            network.failed_requests += 1;
        }
        
        // Update average response time
        network.avg_response_time_ms = (network.avg_response_time_ms * (network.total_requests - 1) as f64 + duration_ms as f64) / network.total_requests as f64;
    }

    /// Record DNS query performance
    pub async fn record_dns_query(&self, duration: Duration) {
        let mut metrics = self.metrics.write().await;
        let network = &mut metrics.network_metrics;
        
        let duration_ms = duration.as_millis() as u64;
        network.dns_query_times.push(duration_ms);
        
        // Keep only last 1000 query times for memory efficiency
        if network.dns_query_times.len() > 1000 {
            network.dns_query_times.remove(0);
        }
        
        // Calculate average
        let sum: u64 = network.dns_query_times.iter().sum();
        network.avg_dns_query_time_ms = sum as f64 / network.dns_query_times.len() as f64;
    }

    /// Record cache operation
    pub async fn record_cache_operation(&self, hit: bool, retrieval_time: Duration) {
        let mut metrics = self.metrics.write().await;
        let cache = &mut metrics.cache_metrics;
        
        cache.total_operations += 1;
        if hit {
            cache.cache_hits += 1;
        } else {
            cache.cache_misses += 1;
        }
        
        cache.hit_rate = cache.cache_hits as f64 / cache.total_operations as f64;
        
        let retrieval_time_us = retrieval_time.as_micros() as f64;
        cache.avg_retrieval_time_us = (cache.avg_retrieval_time_us * (cache.total_operations - 1) as f64 + retrieval_time_us) / cache.total_operations as f64;
    }

    /// Record protocol success
    pub async fn record_protocol_success(&self, protocol: &str, success: bool) {
        let mut metrics = self.metrics.write().await;
        let proxy = &mut metrics.proxy_metrics;
        
        let rate = proxy.protocol_success_rates.entry(protocol.to_string()).or_insert(0.0);
        *rate = (*rate * 0.9) + (if success { 1.0 } else { 0.0 } * 0.1); // Exponential moving average
    }

    /// Update system metrics
    pub async fn update_system_metrics(&self, memory_usage: u64, cpu_usage: f64, active_connections: u64, concurrent_operations: u64) {
        let mut metrics = self.metrics.write().await;
        let system = &mut metrics.system_metrics;
        
        system.memory_usage_bytes = memory_usage;
        system.cpu_usage_percent = cpu_usage;
        system.active_connections = active_connections;
        system.concurrent_operations = concurrent_operations;
    }

    /// Get current metrics snapshot
    pub async fn get_metrics(&self) -> PerformanceMetrics {
        self.metrics.read().await.clone()
    }

    /// Get performance summary
    pub async fn get_performance_summary(&self) -> PerformanceSummary {
        let metrics = self.metrics.read().await.clone();
        
        PerformanceSummary {
            overall_score: self.calculate_overall_score(&metrics),
            dnsbl_efficiency: self.calculate_dnsbl_efficiency(&metrics),
            proxy_throughput: self.calculate_proxy_throughput(&metrics),
            network_performance: self.calculate_network_performance(&metrics),
            cache_efficiency: metrics.cache_metrics.hit_rate,
            recommendations: self.generate_recommendations(&metrics),
        }
    }

    /// Calculate overall performance score (0-100)
    fn calculate_overall_score(&self, metrics: &PerformanceMetrics) -> f64 {
        let dnsbl_score = if metrics.dnsbl_metrics.avg_check_time_ms > 0.0 {
            (100.0 / (1.0 + metrics.dnsbl_metrics.avg_check_time_ms / 100.0)).min(100.0)
        } else {
            100.0
        };
        
        let proxy_score = if metrics.proxy_metrics.avg_check_time_ms > 0.0 {
            (100.0 / (1.0 + metrics.proxy_metrics.avg_check_time_ms / 1000.0)).min(100.0)
        } else {
            100.0
        };
        
        let cache_score = metrics.cache_metrics.hit_rate * 100.0;
        
        (dnsbl_score + proxy_score + cache_score) / 3.0
    }

    /// Calculate DNSBL efficiency
    fn calculate_dnsbl_efficiency(&self, metrics: &PerformanceMetrics) -> f64 {
        let dnsbl = &metrics.dnsbl_metrics;
        
        if dnsbl.total_checks == 0 {
            return 100.0;
        }
        
        let speed_score = if dnsbl.avg_check_time_ms > 0.0 {
            (100.0 / (1.0 + dnsbl.avg_check_time_ms / 50.0)).min(100.0)
        } else {
            100.0
        };
        
        let early_termination_bonus = dnsbl.early_termination_rate * 20.0;
        let cache_bonus = dnsbl.cache_hit_rate * 30.0;
        
        (speed_score + early_termination_bonus + cache_bonus).min(100.0)
    }

    /// Calculate proxy throughput
    fn calculate_proxy_throughput(&self, metrics: &PerformanceMetrics) -> f64 {
        let proxy = &metrics.proxy_metrics;
        
        if proxy.total_checks == 0 {
            return 0.0;
        }
        
        let success_rate = proxy.successful_checks as f64 / proxy.total_checks as f64;
        let speed_factor = if proxy.avg_check_time_ms > 0.0 {
            1000.0 / proxy.avg_check_time_ms
        } else {
            1.0
        };
        
        success_rate * speed_factor * 60.0 // Checks per minute
    }

    /// Calculate network performance
    fn calculate_network_performance(&self, metrics: &PerformanceMetrics) -> f64 {
        let network = &metrics.network_metrics;
        
        if network.total_requests == 0 {
            return 100.0;
        }
        
        let success_rate = network.successful_requests as f64 / network.total_requests as f64;
        let speed_score = if network.avg_response_time_ms > 0.0 {
            (100.0 / (1.0 + network.avg_response_time_ms / 200.0)).min(100.0)
        } else {
            100.0
        };
        
        (success_rate + speed_score) / 2.0
    }

    /// Generate performance recommendations
    fn generate_recommendations(&self, metrics: &PerformanceMetrics) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // DNSBL recommendations
        if metrics.dnsbl_metrics.avg_check_time_ms > 200.0 {
            recommendations.push("Consider reducing DNSBL timeout or using faster DNS servers".to_string());
        }
        
        if metrics.dnsbl_metrics.early_termination_rate < 0.3 {
            recommendations.push("Lower DNSBL threshold for better early termination performance".to_string());
        }
        
        if metrics.dnsbl_metrics.cache_hit_rate < 0.5 {
            recommendations.push("Increase DNSBL cache TTL for better hit rates".to_string());
        }
        
        // Proxy recommendations
        if metrics.proxy_metrics.avg_check_time_ms > 5000.0 {
            recommendations.push("Consider reducing proxy timeout or optimizing judge selection".to_string());
        }
        
        if metrics.proxy_metrics.dnsbl_rejection_rate > 0.5 {
            recommendations.push("High DNSBL rejection rate - consider adjusting DNSBL threshold".to_string());
        }
        
        // Network recommendations
        if metrics.network_metrics.avg_response_time_ms > 1000.0 {
            recommendations.push("Network latency is high - consider using faster judges or connection pooling".to_string());
        }
        
        // Cache recommendations
        if metrics.cache_metrics.hit_rate < 0.3 {
            recommendations.push("Low cache hit rate - consider increasing cache size or TTL".to_string());
        }
        
        recommendations
    }
}

/// Performance summary report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceSummary {
    /// Overall performance score (0-100)
    pub overall_score: f64,
    /// DNSBL efficiency score
    pub dnsbl_efficiency: f64,
    /// Proxy throughput (checks per minute)
    pub proxy_throughput: f64,
    /// Network performance score
    pub network_performance: f64,
    /// Cache efficiency (0-1)
    pub cache_efficiency: f64,
    /// Performance recommendations
    pub recommendations: Vec<String>,
}

impl PerformanceMetrics {
    /// Add async task execution time (for async_optimizer compatibility)
    pub fn add_async_task_time(&mut self, time_ms: f64) {
        self.system_metrics.async_task_time_ms += time_ms;
        self.system_metrics.async_tasks_completed += 1;
        self.system_metrics.avg_async_task_time_ms = self.system_metrics.async_task_time_ms / self.system_metrics.async_tasks_completed as f64;
    }
}

impl Default for PerformanceMonitor {
    fn default() -> Self {
        Self::new()
    }
}

/// Performance timer for measuring operation duration
pub struct PerformanceTimer {
    start_time: Instant,
    monitor: PerformanceMonitor,
    operation_type: OperationType,
}

#[derive(Debug, Clone)]
pub enum OperationType {
    DnsblCheck { lists_checked: usize },
    ProxyCheck,
    NetworkRequest,
    DnsQuery,
    CacheOperation { hit: bool },
}

impl PerformanceTimer {
    /// Create new performance timer
    pub fn new(monitor: PerformanceMonitor, operation_type: OperationType) -> Self {
        Self {
            start_time: Instant::now(),
            monitor,
            operation_type,
        }
    }

    /// Complete the timer and record the metrics
    pub async fn finish(self) {
        let duration = self.start_time.elapsed();
        
        match self.operation_type {
            OperationType::DnsblCheck { lists_checked } => {
                self.monitor.record_dnsbl_check(duration, lists_checked, false).await;
                self.monitor.record_dnsbl_result(true, false).await;
            }
            OperationType::ProxyCheck => {
                self.monitor.record_proxy_check(duration, true, false).await;
            }
            OperationType::NetworkRequest => {
                self.monitor.record_network_request(duration, true).await;
            }
            OperationType::DnsQuery => {
                self.monitor.record_dns_query(duration).await;
            }
            OperationType::CacheOperation { hit } => {
                self.monitor.record_cache_operation(hit, duration).await;
            }
        }
    }

    /// Complete the timer with custom result
    pub async fn finish_with_result(self, success: bool, additional_data: Option<PerformanceData>) {
        let duration = self.start_time.elapsed();
        
        match self.operation_type {
            OperationType::DnsblCheck { lists_checked } => {
                let early_termination = additional_data
                    .and_then(|d| match d {
                        PerformanceData::EarlyTermination(v) => Some(v),
                        _ => None,
                    })
                    .unwrap_or(false);
                
                self.monitor.record_dnsbl_check(duration, lists_checked, early_termination).await;
                self.monitor.record_dnsbl_result(success, false).await;
            }
            OperationType::ProxyCheck => {
                let dnsbl_rejected = additional_data
                    .and_then(|d| match d {
                        PerformanceData::DnsblRejected(v) => Some(v),
                        _ => None,
                    })
                    .unwrap_or(false);
                
                self.monitor.record_proxy_check(duration, success, dnsbl_rejected).await;
            }
            OperationType::NetworkRequest => {
                self.monitor.record_network_request(duration, success).await;
            }
            OperationType::DnsQuery => {
                self.monitor.record_dns_query(duration).await;
            }
            OperationType::CacheOperation { hit } => {
                self.monitor.record_cache_operation(hit, duration).await;
            }
        }
    }
}

/// Additional performance data
#[derive(Debug, Clone)]
pub enum PerformanceData {
    EarlyTermination(bool),
    DnsblRejected(bool),
}

impl Default for PerformanceMetrics {
    fn default() -> Self {
        Self {
            dnsbl_metrics: DnsblMetrics::default(),
            proxy_metrics: ProxyMetrics::default(),
            network_metrics: NetworkMetrics::default(),
            cache_metrics: CacheMetrics::default(),
            system_metrics: SystemMetrics::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    
    #[tokio::test]
    async fn test_performance_monitor_creation() {
        let monitor = PerformanceMonitor::new();
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.dnsbl_metrics.total_checks, 0);
    }
    
    #[tokio::test]
    async fn test_dnsbl_metrics_recording() {
        let monitor = PerformanceMonitor::new();
        
        monitor.record_dnsbl_check(Duration::from_millis(100), 3, false).await;
        monitor.record_dnsbl_result(true, false).await;
        
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.dnsbl_metrics.total_checks, 1);
        assert_eq!(metrics.dnsbl_metrics.successful_checks, 1);
        assert_eq!(metrics.dnsbl_metrics.avg_check_time_ms, 100.0);
        assert_eq!(metrics.dnsbl_metrics.avg_lists_per_check, 3.0);
    }
    
    #[tokio::test]
    async fn test_performance_timer() {
        let monitor = PerformanceMonitor::new();
        let timer = PerformanceTimer::new(monitor.clone(), OperationType::DnsblCheck { lists_checked: 2 });
        
        tokio::time::sleep(Duration::from_millis(10)).await;
        timer.finish().await;
        
        let metrics = monitor.get_metrics().await;
        assert_eq!(metrics.dnsbl_metrics.total_checks, 1);
        assert!(metrics.dnsbl_metrics.avg_check_time_ms >= 10.0);
    }
    
    #[tokio::test]
    async fn test_performance_summary() {
        let monitor = PerformanceMonitor::new();
        
        // Record some test data
        monitor.record_dnsbl_check(Duration::from_millis(50), 2, true).await;
        monitor.record_proxy_check(Duration::from_millis(200), true, false).await;
        monitor.record_cache_operation(true, Duration::from_micros(100)).await;
        
        let summary = monitor.get_performance_summary().await;
        assert!(summary.overall_score > 0.0);
        assert!(summary.dnsbl_efficiency > 0.0);
        assert!(summary.proxy_throughput > 0.0);
    }
    
    #[tokio::test]
    async fn test_recommendations() {
        let monitor = PerformanceMonitor::new();
        
        // Record slow performance to trigger recommendations
        monitor.record_dnsbl_check(Duration::from_millis(300), 5, false).await;
        monitor.record_proxy_check(Duration::from_millis(6000), true, false).await;
        
        let summary = monitor.get_performance_summary().await;
        assert!(!summary.recommendations.is_empty());
        assert!(summary.recommendations.iter().any(|r| r.contains("DNSBL timeout")));
    }
}
