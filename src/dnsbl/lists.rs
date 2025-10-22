//! DNSBL lists configuration and management

use crate::dnsbl::DnsblResponseFormat;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a single DNSBL list
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsblList {
    /// Unique identifier for the list
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// DNS zone for this list
    pub zone: String,
    /// Description of what this list tracks
    pub description: String,
    /// Category of threats this list covers
    pub category: DnsblCategory,
    /// Whether this list is enabled by default
    pub default_enabled: bool,
    /// Expected response format
    pub response_format: DnsblResponseFormat,
    /// Priority for speed optimization (lower = faster/more reliable)
    pub priority: u8,
    /// Average response time in milliseconds (for optimization)
    pub avg_response_time_ms: u32,
}

/// Categories of DNSBL lists
#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum DnsblCategory {
    /// Spam sources
    Spam,
    /// Malware distribution
    Malware,
    /// Botnet command and control
    Botnet,
    /// Open proxies
    Proxy,
    /// Compromised hosts
    Compromised,
    /// General threats
    Threat,
    /// Reputation-based
    Reputation,
}

/// Collection of DNSBL lists
#[derive(Debug, Clone)]
pub struct DnsblLists {
    lists: HashMap<String, DnsblList>,
}

impl DnsblLists {
    /// Create new DNSBL lists collection with default lists
    pub fn new() -> Self {
        let mut lists = HashMap::new();
        
        // Spamhaus lists - highest priority (fastest and most reliable)
        lists.insert("zen".to_string(), DnsblList {
            id: "zen".to_string(),
            name: "Spamhaus ZEN".to_string(),
            zone: "zen.spamhaus.org".to_string(),
            description: "Combined Spamhaus lists (SBL+XBL+PBL)".to_string(),
            category: DnsblCategory::Spam,
            default_enabled: true,
            response_format: DnsblResponseFormat::Standard,
            priority: 1, // Highest priority - most comprehensive
            avg_response_time_ms: 50,
        });
        
        lists.insert("sbl".to_string(), DnsblList {
            id: "sbl".to_string(),
            name: "Spamhaus SBL".to_string(),
            zone: "sbl.spamhaus.org".to_string(),
            description: "Spamhaus Block List - Verified spam sources".to_string(),
            category: DnsblCategory::Spam,
            default_enabled: false, // Disabled since ZEN includes it
            response_format: DnsblResponseFormat::Standard,
            priority: 2,
            avg_response_time_ms: 45,
        });
        
        lists.insert("xbl".to_string(), DnsblList {
            id: "xbl".to_string(),
            name: "Spamhaus XBL".to_string(),
            zone: "xbl.spamhaus.org".to_string(),
            description: "Exploits Block List - Illegal 3rd party exploits".to_string(),
            category: DnsblCategory::Malware,
            default_enabled: false, // Disabled since ZEN includes it
            response_format: DnsblResponseFormat::Standard,
            priority: 2,
            avg_response_time_ms: 45,
        });
        
        lists.insert("pbl".to_string(), DnsblList {
            id: "pbl".to_string(),
            name: "Spamhaus PBL".to_string(),
            zone: "pbl.spamhaus.org".to_string(),
            description: "Policy Block List - Dynamic/Residential IPs".to_string(),
            category: DnsblCategory::Spam,
            default_enabled: false, // Less relevant for proxies
            response_format: DnsblResponseFormat::Standard,
            priority: 5, // Lower priority - many false positives
            avg_response_time_ms: 40,
        });
        
        // Fast secondary lists
        lists.insert("barracuda".to_string(), DnsblList {
            id: "barracuda".to_string(),
            name: "Barracuda Central".to_string(),
            zone: "b.barracudacentral.org".to_string(),
            description: "Barracuda Reputation Block List".to_string(),
            category: DnsblCategory::Reputation,
            default_enabled: true,
            response_format: DnsblResponseFormat::Standard,
            priority: 2, // Fast and reliable
            avg_response_time_ms: 60,
        });
        
        lists.insert("dronebl".to_string(), DnsblList {
            id: "dronebl".to_string(),
            name: "DroneBL".to_string(),
            zone: "dnsbl.dronebl.org".to_string(),
            description: "DroneBL - Abused infected bots, spammers".to_string(),
            category: DnsblCategory::Botnet,
            default_enabled: true,
            response_format: DnsblResponseFormat::Standard,
            priority: 2, // Good for botnet detection
            avg_response_time_ms: 70,
        });
        
        // Spamcop - slower but valuable
        lists.insert("spamcop".to_string(), DnsblList {
            id: "spamcop".to_string(),
            name: "SpamCop".to_string(),
            zone: "bl.spamcop.net".to_string(),
            description: "SpamCop Blocking List - Reported spam sources".to_string(),
            category: DnsblCategory::Spam,
            default_enabled: false, // Disabled by default for speed
            response_format: DnsblResponseFormat::Standard,
            priority: 4, // Slower but comprehensive
            avg_response_time_ms: 120,
        });
        
        // Specialized lists - lower priority
        lists.insert("projecthoneypot".to_string(), DnsblList {
            id: "projecthoneypot".to_string(),
            name: "Project Honeypot".to_string(),
            zone: "dnsbl.httpbl.org".to_string(),
            description: "Project Honeypot - HTTP comment spammers, harvesters".to_string(),
            category: DnsblCategory::Proxy,
            default_enabled: false, // Requires API key, disabled for simplicity
            response_format: DnsblResponseFormat::Standard,
            priority: 6, // Specialized use case
            avg_response_time_ms: 80,
        });
        
        lists.insert("multisurbl".to_string(), DnsblList {
            id: "multisurbl".to_string(),
            name: "MultiSURBL".to_string(),
            zone: "multi.surbl.org".to_string(),
            description: "MultiSURBL - Multiple spam URI blocklists".to_string(),
            category: DnsblCategory::Spam,
            default_enabled: false, // Less relevant for proxy checking
            response_format: DnsblResponseFormat::Standard,
            priority: 5, // URI-focused, less relevant for IP checking
            avg_response_time_ms: 90,
        });
        
        lists.insert("emergingthreats".to_string(), DnsblList {
            id: "emergingthreats".to_string(),
            name: "Emerging Threats".to_string(),
            zone: "bl.spameatingmonkey.net".to_string(),
            description: "Emerging Threats - Compromised hosts, botnets".to_string(),
            category: DnsblCategory::Compromised,
            default_enabled: false, // Less reliable
            response_format: DnsblResponseFormat::Standard,
            priority: 7, // Lower reliability
            avg_response_time_ms: 150,
        });
        
        Self { lists }
    }
    
