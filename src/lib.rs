//! Proxy.rs - High-performance proxy pool manager (Simplified Version)
//!
//! This library provides simplified functionality for discovering, testing, and serving quality proxies.
//!
//! # Features
//!
//! - **Basic Proxy Discovery**: Simplified proxy source management
//! - **Quality Testing**: Multi-protocol validation (HTTP, HTTPS, SOCKS4, SOCKS5)
//! - **Simple Pooling**: Basic connection management
//! - **Security Filtering**: DNSBL integration with security lists
//! - **High Performance**: Async/await based architecture
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use proxy_rs_simple::{Proxy, ProxyChecker};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let proxy = Proxy::new("127.0.0.1", 8080);
//!     let checker = ProxyChecker::new();
//!
//!     if checker.check_proxy(&proxy).await {
//!         println!("Proxy is working: {}", proxy);
//!     }
//!
//!     Ok(())
//! }
//! ```

// Core modules
pub mod argument;
pub mod checker;
pub mod proxy;
pub mod dnsbl;
pub mod judge;
pub mod judge_optimized;
pub mod resolver;
pub mod negotiators;
pub mod providers;
pub mod utils;
pub mod performance;

// Configuration modules
pub mod config;

// API REST modules
pub mod api;

// Server modules
pub mod server {
    pub mod proxy_pool;
    pub mod connection_pool;
    pub mod multi_cache;
    pub mod async_optimizer;
    pub mod mod_simple;
}

// Re-export commonly used types
pub use proxy::Proxy;
pub use checker::Checker;
pub use server::mod_simple::{SimpleServer, start_simple_server};

// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");