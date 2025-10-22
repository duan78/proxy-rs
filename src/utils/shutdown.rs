//! Graceful shutdown utilities for proxy.rs

use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};
use std::time::Duration;
use log::{info, warn, error};

/// Shutdown signal manager
#[derive(Debug, Clone)]
pub struct ShutdownManager {
    /// Sender for shutdown signals
    shutdown_tx: broadcast::Sender<ShutdownReason>,
    /// Tracking active tasks
    active_tasks: Arc<RwLock<std::collections::HashMap<String, TaskInfo>>>,
    /// Configuration
    config: ShutdownConfig,
}

#[derive(Debug, Clone)]
pub struct ShutdownConfig {
    /// Graceful shutdown timeout
    pub timeout: Duration,
    /// Force shutdown timeout
    pub force_timeout: Duration,
    /// Number of shutdown retries
    pub max_retries: usize,
}

impl Default for ShutdownConfig {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            force_timeout: Duration::from_secs(10),
            max_retries: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ShutdownReason {
    UserInterrupt,
    CriticalError(String),
    ResourceExhausted,
    Maintenance,
    Timeout,
}

#[derive(Debug)]
struct TaskInfo {
    name: String,
    created_at: std::time::Instant,
}

impl ShutdownManager {
    pub fn new(config: ShutdownConfig) -> Self {
        let (shutdown_tx, _) = broadcast::channel(1000);

        Self {
            shutdown_tx,
            active_tasks: Arc::new(RwLock::new(std::collections::HashMap::new())),
            config,
        }
    }

    /// Register a task for graceful shutdown
    pub async fn register_task(&self, task_name: String) -> mpsc::Receiver<ShutdownReason> {
        let (tx, rx) = mpsc::channel(100);

        // Store task info
        let mut tasks = self.active_tasks.write().await;
        tasks.insert(task_name.clone(), TaskInfo {
            name: task_name.clone(),
            created_at: std::time::Instant::now(),
        });

        // Subscribe to shutdown signals
        let mut shutdown_rx = self.shutdown_tx.subscribe();
        let tasks_clone = self.active_tasks.clone();

        tokio::spawn(async move {
            if let Ok(reason) = shutdown_rx.recv().await {
                // Send reason through channel
                let _ = tx.send(reason).await;

                // Remove task from active tasks
                let mut tasks = tasks_clone.write().await;
                tasks.remove(&task_name);
            }
        });

        rx
    }

    /// Trigger graceful shutdown
    pub async fn shutdown(&self, reason: ShutdownReason) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!("Initiating graceful shutdown: {:?}", reason);

        // Send shutdown signal to all tasks
        if let Err(e) = self.shutdown_tx.send(reason.clone()) {
            warn!("Failed to send shutdown signal to some tasks: {}", e);
        }

        // Wait for tasks to shutdown gracefully
        let mut attempts = 0;
        while attempts < self.config.max_retries {
            let active_count = self.active_tasks.read().await.len();

            if active_count == 0 {
                info!("All tasks shutdown gracefully");
                return Ok(());
            }

            info!("Waiting for {} tasks to shutdown (attempt {}/{})",
                  active_count, attempts + 1, self.config.max_retries);

            tokio::time::sleep(Duration::from_secs(1)).await;
            attempts += 1;

            if attempts >= self.config.max_retries {
                warn!("Graceful shutdown timeout, forcing termination");
                return self.force_shutdown().await;
            }
        }

        Ok(())
    }

    /// Force shutdown remaining tasks
    async fn force_shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let tasks = self.active_tasks.read().await;
        let remaining_tasks: Vec<_> = tasks.keys().cloned().collect();
        drop(tasks);

        if !remaining_tasks.is_empty() {
            warn!("Force shutting down {} remaining tasks: {:?}",
                  remaining_tasks.len(), remaining_tasks);
        }

        // In a real implementation, you might use more aggressive methods here
        // For now, we'll just log and continue
        error!("Force shutdown completed - some tasks may not have cleaned up properly");

