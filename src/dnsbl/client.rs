//! DNS client for DNSBL queries

use std::time::{Duration, Instant};

use crate::dnsbl::{DnsblList, DnsblResult, DnsblResponseFormat};
use hickory_resolver::{
    config::{ResolverConfig, ResolverOpts},
        AsyncResolver,
};
use hickory_resolver::name_server::TokioConnectionProvider;

/// DNS client for performing DNSBL queries
#[derive(Debug, Clone)]
pub struct DnsblClient {
    resolver: AsyncResolver<TokioConnectionProvider>,
    timeout: Duration,
}

impl DnsblClient {
    /// Create new DNSBL client with default settings
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::with_timeout(Duration::from_secs(5)).await
    }
    
    /// Create new DNSBL client with custom timeout
    pub async fn with_timeout(timeout: Duration) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Self::with_optimized_config(timeout, false).await
    }
    
    /// Create new DNSBL client with optimized configuration for speed
    pub async fn with_optimized_config(
        timeout: Duration,
        use_fast_dns: bool,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let resolver = if use_fast_dns {
            // Use fast public DNS servers for optimal performance
            let name_servers = vec![
                "1.1.1.1:53".parse().expect("Invalid Cloudflare DNS address"), // Cloudflare
                "8.8.8.8:53".parse().expect("Invalid Google DNS address"), // Google
                "1.0.0.1:53".parse().expect("Invalid Cloudflare backup DNS address"), // Cloudflare backup
            ];

            let config = ResolverConfig::from_parts(
                None,
                name_servers,
                vec![],
            );

            let mut opts = ResolverOpts::default();
            opts.timeout = timeout;
            opts.attempts = 2; // Limit retry attempts for speed
            opts.rotate = true; // Rotate between DNS servers for load balancing
            opts.ndots = 1; // Optimize for short domain names
            opts.cache_size = 1024; // Increase cache size

            AsyncResolver::tokio(config, opts)
        } else {
            AsyncResolver::tokio(ResolverConfig::default(), ResolverOpts::default())
        };
        
        Ok(Self { resolver, timeout })
    }
    
    /// Check a single IP against a single DNSBL list
    pub async fn check_ip_against_list(
        &self,
        ip: &str,
        list: &DnsblList,
    ) -> Result<DnsblResult, Box<dyn std::error::Error + Send + Sync>> {
        let start_time = Instant::now();
        
        // Convert IP to DNSBL format
        let reversed_ip = match crate::dnsbl::lists::ip_to_dnsbl_format(ip) {
            Ok(ip) => ip,
            Err(e) => {
                return Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed: false,
                    reason: Some(format!("Invalid IP format: {}", e)),
                    response_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        };
        
        // Construct DNS query
        let query_domain = format!("{}.{}", reversed_ip, list.zone);
        
        // Perform DNS lookup based on expected response format
        let result = match list.response_format {
            DnsblResponseFormat::Standard => {
                self.check_simple_response(&query_domain, list, start_time).await
            }
            DnsblResponseFormat::Text => {
                self.check_text_response(&query_domain, list, start_time).await
            }
            DnsblResponseFormat::Both => {
                // Try both methods, preferring the standard response
                match self.check_simple_response(&query_domain, list, start_time).await {
                    Ok(result) if result.listed => Ok(result),
                    _ => self.check_text_response(&query_domain, list, start_time).await
                }
            }
        };
        
        result
    }
    
    /// Check IP against multiple DNSBL lists concurrently
    pub async fn check_ip_against_lists(
        &self,
        ip: &str,
        lists: &[&DnsblList],
        max_concurrent: usize,
    ) -> Vec<DnsblResult> {
        use futures_util::stream::{FuturesUnordered, StreamExt};
        
        let mut futures = FuturesUnordered::new();
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(max_concurrent));
        
        // Create tasks for each list
        for list in lists {
            let permit = semaphore.clone().acquire_owned().await
                .expect("Failed to acquire semaphore permit for DNSBL check");
            let client = self.clone();
            let ip = ip.to_string();
            let list = (*list).clone(); // Dereference and clone

            futures.push(tokio::spawn(async move {
                let _permit = permit;
                client.check_ip_against_list(&ip, &list).await
            }));
        }
        
        // Collect results
        let mut results = Vec::new();
        while let Some(result) = futures.next().await {
            match result {
                Ok(Ok(dnsbl_result)) => results.push(dnsbl_result),
                Ok(Err(e)) => {
                    log::warn!("DNSBL query failed: {}", e);
                    // Create a failed result
                    results.push(DnsblResult {
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
        
        results
    }
    
    /// Check with simple A record response (any response = listed)
    async fn check_simple_response(
        &self,
        query_domain: &str,
        list: &DnsblList,
        start_time: Instant,
    ) -> Result<DnsblResult, Box<dyn std::error::Error + Send + Sync>> {
        match tokio::time::timeout(self.timeout, self.resolver.ipv4_lookup(query_domain)).await {
            Ok(Ok(lookup)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                let listed = !lookup.iter().next().is_none();
                
                Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed,
                    reason: if listed { Some("Listed in DNSBL".to_string()) } else { None },
                    response_time_ms: response_time,
                })
            }
            Ok(Err(e)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed: false,
                    reason: Some(format!("DNS lookup failed: {}", e)),
                    response_time_ms: response_time,
                })
            }
            Err(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed: false,
                    reason: Some("DNS lookup timeout".to_string()),
                    response_time_ms: response_time,
                })
            }
        }
    }
    
    /// Check with IP address response (specific codes indicate listing type)
    async fn check_ip_response(
        &self,
        query_domain: &str,
        list: &DnsblList,
        start_time: Instant,
    ) -> Result<DnsblResult, Box<dyn std::error::Error + Send + Sync>> {
        match tokio::time::timeout(self.timeout, self.resolver.ipv4_lookup(query_domain)).await {
            Ok(Ok(lookup)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                
                if let Some(ip_addr) = lookup.iter().next() {
                    let ip_str = ip_addr.to_string();
                    let reason = self.interpret_ip_response(&ip_str, list);
                    
                    Ok(DnsblResult {
                        list_name: list.id.clone(),
                        listed: true,
                        reason: Some(reason),
                        response_time_ms: response_time,
                    })
                } else {
                    Ok(DnsblResult {
                        list_name: list.id.clone(),
                        listed: false,
                        reason: None,
                        response_time_ms: response_time,
                    })
                }
            }
            Ok(Err(e)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed: false,
                    reason: Some(format!("DNS lookup failed: {}", e)),
                    response_time_ms: response_time,
                })
            }
            Err(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed: false,
                    reason: Some("DNS lookup timeout".to_string()),
                    response_time_ms: response_time,
                })
            }
        }
    }
    
    /// Check with text response (TXT record)
    async fn check_text_response(
        &self,
        query_domain: &str,
        list: &DnsblList,
        start_time: Instant,
    ) -> Result<DnsblResult, Box<dyn std::error::Error + Send + Sync>> {
        match tokio::time::timeout(self.timeout, self.resolver.txt_lookup(query_domain)).await {
            Ok(Ok(lookup)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                
                if let Some(txt_record) = lookup.iter().next() {
                    let text_data = txt_record.to_string();
                    let reason = format!("Listed: {}", text_data);
                    
                    Ok(DnsblResult {
                        list_name: list.id.clone(),
                        listed: true,
                        reason: Some(reason),
                        response_time_ms: response_time,
                    })
                } else {
                    Ok(DnsblResult {
                        list_name: list.id.clone(),
                        listed: false,
                        reason: None,
                        response_time_ms: response_time,
                    })
                }
            }
            Ok(Err(e)) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed: false,
                    reason: Some(format!("DNS lookup failed: {}", e)),
                    response_time_ms: response_time,
                })
            }
            Err(_) => {
                let response_time = start_time.elapsed().as_millis() as u64;
                Ok(DnsblResult {
                    list_name: list.id.clone(),
                    listed: false,
                    reason: Some("DNS lookup timeout".to_string()),
                    response_time_ms: response_time,
                })
            }
        }
    }
    
    /// Interpret IP address response from DNSBL
    fn interpret_ip_response(&self, ip_str: &str, list: &DnsblList) -> String {
        match list.id.as_str() {
            "sbl" | "xbl" | "zen" => {
                // Spamhaus response codes
                match ip_str.as_ref() {
                    "127.0.0.2" => "Spamhaus SBL - Verified spam source".to_string(),
                    "127.0.0.3" => "Spamhaus SBL - Verified spam source".to_string(),
                    "127.0.0.4" => "Spamhaus XBL - Exploit bot net C&C".to_string(),
                    "127.0.0.5" => "Spamhaus XBL - Exploit bot net C&C".to_string(),
                    "127.0.0.6" => "Spamhaus XBL - Illegal 3rd party exploits".to_string(),
                    "127.0.0.7" => "Spamhaus XBL - Illegal 3rd party exploits".to_string(),
                    "127.0.0.10" => "Spamhaus PBL - ISP dynamic IP range".to_string(),
                    "127.0.0.11" => "Spamhaus PBL - ISP dynamic IP range".to_string(),
                    _ => format!("Spamhaus listed with response: {}", ip_str),
                }
            }
            "projecthoneypot" => {
                // Project Honeypot response codes
                match ip_str.as_ref() {
                    "127.0.0.2" => "Project Honeypot - Suspicious commenter".to_string(),
                    "127.0.0.3" => "Project Honeypot - Harvester".to_string(),
                    "127.0.0.4" => "Project Honeypot - Suspicious commenter + Harvester".to_string(),
                    "127.0.0.5" => "Project Honeypot - Comment spammer".to_string(),
                    "127.0.0.6" => "Project Honeypot - Suspicious commenter + Comment spammer".to_string(),
                    "127.0.0.7" => "Project Honeypot - Harvester + Comment spammer".to_string(),
                    _ => format!("Project Honeypot listed with response: {}", ip_str),
                }
            }
            "dronebl" => {
                // DroneBL response codes
                match ip_str.as_ref() {
                    "127.0.0.2" => "DroneBL - Sampled IP".to_string(),
                    "127.0.0.3" => "DroneBL - IRC Drone".to_string(),
                    "127.0.0.5" => "DroneBL - Bottler".to_string(),
                    "127.0.0.6" => "DroneBL - Unknown spambot or drone".to_string(),
                    "127.0.0.7" => "DroneBL - DDOS Drone".to_string(),
                    "127.0.0.8" => "DroneBL - Open SOCKS proxy".to_string(),
                    "127.0.0.9" => "DroneBL - Open HTTP proxy".to_string(),
                    "127.0.0.10" => "DroneBL - ProxyChain".to_string(),
                    "127.0.0.11" => "DroneBL - Web Page Proxy".to_string(),
                    "127.0.0.12" => "DroneBL - Open HTTP proxy (transparent)".to_string(),
                    "127.0.0.13" => "DroneBL - Open HTTP proxy (anonymous)".to_string(),
                    _ => format!("DroneBL listed with response: {}", ip_str),
                }
            }
            _ => format!("Listed with response: {}", ip_str),
        }
    }
    
    /// Test DNS connectivity
    pub async fn test_connectivity(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        match tokio::time::timeout(Duration::from_secs(5), self.resolver.ipv4_lookup("google.com")).await {
            Ok(Ok(_)) => Ok(true),
            Ok(Err(_)) | Err(_) => Ok(false),
        }
    }
}

