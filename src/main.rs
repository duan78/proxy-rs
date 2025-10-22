
use argument::GrabArgs;
use checker::Checker;
use clap::Parser;
use dnsbl::DnsblConfig;
use lazy_static::lazy_static;
use parking_lot::Mutex;
use proxy::Proxy;
use regex::Regex;
use server::{proxy_pool::{LIVE_PROXIES, ProxyPool}, Server, POOL};
use simple_logger::SimpleLogger;
use std::{path::PathBuf, pin::Pin, sync::Arc, time::Duration};
use tokio::{
    fs::File,
    io::{stdout, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    runtime,
    sync::{
        mpsc::{self, Sender},
    },
    task, time,
};

// New imports for improved error handling and resource management
use utils::{
    error::{ProxyError, ProxyResult},
    resource_manager::{init_resource_managers, create_resource_semaphore},
    shutdown::{init_shutdown_manager, setup_signal_handlers, register_for_shutdown},
};

mod api;
use config::{DynamicConfig, SharedConfig};
use config::hot_reload::start_config_watcher;
use api::{ApiConfig, server::{start_default_api_server, start_api_server_with_config}};

use crate::{
    argument::{Cli, Commands},
    providers::PROXIES,
    utils::update::check_version,
};

mod argument;
mod checker;
mod config;
mod dnsbl;
mod judge;
mod judge_optimized;
mod negotiators;
mod performance;
mod providers;
mod proxy;
mod resolver;
mod server;
mod utils;

lazy_static! {
    static ref STOP_FIND_LOOP: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
}

struct ProxiesIter;
impl Iterator for ProxiesIter {
    type Item = Proxy; //(String, u16, Vec<String>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Ok(proxy) = PROXIES.pop() {
            return Some(proxy);
        }
        None
    }
}

async fn handle_grab_command(args: GrabArgs, tx: Sender<Option<Proxy>>) -> ProxyResult<()> {
    // Register this task for graceful shutdown
    let mut shutdown_rx = register_for_shutdown("grab_command".to_string()).await;

    let expected_countries = args.countries;

    loop {
        // Check for shutdown signal
        if shutdown_rx.try_recv().is_ok() {
            log::info!("Grab command received shutdown signal");
            break;
        }

        let proxies = ProxiesIter {};
        for proxy in proxies {
            // Check for shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                log::info!("Grab command received shutdown signal during processing");
                break;
            }

            if !expected_countries.is_empty() && !expected_countries.contains(&proxy.geo.iso_code) {
                continue;
            }

            if tx.send(Some(proxy)).await.is_err() {
                log::warn!("Failed to send proxy, channel closed");
                return Err(ProxyError::Http("Channel closed".to_string()));
            }
        }

        // Small delay to prevent tight loop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

async fn handle_find_command(checker: Checker, _max_conn: usize, tx: Sender<Option<Proxy>>) -> ProxyResult<()> {
    // Register this task for graceful shutdown
    let mut shutdown_rx = register_for_shutdown("find_command".to_string()).await;

    // Use resource semaphore instead of creating new one
    let sem = utils::resource_manager::get_resource_semaphore("connections").await?;

    while !*STOP_FIND_LOOP.lock() {
        // Check for shutdown signal
        if shutdown_rx.try_recv().is_ok() {
            log::info!("Find command received shutdown signal");
            break;
        }

        let proxies = ProxiesIter {};
        for mut proxy in proxies {
            // Check for shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                log::info!("Find command received shutdown signal during processing");
                break;
            }

            if *STOP_FIND_LOOP.lock() {
                if let Err(e) = tx.send(None).await {
                    log::error!("Failed to send stop signal: {}", e);
                    return Err(ProxyError::Http("Failed to send stop signal".to_string()));
                }
            }

            match sem.clone().acquire_owned().await {
                Ok(permit) => {
                    let checker = checker.clone();
                    let tx = tx.clone();

                    let mut checker_clone = checker.clone();
                    task::spawn(async move {
                        let _permit = permit;
                        if checker_clone.check_proxy(&mut proxy).await {
                            if let Err(e) = tx.send(Some(proxy)).await {
                                log::error!("Failed to send proxy result: {}", e);
                            }
                        }
                    });
                }
                Err(e) => {
                    log::warn!("Failed to acquire connection permit: {}", e);
                    // Continue instead of failing
                    continue;
                }
            }
        }

        // Small delay to prevent tight loop
        tokio::time::sleep(Duration::from_millis(100)).await;
    }

    Ok(())
}

