//! DNSBL (DNS-based Blackhole List) module for proxy.rs
//! 
//! This module provides functionality to check proxy IP addresses against
//! various DNS-based blacklists to identify potentially malicious or
//! compromised proxies.

pub mod client;
pub mod lists;
pub mod checker;
pub mod cache;

pub use client::DnsblClient;
pub use lists::{DnsblList, DnsblLists};
pub use checker::DnsblChecker;
pub use cache::DnsblCacheManager;

use serde::{Deserialize, Serialize};

/// DNSBL response format variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DnsblResponseFormat {
    /// Standard A record response (127.0.0.x)
    Standard,
    /// Text record response with details
    Text,
    /// Both A and TXT records
    Both,
}

impl Default for DnsblResponseFormat {
    fn default() -> Self {
        DnsblResponseFormat::Standard
    }
}

/// DNSBL check result for a single list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsblResult {
    /// The DNSBL list that was checked
    pub list_name: String,
    /// Whether the IP is listed in this DNSBL
    pub listed: bool,
    /// Additional information from the DNSBL response
    pub reason: Option<String>,
    /// Time taken for the check in milliseconds
    pub response_time_ms: u64,
}

/// Complete DNSBL check results for an IP
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsblCheckResults {
    /// The IP address that was checked
    pub ip: String,
    /// All DNSBL check results
    pub results: Vec<DnsblResult>,
    /// Total number of lists where the IP is listed
    pub listed_count: usize,
    /// Total number of lists checked
    pub total_checked: usize,
    /// Overall check time in milliseconds
    pub total_time_ms: u64,
    /// Whether the IP is considered malicious based on threshold
    pub is_malicious: bool,
}

/// DNSBL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsblConfig {
    /// Enable DNSBL checking
    pub enabled: bool,
    /// Timeout for DNSBL queries in seconds
    pub timeout_secs: u64,
    /// Maximum number of DNSBL lists to check concurrently
    pub max_concurrent: usize,
    /// Cache TTL in seconds
    pub cache_ttl_secs: u64,
    /// Maximum number of failed lists before considering an IP malicious
    pub malicious_threshold: usize,
    /// Specific DNSBL lists to use (empty = use default lists)
    pub specific_lists: Vec<String>,
    /// DNSBL lists to exclude
    pub excluded_lists: Vec<String>,
}

impl Default for DnsblConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            timeout_secs: 5,
            max_concurrent: 10,
            cache_ttl_secs: 3600, // 1 hour
            malicious_threshold: 2, // Listed in 2+ lists = malicious
            specific_lists: Vec::new(),
            excluded_lists: Vec::new(),
        }
    }
}

impl DnsblCheckResults {
    /// Create new DNSBL check results
    pub fn new(ip: String) -> Self {
        Self {
            ip,
            results: Vec::new(),
            listed_count: 0,
            total_checked: 0,
            total_time_ms: 0,
            is_malicious: false,
        }
    }

    /// Add a DNSBL result
    pub fn add_result(&mut self, result: DnsblResult) {
        if result.listed {
            self.listed_count += 1;
        }
        self.total_checked += 1;
        self.total_time_ms += result.response_time_ms;
        self.results.push(result);
    }

    /// Determine if IP is malicious based on threshold
    pub fn update_malicious_status(&mut self, threshold: usize) {
        self.is_malicious = self.listed_count >= threshold;
    }

    /// Get the listing rate (percentage of lists that flagged this IP)
    pub fn listing_rate(&self) -> f64 {
        if self.total_checked == 0 {
            0.0
        } else {
            (self.listed_count as f64 / self.total_checked as f64) * 100.0
        }
    }
}
