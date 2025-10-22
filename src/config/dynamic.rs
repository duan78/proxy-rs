use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DynamicConfig {
    pub general: GeneralConfig,
    pub dnsbl: DnsblConfig,
    pub server: ServerConfig,
    pub protocols: ProtocolConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub max_connections: usize,
    pub default_timeout: u64,
    pub rate_limit_delay_ms: u64,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsblConfig {
    pub enabled: bool,
    pub timeout_secs: u64,
    pub max_concurrent: usize,
    pub cache_ttl_secs: u64,
    pub malicious_threshold: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub max_clients: usize,
    pub port: u16,
    pub timeout: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProtocolConfig {
    pub http: bool,
    pub https: bool,
    pub socks4: bool,
    pub socks5: bool,
    pub connect_25: bool,
    pub connect_80: bool,
}

impl DynamicConfig {
    pub fn new() -> Self {
        Self {
            general: GeneralConfig {
                max_connections: 5000,
                default_timeout: 8,
                rate_limit_delay_ms: 500,
                log_level: "info".to_string(),
            },
            dnsbl: DnsblConfig {
                enabled: true,
                timeout_secs: 5,
                max_concurrent: 10,
                cache_ttl_secs: 3600,
                malicious_threshold: 2,
            },
            server: ServerConfig {
                max_clients: 1000,
                port: 8080,
                timeout: 30,
            },
            protocols: ProtocolConfig {
                http: true,
                https: true,
                socks4: true,
                socks5: true,
                connect_25: true,
                connect_80: true,
            },
        }
    }

    pub fn update_section(&mut self, section: ConfigSection, new_config: serde_json::Value) -> Result<(), String> {
        match section {
            ConfigSection::General => {
                let new_general: GeneralConfig = serde_json::from_value(new_config)
                    .map_err(|e| format!("Invalid general config: {}", e))?;
                self.general = new_general;
            }
            ConfigSection::Dnsbl => {
                let new_dnsbl: DnsblConfig = serde_json::from_value(new_config)
                    .map_err(|e| format!("Invalid dnsbl config: {}", e))?;
                self.dnsbl = new_dnsbl;
            }
            ConfigSection::Server => {
                let new_server: ServerConfig = serde_json::from_value(new_config)
                    .map_err(|e| format!("Invalid server config: {}", e))?;
                self.server = new_server;
            }
            ConfigSection::Protocols => {
                let new_protocols: ProtocolConfig = serde_json::from_value(new_config)
                    .map_err(|e| format!("Invalid protocols config: {}", e))?;
                self.protocols = new_protocols;
            }
        }
        Ok(())
    }

    pub fn get_section_as_json(&self, section: ConfigSection) -> serde_json::Value {
        match section {
            ConfigSection::General => serde_json::to_value(&self.general).unwrap_or_default(),
            ConfigSection::Dnsbl => serde_json::to_value(&self.dnsbl).unwrap_or_default(),
            ConfigSection::Server => serde_json::to_value(&self.server).unwrap_or_default(),
            ConfigSection::Protocols => serde_json::to_value(&self.protocols).unwrap_or_default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConfigSection {
    General,
    Dnsbl,
    Server,
    Protocols,
}

impl std::fmt::Display for ConfigSection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigSection::General => write!(f, "general"),
            ConfigSection::Dnsbl => write!(f, "dnsbl"),
            ConfigSection::Server => write!(f, "server"),
            ConfigSection::Protocols => write!(f, "protocols"),
        }
    }
}

pub type SharedConfig = Arc<RwLock<DynamicConfig>>;