// Note: Default implementation removed as it requires async initialization
// Use DnsblClient::new() or DnsblClient::with_timeout() instead

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dnsbl::lists::DnsblCategory;
    
    #[tokio::test]
    async fn test_dnsbl_client_creation() {
        let client = DnsblClient::new().await;
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_connectivity() {
        let client = DnsblClient::new().await.unwrap();
        let connectivity = client.test_connectivity().await.unwrap();
        // Should be true in most environments, but don't fail test if not
        log::info!("DNS connectivity: {}", connectivity);
    }
    
    #[tokio::test]
    async fn test_check_known_clean_ip() {
        let client = DnsblClient::new().await.unwrap();
        let list = DnsblList {
            id: "test".to_string(),
            name: "Test List".to_string(),
            zone: "zen.spamhaus.org".to_string(),
            description: "Test".to_string(),
            category: DnsblCategory::Spam,
            default_enabled: true,
            response_format: DnsblResponseFormat::Standard,
        };
        
        // Use Google's DNS as a known clean IP
        let result = client.check_ip_against_list("8.8.8.8", &list).await.unwrap();
        assert!(!result.listed); // Should not be listed
    }
    
    #[tokio::test]
    async fn test_invalid_ip_format() {
        let client = DnsblClient::new().await.unwrap();
        let list = DnsblList {
            id: "test".to_string(),
            name: "Test List".to_string(),
            zone: "zen.spamhaus.org".to_string(),
            description: "Test".to_string(),
            category: DnsblCategory::Spam,
            default_enabled: true,
            response_format: DnsblResponseFormat::Standard,
        };
        
        let result = client.check_ip_against_list("invalid", &list).await.unwrap();
        assert!(!result.listed);
        assert!(result.reason.is_some());
        assert!(result.reason.unwrap().contains("Invalid IP format"));
    }
}