    /// Get all default enabled lists
    pub fn get_default_enabled(&self) -> Vec<&DnsblList> {
        self.lists
            .values()
            .filter(|list| list.default_enabled)
            .collect()
    }
    
    /// Get all lists
    pub fn get_all(&self) -> Vec<&DnsblList> {
        self.lists.values().collect()
    }
    
    /// Get list by ID
    pub fn get_by_id(&self, id: &str) -> Option<&DnsblList> {
        self.lists.get(id)
    }
    
    /// Get lists by category
    pub fn get_by_category(&self, category: &DnsblCategory) -> Vec<&DnsblList> {
        self.lists
            .values()
            .filter(|list| &list.category == category)
            .collect()
    }
    
    /// Filter lists based on inclusion/exclusion criteria
    pub fn filter_lists(
        &self,
        include: &[String],
        exclude: &[String],
    ) -> Vec<&DnsblList> {
        let mut lists: Vec<&DnsblList> = if include.is_empty() {
            self.get_default_enabled()
        } else {
            include
                .iter()
                .filter_map(|id| self.get_by_id(id))
                .collect()
        };
        
        // Apply exclusions
        if !exclude.is_empty() {
            lists.retain(|list| !exclude.contains(&list.id));
        }
        
        // Sort by priority (lower priority = faster/more reliable)
        lists.sort_by(|a, b| a.priority.cmp(&b.priority));
        
        lists
    }
    
    /// Get lists sorted by priority for optimal performance
    pub fn get_lists_by_priority(&self, include: &[String], exclude: &[String]) -> Vec<&DnsblList> {
        let mut lists = self.filter_lists(include, exclude);
        
        // Additional sorting by average response time as secondary criteria
        lists.sort_by(|a, b| {
            a.priority.cmp(&b.priority)
                .then_with(|| a.avg_response_time_ms.cmp(&b.avg_response_time_ms))
        });
        
        lists
    }
    
    /// Get fast lists only (priority <= 3) for quick checks
    pub fn get_fast_lists(&self, include: &[String], exclude: &[String]) -> Vec<&DnsblList> {
        let lists = self.get_lists_by_priority(include, exclude);
        lists.into_iter()
            .filter(|list| list.priority <= 3)
            .collect()
    }
    
    /// Get statistics about the lists
    pub fn get_stats(&self) -> DnsblStats {
        let total = self.lists.len();
        let enabled = self.lists.values().filter(|l| l.default_enabled).count();
        let mut by_category = HashMap::new();
        
        for list in self.lists.values() {
            *by_category.entry(list.category.clone()).or_insert(0) += 1;
        }
        
        DnsblStats {
            total_lists: total,
            default_enabled: enabled,
            by_category,
        }
    }
}

/// Statistics about DNSBL lists
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsblStats {
    pub total_lists: usize,
    pub default_enabled: usize,
    pub by_category: HashMap<DnsblCategory, usize>,
}

impl Default for DnsblLists {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert IP address to DNSBL query format
pub fn ip_to_dnsbl_format(ip: &str) -> Result<String, String> {
    if ip.contains(':') {
        // IPv6 - not currently supported by most DNSBLs
        return Err("IPv6 not supported for DNSBL queries".to_string());
    }
    
    let parts: Vec<&str> = ip.split('.').collect();
    if parts.len() != 4 {
        return Err("Invalid IPv4 address format".to_string());
    }
    
    // Reverse IP for DNSBL query: 1.2.3.4 -> 4.3.2.1
    Ok(format!("{}.{}.{}.{}", parts[3], parts[2], parts[1], parts[0]))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_ip_to_dnsbl_format() {
        assert_eq!(ip_to_dnsbl_format("192.168.1.1").expect("Invalid IP format"), "1.1.168.192");
        assert_eq!(ip_to_dnsbl_format("8.8.8.8").expect("Invalid IP format"), "8.8.8.8");
        assert!(ip_to_dnsbl_format("invalid").is_err());
        assert!(ip_to_dnsbl_format("2001:db8::1").is_err());
    }
    
    #[test]
    fn test_dnsbl_lists_creation() {
        let lists = DnsblLists::new();
        let default_enabled = lists.get_default_enabled();
        assert!(!default_enabled.is_empty());
        
        let sbl = lists.get_by_id("sbl");
        assert!(sbl.is_some());
        assert_eq!(sbl.expect("Invalid DNSBL list").zone, "sbl.spamhaus.org");
    }
    
    #[test]
    fn test_filter_lists() {
        let lists = DnsblLists::new();
        
        // Test inclusion filter
        let included = lists.filter_lists(&["sbl".to_string(), "xbl".to_string()], &[]);
        assert_eq!(included.len(), 2);
        
        // Test exclusion filter
        let excluded = lists.filter_lists(&[], &["pbl".to_string()]);
        let pbl_found = excluded.iter().any(|l| l.id == "pbl");
        assert!(!pbl_found);
    }
}
