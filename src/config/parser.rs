use crate::config::{DynamicConfig, ConfigSection};
use crate::config::dynamic::{GeneralConfig, DnsblConfig, ServerConfig, ProtocolConfig};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct TomlConfig {
    pub general: Option<TomlGeneralConfig>,
    pub dnsbl: Option<TomlDnsblConfig>,
    pub server: Option<TomlServerConfig>,
    pub protocols: Option<TomlProtocolConfig>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TomlGeneralConfig {
    pub max_connections: Option<usize>,
    pub default_timeout: Option<u64>,
    pub rate_limit_delay_ms: Option<u64>,
    pub log_level: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TomlDnsblConfig {
    pub enabled: Option<bool>,
    pub timeout_secs: Option<u64>,
    pub max_concurrent: Option<usize>,
    pub cache_ttl_secs: Option<u64>,
    pub malicious_threshold: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TomlServerConfig {
    pub max_clients: Option<usize>,
    pub port: Option<u16>,
    pub timeout: Option<u64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TomlProtocolConfig {
    pub http: Option<bool>,
    pub https: Option<bool>,
    pub socks4: Option<bool>,
    pub socks5: Option<bool>,
    pub connect_25: Option<bool>,
    pub connect_80: Option<bool>,
}

pub fn parse_toml_config(content: &str) -> Result<TomlConfig, Box<dyn std::error::Error>> {
    let config: TomlConfig = toml::from_str(content)?;
    Ok(config)
}

pub fn update_dynamic_config(
    dynamic_config: &mut DynamicConfig,
    toml_config: &TomlConfig,
) -> Result<Vec<ConfigChange>, Box<dyn std::error::Error>> {
    let mut changes = Vec::new();

    // Update general section
    if let Some(ref general) = toml_config.general {
        let old_value = serde_json::to_value(&dynamic_config.general)?;
        merge_general_config(&mut dynamic_config.general, general);
        let new_value = serde_json::to_value(&dynamic_config.general)?;
        changes.push(ConfigChange {
            section: ConfigSection::General,
            old_value,
            new_value,
        });
    }

    // Update DNSBL section
    if let Some(ref dnsbl) = toml_config.dnsbl {
        let old_value = serde_json::to_value(&dynamic_config.dnsbl)?;
        merge_dnsbl_config(&mut dynamic_config.dnsbl, dnsbl);
        let new_value = serde_json::to_value(&dynamic_config.dnsbl)?;
        changes.push(ConfigChange {
            section: ConfigSection::Dnsbl,
            old_value,
            new_value,
        });
    }

    // Update server section
    if let Some(ref server) = toml_config.server {
        let old_value = serde_json::to_value(&dynamic_config.server)?;
        merge_server_config(&mut dynamic_config.server, server);
        let new_value = serde_json::to_value(&dynamic_config.server)?;
        changes.push(ConfigChange {
            section: ConfigSection::Server,
            old_value,
            new_value,
        });
    }

    // Update protocols section
    if let Some(ref protocols) = toml_config.protocols {
        let old_value = serde_json::to_value(&dynamic_config.protocols)?;
        merge_protocols_config(&mut dynamic_config.protocols, protocols);
        let new_value = serde_json::to_value(&dynamic_config.protocols)?;
        changes.push(ConfigChange {
            section: ConfigSection::Protocols,
            old_value,
            new_value,
        });
    }

    Ok(changes)
}

fn merge_general_config(existing: &mut GeneralConfig, update: &TomlGeneralConfig) {
    if let Some(max_connections) = update.max_connections {
        existing.max_connections = max_connections;
    }
    if let Some(default_timeout) = update.default_timeout {
        existing.default_timeout = default_timeout;
    }
    if let Some(rate_limit_delay_ms) = update.rate_limit_delay_ms {
        existing.rate_limit_delay_ms = rate_limit_delay_ms;
    }
    if let Some(ref log_level) = update.log_level {
        existing.log_level = log_level.clone();
    }
}

fn merge_dnsbl_config(existing: &mut DnsblConfig, update: &TomlDnsblConfig) {
    if let Some(enabled) = update.enabled {
        existing.enabled = enabled;
    }
    if let Some(timeout_secs) = update.timeout_secs {
        existing.timeout_secs = timeout_secs;
    }
    if let Some(max_concurrent) = update.max_concurrent {
        existing.max_concurrent = max_concurrent;
    }
    if let Some(cache_ttl_secs) = update.cache_ttl_secs {
        existing.cache_ttl_secs = cache_ttl_secs;
    }
    if let Some(malicious_threshold) = update.malicious_threshold {
        existing.malicious_threshold = malicious_threshold;
    }
}

fn merge_server_config(existing: &mut ServerConfig, update: &TomlServerConfig) {
    if let Some(max_clients) = update.max_clients {
        existing.max_clients = max_clients;
    }
    if let Some(port) = update.port {
        existing.port = port;
    }
    if let Some(timeout) = update.timeout {
        existing.timeout = timeout;
    }
}

fn merge_protocols_config(existing: &mut ProtocolConfig, update: &TomlProtocolConfig) {
    if let Some(http) = update.http {
        existing.http = http;
    }
    if let Some(https) = update.https {
        existing.https = https;
    }
    if let Some(socks4) = update.socks4 {
        existing.socks4 = socks4;
    }
    if let Some(socks5) = update.socks5 {
        existing.socks5 = socks5;
    }
    if let Some(connect_25) = update.connect_25 {
        existing.connect_25 = connect_25;
    }
    if let Some(connect_80) = update.connect_80 {
        existing.connect_80 = connect_80;
    }
}

#[derive(Debug)]
pub struct ConfigChange {
    pub section: ConfigSection,
    pub old_value: serde_json::Value,
    pub new_value: serde_json::Value,
}