use crate::config::{DynamicConfig, ConfigSection};
use parking_lot::RwLock;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{self, Sender, Receiver};

#[derive(Debug, Clone)]
pub enum WatcherEvent {
    ConfigChanged {
        section: ConfigSection,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
    },
    Error(String),
}

pub struct ConfigWatcher {
    config_path: std::path::PathBuf,
}

impl ConfigWatcher {
    pub fn new<P: AsRef<Path>>(
        config_path: P,
        _event_sender: Sender<WatcherEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config_path = config_path.as_ref().to_path_buf();

        // Simple file watcher implementation without notify complexity
        log::info!("Config watcher started for: {:?}", config_path);

        Ok(ConfigWatcher {
            config_path,
        })
    }

    pub fn start_watching(self) -> Receiver<WatcherEvent> {
        let (tx, rx) = mpsc::channel(100);
        let config_path = self.config_path.clone();

        // Start a simple file watcher using polling
        tokio::spawn(async move {
            let mut last_modified = match std::fs::metadata(&config_path) {
                Ok(meta) => meta.modified().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH),
                Err(_) => std::time::SystemTime::UNIX_EPOCH,
            };

            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;

                match std::fs::metadata(&config_path) {
                    Ok(meta) => {
                        let current_modified = meta.modified().unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH);

                        if current_modified != last_modified {
                            last_modified = current_modified;

                            // File was modified
                            log::debug!("Config file modification detected");

                            // Small delay to ensure file write is complete
                            tokio::time::sleep(Duration::from_millis(100)).await;

                            let change_result = Self::read_config_changes(&config_path);
                            match change_result {
                                Ok(change) => {
                                    log::info!("Config section '{}' changed", change.section);
                                    if let Err(e) = tx.send(WatcherEvent::ConfigChanged {
                                        section: change.section,
                                        old_value: change.old_value,
                                        new_value: change.new_value,
                                    }).await {
                                        log::error!("Failed to send watcher event: {}", e);
                                        break;
                                    }
                                }
                                Err(e) => {
                                    let error_msg = format!("Config read error: {}", e);
                                    log::error!("{}", error_msg);
                                    if let Err(send_err) = tx.send(WatcherEvent::Error(error_msg)).await {
                                        log::error!("Failed to send error event: {}", send_err);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        log::warn!("Failed to read config file metadata: {}", e);
                    }
                }
            }
        });

        rx
    }

    fn read_config_changes(config_path: &std::path::Path) -> Result<ConfigChange, Box<dyn std::error::Error + Send + Sync>> {
        let config_content = std::fs::read_to_string(config_path)?;

        // For this implementation, we'll just return a generic change notification
        log::info!("Configuration file content updated");

        let config: DynamicConfig = toml::from_str(&config_content)?;

        // Return a simple change notification
        Ok(ConfigChange {
            section: ConfigSection::General,
            old_value: serde_json::json!({}), // Empty old value
            new_value: serde_json::to_value(&config.general)?,
        })
    }
}

#[derive(Debug)]
struct ConfigChange {
    section: ConfigSection,
    old_value: serde_json::Value,
    new_value: serde_json::Value,
}

pub async fn start_config_watcher<P: AsRef<Path>>(
    config_path: P,
    shared_config: Arc<RwLock<DynamicConfig>>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let (tx, _rx) = mpsc::channel(100);

    let watcher = ConfigWatcher::new(config_path, tx)?;
    let mut event_rx = watcher.start_watching();

    // Process config change events
    while let Some(event) = event_rx.recv().await {
        match event {
            WatcherEvent::ConfigChanged { section, new_value, .. } => {
                log::info!("Updating config section: {}", section);

                // Update the shared configuration
                {
                    let mut config = shared_config.write();
                    match config.update_section(section.clone(), new_value) {
                        Ok(()) => {
                            log::info!("Successfully updated {} configuration", section);
                        }
                        Err(e) => {
                            log::error!("Failed to update config: {}", e);
                        }
                    }
                } // Write guard is dropped here

                // Apply the changes if needed - clone needed data before async call
                let config_clone = {
                    let config = shared_config.read();
                    config.clone()
                };
                if let Err(e) = apply_config_changes(&config_clone, &section).await {
                    log::error!("Failed to apply config changes: {}", e);
                }
            }
            WatcherEvent::Error(error) => {
                log::error!("Config watcher error: {}", error);
            }
        }
    }

    Ok(())
}

async fn apply_config_changes(
    config: &DynamicConfig,
    section: &ConfigSection,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match section {
        ConfigSection::General => {
            log::info!("Applying general config changes:");
            log::info!("  max_connections: {}", config.general.max_connections);
            log::info!("  default_timeout: {}s", config.general.default_timeout);
            log::info!("  rate_limit_delay_ms: {}ms", config.general.rate_limit_delay_ms);

            // Apply to runtime components
            // This would interface with the actual proxy components
        }
        ConfigSection::Dnsbl => {
            log::info!("Applying DNSBL config changes:");
            log::info!("  enabled: {}", config.dnsbl.enabled);
            log::info!("  timeout_secs: {}", config.dnsbl.timeout_secs);
            log::info!("  malicious_threshold: {}", config.dnsbl.malicious_threshold);

            // Update DNSBL checker configuration
        }
        ConfigSection::Server => {
            log::info!("Applying server config changes:");
            log::info!("  max_clients: {}", config.server.max_clients);
            log::info!("  port: {}", config.server.port);
            log::info!("  timeout: {}s", config.server.timeout);

            // Note: Port changes would require server restart
            if config.server.port != 8080 {
                log::warn!("Port change detected - server restart required");
            }
        }
        ConfigSection::Protocols => {
            log::info!("Applying protocols config changes:");
            log::info!("  http: {}", config.protocols.http);
            log::info!("  https: {}", config.protocols.https);
            log::info!("  socks4: {}", config.protocols.socks4);
            log::info!("  socks5: {}", config.protocols.socks5);
        }
    }

    Ok(())
}