async fn handle_file_input(files: Vec<PathBuf>) -> ProxyResult<()> {
    // Register this task for graceful shutdown
    let mut shutdown_rx = register_for_shutdown("file_input".to_string()).await;

    let ip_port = Regex::new(r#"(?P<ip>(?:\d+\.?){4}):(?P<port>\d+)"#)
        .map_err(|e| ProxyError::Config(format!("Failed to compile regex: {}", e)))?;

    for file in files {
        // Check for shutdown signal
        if shutdown_rx.try_recv().is_ok() {
            log::info!("File input received shutdown signal");
            break;
        }

        match File::open(&file).await {
            Ok(file_handle) => {
                let buffer = BufReader::new(file_handle);
                let mut lines = buffer.lines();

                while let Ok(Some(line)) = lines.next_line().await {
                    // Check for shutdown signal periodically
                    if shutdown_rx.try_recv().is_ok() {
                        log::info!("File input received shutdown signal during processing");
                        break;
                    }

                    if let Some(cap) = ip_port.captures(&line) {
                        if let (Some(ip_match), Some(port_match)) = (cap.get(1), cap.get(2)) {
                            let ip = ip_match.as_str();
                            let port = port_match.as_str();

                            match port.parse::<u16>() {
                                Ok(port_num) => {
                                    if let Some(proxy) = Proxy::create(ip, port_num, vec![]).await {
                                        match PROXIES.push(proxy) {
                                            Ok(_) => {
                                                log::debug!("Successfully added proxy from file");
                                            }
                                            Err(e) => {
                                                log::error!("Failed to push proxy to queue: {}", e);
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::warn!("Invalid port number in file: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Failed to open file {:?}: {}", file, e);
                // Continue with other files instead of failing
            }
        }
    }

    Ok(())
}

fn main() -> ProxyResult<()> {
    let cli = Cli::parse();

    // Initialize resource and shutdown managers
    init_resource_managers();
    init_shutdown_manager();

    // Initialize dynamic configuration
    let shared_config = Arc::new(parking_lot::RwLock::new(DynamicConfig::new()));

    // Load initial configuration from file if it exists
    let config_path = "proxy-rs.toml";
    if std::path::Path::new(config_path).exists() {
        match load_initial_config(config_path) {
            Ok(config) => {
                *shared_config.write() = config;
                log::info!("Loaded initial configuration from {}", config_path);
            }
            Err(e) => {
                log::warn!("Failed to load initial config: {}, using defaults", e);
            }
        }
    }

    let log_level = match cli.log_level.as_str() {
        "debug" => log::LevelFilter::Debug,
        "info" => log::LevelFilter::Info,
        "error" => log::LevelFilter::Error,
        _ => log::LevelFilter::Warn,
    };

    SimpleLogger::new()
        .with_level(log::LevelFilter::Off)
        .with_module_level("proxy_rs", log_level)
        .without_timestamps()
        .init()
        .map_err(|e| ProxyError::Config(format!("Failed to initialize logger: {}", e)))?;

    let runtime = runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .map_err(|e| ProxyError::Config(format!("Failed to create runtime: {}", e)))?;

    let result = runtime.block_on(async move {
        // Setup signal handlers for graceful shutdown
        let shutdown_handle = tokio::spawn(setup_signal_handlers());

        // Register main task for graceful shutdown
        let mut shutdown_rx = register_for_shutdown("main".to_string()).await;

        let max_conn = cli.max_conn;
        let timeout = cli.timeout as i32;

        // Create resource semaphore for connection limiting
        create_resource_semaphore("connections".to_string(), max_conn).await?;

              // Start config watcher for hot-reload functionality
        let _config_watcher_handle = tokio::spawn(start_config_watcher_with_retry(
            "proxy-rs.toml",
            shared_config.clone(),
        ));

        // Start REST API server
        let api_config = ApiConfig {
            host: "127.0.0.1".to_string(),
            port: 3000,
            enable_auth: false, // Can be enabled in production
            jwt_secret: "proxy-rs-api-secret".to_string(),
            rate_limit: 1000,
            cors_origins: vec!["*".to_string()],
            request_timeout_ms: 30000,
        };

        let api_shared_config = shared_config.clone();
        let _api_server_handle = tokio::spawn(async move {
            if let Err(e) = start_api_server_with_config(api_config, api_shared_config).await {
                log::error!("API server failed to start: {}", e);
            }
        });

        log::info!("🚀 REST API Server started on http://127.0.0.1:3000");
        log::info!("📚 API Documentation: http://127.0.0.1:3000/docs");
        log::info!("🔗 API Health: http://127.0.0.1:3000/api/v1/health");

        let mut files = vec![];
        let (tx, mut rx) = mpsc::channel(50);
        let mut tasks = vec![];

        let mut outfile = None;
        let mut limit = 0;
        let mut format = "default".to_string();

        let mut is_server = false;
        let mut host = "127.0.0.1".to_string();
        let mut port = 8080;

        if !cli.skip_version_check {
            task::spawn(check_version());
        }

        match cli.sub {
            Commands::Grab(grab_args) => {
                outfile = grab_args.outfile.clone();
                limit = grab_args.limit;
                format = grab_args.format.clone();

                let tx = tx.clone();
                task::spawn(handle_grab_command(grab_args, tx));
            }
            Commands::Find(find_args) => {
                outfile = find_args.outfile.clone();
                limit = find_args.limit;
                format = find_args.format.clone();

                let mut checker = Checker::new().await;
                checker.max_tries = find_args.max_tries as i32;
                checker.timeout = timeout;
                checker.support_cookie = find_args.support_cookies;
                checker.support_referer = find_args.support_referer;
                checker.expected_types = find_args.types.clone();
                checker.expected_levels = find_args.levels;
                checker.expected_countries = find_args.countries;

                // Initialize DNSBL if enabled
                if find_args.dnsbl_check {
                    let dnsbl_config = DnsblConfig {
                        enabled: true,
                        timeout_secs: find_args.dnsbl_timeout_secs,
                        max_concurrent: find_args.dnsbl_max_concurrent,
                        cache_ttl_secs: find_args.dnsbl_cache_ttl_secs,
                        malicious_threshold: find_args.dnsbl_malicious_threshold,
                        specific_lists: find_args.dnsbl_specific_lists,
                        excluded_lists: find_args.dnsbl_excluded_lists,
                    };

                    if let Err(e) = checker.enable_dnsbl(dnsbl_config).await {
                        log::error!("Failed to initialize DNSBL checker: {}", e);
                    }
                }

                let ext_ip = checker.ext_ip.clone();

                let expected_types = find_args.types.clone();
                let verify_ssl = false;
                task::spawn(async move {
                    checker::check_judges(verify_ssl, ext_ip, expected_types).await;
                });

                files.extend(find_args.files.clone());

                let tx = tx.clone();
                task::spawn(handle_find_command(checker, max_conn, tx));
            }
            Commands::Serve(serve_args) => {
                is_server = true;

                host = serve_args.host;
                port = serve_args.port;

                let mut checker = Checker::new().await;
                checker.max_tries = serve_args.max_tries as i32;
                checker.support_cookie = true;
                checker.support_referer = true;

                checker.expected_types = serve_args.types.clone();
                checker.expected_levels = serve_args.levels;
                checker.expected_countries = serve_args.countries;

                // Initialize DNSBL if enabled
                if serve_args.dnsbl_check {
                    let dnsbl_config = DnsblConfig {
                        enabled: true,
                        timeout_secs: serve_args.dnsbl_timeout_secs,
                        max_concurrent: serve_args.dnsbl_max_concurrent,
                        cache_ttl_secs: serve_args.dnsbl_cache_ttl_secs,
                        malicious_threshold: serve_args.dnsbl_malicious_threshold,
                        specific_lists: serve_args.dnsbl_specific_lists,
                        excluded_lists: serve_args.dnsbl_excluded_lists,
                    };

                    if let Err(e) = checker.enable_dnsbl(dnsbl_config).await {
                        log::error!("Failed to initialize DNSBL checker: {}", e);
                    }
                }

                let ext_ip = checker.ext_ip.clone();

                let expected_types = serve_args.types.clone();
                let verify_ssl = false;
                task::spawn(async move {
                    checker::check_judges(verify_ssl, ext_ip, expected_types).await;
                });
                files.extend(serve_args.files.clone());

                // Initialize ProxyPool with custom max response time
                let max_avg_resp_time_sec = serve_args.max_avg_resp_time as f64 / 1000.0;
                *POOL.lock() = ProxyPool::with_max_resp_time(max_avg_resp_time_sec);

                let tx = tx.clone();
                task::spawn(handle_find_command(checker, max_conn, tx));
            }
        }

        if !files.is_empty() {
            task::spawn(async move {
                handle_file_input(files).await;
                let mut stop_file_loop = STOP_FIND_LOOP.lock();
                *stop_file_loop = true
            });
        } else {
            if !is_server {
                log::info!("Start collecting proxies.. ");
            }

            // providers
            tasks.push(tokio::task::spawn(async {
                let dur = Duration::from_secs(60);
                loop {
                    providers::run_all_providers(3).await;
                    log::debug!("Next cycle starts at {:?}", dur);
                    time::sleep(dur).await;
                }
            }));
        }

        if is_server {
            tasks.push(tokio::task::spawn(async move {
                let server = Server::new(host.as_str(), port);
                server.start().await;
            }));

            loop {
                if let Some(Some(proxy)) = rx.recv().await {
                    while LIVE_PROXIES.is_full() {
                        continue;
                    }
                    if let Err(e) = LIVE_PROXIES.push(proxy) {
                        log::error!("Failed to add proxy to live pool: {}", e);
                    }
                }
            }
        } else {
            let mut output: Pin<Box<dyn AsyncWrite>> = if let Some(path) = outfile {
                match File::create(path).await {
                    Ok(file) => Box::pin(file),
                    Err(e) => {
                        log::error!("Failed to create output file: {}", e);
                        return Err(ProxyError::Http(format!("Failed to create output file: {}", e)));
                    }
                }
            } else {
                Box::pin(stdout())
            };

            let mut open_list = false;
            let mut counter = limit;

            while let Some(proxy) = rx.recv().await {
                let stop = proxy.is_none() || (limit != 0 && counter <= 1);
                if let Some(proxy) = proxy {
                    if format == "json" && !open_list {
                        if let Err(e) = output.write_all(b"[").await {
                            log::error!("Failed to write JSON opening bracket: {}", e);
                            break;
                        }
                        open_list = true;
                    }

                    let msg = match format.as_str() {
                        "text" => proxy.as_text(),
                        "json" => proxy.as_json(),
                        _ => format!("{}", proxy),
                    };

                    if let Err(e) = output.write_all(msg.as_bytes()).await {
                        log::error!("Failed to write proxy data: {}", e);
                        break;
                    }

                    if stop {
                        let closing = if format == "json" { &b"]"[..] } else { &b""[..] };
                        if let Err(e) = output.write_all(closing).await {
                            log::error!("Failed to write closing data: {}", e);
                        }
                    } else if format == "json" {
                        if let Err(e) = output.write_all(b",").await {
                            log::error!("Failed to write JSON comma: {}", e);
                            break;
                        }
                    }

                    if let Err(e) = output.write_all(b"\n").await {
                        log::error!("Failed to write newline: {}", e);
                        break;
                    }
                }
                if limit != 0 {
                    counter -= 1;
                }

                if stop {
                    log::info!("Stopping proxy collection gracefully");
                    break;
                }
            }
        }

        Ok(())
    })?;

    Ok(())
}

fn load_initial_config(config_path: &str) -> Result<DynamicConfig, ProxyError> {
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| ProxyError::Config(format!("Failed to read config file: {}", e)))?;

    let config = toml::from_str::<DynamicConfig>(&content)
        .map_err(|e| ProxyError::Config(format!("Failed to parse config: {}", e)))?;

    Ok(config)
}

async fn start_config_watcher_with_retry(
    config_path: &str,
    shared_config: SharedConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    loop {
        match start_config_watcher(config_path, shared_config.clone()).await {
            Ok(()) => {
                log::info!("Config watcher started successfully");
                return Ok(());
            }
            Err(e) => {
                log::warn!("Failed to start config watcher: {}. Retrying in 30 seconds...", e);
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        }
    }
}