        Ok(())
    }

    /// Get active task statistics
    pub async fn get_task_stats(&self) -> TaskStats {
        let tasks = self.active_tasks.read().await;
        let now = std::time::Instant::now();

        let task_count = tasks.len();
        let oldest_task = tasks.values()
            .map(|task| now.duration_since(task.created_at))
            .max();

        TaskStats {
            active_tasks: task_count,
            oldest_task_uptime: oldest_task,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TaskStats {
    pub active_tasks: usize,
    pub oldest_task_uptime: Option<Duration>,
}

/// Global shutdown manager
use once_cell::sync::Lazy;
use std::sync::atomic::{AtomicBool, Ordering};

pub static SHUTDOWN_MANAGER: Lazy<ShutdownManager> =
    Lazy::new(|| ShutdownManager::new(ShutdownConfig::default()));

pub static SHUTDOWN_TRIGGERED: AtomicBool = AtomicBool::new(false);

/// Initialize the global shutdown manager
pub fn init_shutdown_manager() {
    Lazy::force(&SHUTDOWN_MANAGER);
}

/// Setup signal handlers for graceful shutdown
pub async fn setup_signal_handlers() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use tokio::signal;

    // Handle Ctrl+C
    let ctrl_c_manager = SHUTDOWN_MANAGER.clone();
    let ctrl_c_task = tokio::spawn(async move {
        if let Err(e) = signal::ctrl_c().await {
            error!("Failed to listen for Ctrl+C: {}", e);
            return;
        }

        info!("Ctrl+C received, initiating graceful shutdown");
        SHUTDOWN_TRIGGERED.store(true, Ordering::SeqCst);

        if let Err(e) = ctrl_c_manager.shutdown(ShutdownReason::UserInterrupt).await {
            error!("Graceful shutdown failed: {}", e);
        }
    });

    // Handle SIGTERM (Unix systems)
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let sigterm_manager = SHUTDOWN_MANAGER.clone();
        let sigterm_task = tokio::spawn(async move {
            let mut sigterm = match signal(SignalKind::terminate()) {
                Ok(sig) => sig,
                Err(e) => {
                    error!("Failed to setup SIGTERM handler: {}", e);
                    return;
                }
            };

            match sigterm.recv().await {
                Some(_) => {
                    info!("SIGTERM received, initiating graceful shutdown");
                    SHUTDOWN_TRIGGERED.store(true, Ordering::SeqCst);

                    if let Err(e) = sigterm_manager.shutdown(ShutdownReason::UserInterrupt).await {
                        error!("Graceful shutdown failed: {}", e);
                    }
                }
                None => {
                    info!("SIGTERM stream ended");
                }
            }
        });

        // Wait for either signal
        tokio::select! {
            _ = ctrl_c_task => {},
            _ = sigterm_task => {},
        }
    }

    #[cfg(not(unix))]
    {
        ctrl_c_task.await?;
    }

    Ok(())
}

/// Check if shutdown has been triggered
pub fn is_shutdown_triggered() -> bool {
    SHUTDOWN_TRIGGERED.load(Ordering::SeqCst)
}

/// Register current task for graceful shutdown
pub async fn register_for_shutdown(task_name: String) -> mpsc::Receiver<ShutdownReason> {
    SHUTDOWN_MANAGER.register_task(task_name).await
}

/// Gracefully shutdown the application
pub async fn graceful_shutdown(reason: ShutdownReason) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    SHUTDOWN_MANAGER.shutdown(reason).await
}

/// Macro for task registration with graceful shutdown
#[macro_export]
macro_rules! task_with_shutdown {
    ($name:expr, $body:expr) => {{
        let task_name = $name.to_string();
        let mut shutdown_rx = $crate::utils::shutdown::register_for_shutdown(task_name.clone()).await;

        tokio::spawn(async move {
            tokio::select! {
                _ = $body => {
                    log::info!("Task {} completed normally", task_name);
                }
                reason = shutdown_rx.recv() => {
                    if let Some(reason) = reason {
                        log::info!("Task {} received shutdown signal: {:?}", task_name, reason);
                    }
                }
            }
        })
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_shutdown_manager() {
        let manager = ShutdownManager::new(ShutdownConfig::default());

        // Register a task
        let mut shutdown_rx = manager.register_task("test_task".to_string()).await;

        // Trigger shutdown
        let shutdown_reason = ShutdownReason::Maintenance;
        manager.shutdown(shutdown_reason.clone()).await.unwrap();

        // Task should receive shutdown signal
        let received_reason = shutdown_rx.recv().await.unwrap();
        assert!(matches!(received_reason, ShutdownReason::Maintenance));
    }

    #[tokio::test]
    async fn test_task_stats() {
        let manager = ShutdownManager::new(ShutdownConfig::default());

        let stats = manager.get_task_stats().await;
        assert_eq!(stats.active_tasks, 0);

        // Register a task
        manager.register_task("test_task".to_string()).await;

        let stats = manager.get_task_stats().await;
        assert_eq!(stats.active_tasks, 1);
        assert!(stats.oldest_task_uptime.is_some());
    }
}