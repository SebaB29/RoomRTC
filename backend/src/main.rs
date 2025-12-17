pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod tcp;

use std::sync::Arc;

use config::RoomRtcConfig;
use infrastructure::storage::Storage;

fn main() {
    println!("RoomRTC Server - Starting...");

    // Load configuration
    let config = load_config();

    // Initialize logger
    let logger = initialize_logger(&config);

    logger.info("RoomRTC Server starting...");

    // Initialize storage with persistence
    let storage = Arc::new(Storage::with_persistence());

    // Run TCP server
    run_tcp_server(&config, storage, logger);
}

/// Initializes the main logger from configuration
fn initialize_logger(config: &RoomRtcConfig) -> logging::Logger {
    let log_level = config
        .logging
        .log_level
        .parse()
        .unwrap_or(logging::LogLevel::Info);
    let log_path = config.logging.log_file_path.clone().into();
    let enable_console = config.logging.enable_console;

    match logging::Logger::with_component(log_path, log_level, "Main".to_string(), enable_console) {
        Ok(logger) => {
            println!(
                "Logging initialized: {} (level: {})",
                config.logging.log_file_path, config.logging.log_level
            );
            logger
        }
        Err(e) => {
            eprintln!("Failed to create logger: {}", e);
            eprintln!("Cannot continue without logging system.");
            std::process::exit(1);
        }
    }
}

/// Loads configuration from file or returns default values
fn load_config() -> RoomRtcConfig {
    // Determine the configuration file path in this order:
    // 1. CONFIG environment variable
    // 2. First command-line argument
    // 3. Default to "server_config.json"
    if let Ok(json_str) = std::env::var("CONFIG") {
        match json_parser::from_str::<RoomRtcConfig>(&json_str) {
            Ok(cfg) => {
                println!("Configuration loaded from CONFIG env as JSON string");
                return cfg;
            }
            Err(e) => {
                eprintln!("CONFIG env is not valid JSON: {}", e);
            }
        }
    }

    let config_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "server_config.json".to_string());

    match RoomRtcConfig::load_from_file(&config_path) {
        Ok(c) => {
            println!("Configuration loaded from: {}", config_path);
            c
        }
        Err(e) => {
            eprintln!(" Failed to load configuration from {}: {}", config_path, e);
            eprintln!("Using default values...");
            RoomRtcConfig::default()
        }
    }
}

/// Runs the TCP server (blocking)
fn run_tcp_server(config: &RoomRtcConfig, storage: Arc<Storage>, main_logger: logging::Logger) {
    let bind_addr = format!("{}:{}", config.server.bind_address, config.server.port);

    let tcp_logger = main_logger.for_component("TCP").unwrap_or_else(|e| {
        eprintln!("Failed to create TCP logger: {}", e);
        std::process::exit(1);
    });

    let tcp_server = tcp::TcpServer::new(storage.as_ref().clone(), tcp_logger.clone());
    tcp_logger.info(&format!("TCP Server starting on {}", bind_addr));

    // Enable TLS if configured
    let tcp_server = if config.server.enable_tls {
        tcp_logger.info(&format!(
            "TLS enabled - loading certificate from {}",
            config
                .server
                .pkcs12_path
                .as_ref()
                .unwrap_or(&"".to_string())
        ));

        if let Some(pkcs12_path) = &config.server.pkcs12_path {
            let password = config.server.pkcs12_password.as_deref().unwrap_or("");

            match tcp_server.with_tls(pkcs12_path, password) {
                Ok(server) => {
                    tcp_logger.info("TLS enabled successfully");
                    server
                }
                Err(e) => {
                    tcp_logger.error(&format!("Failed to enable TLS: {}", e));
                    tcp_logger.error("Server will NOT start without valid TLS certificate");
                    return;
                }
            }
        } else {
            tcp_logger.error("TLS enabled but pkcs12_path not set in config");
            tcp_logger.error("Server will NOT start - please provide certificate path");
            return;
        }
    } else {
        tcp_logger.warn("TLS is DISABLED - connections will not be encrypted!");
        tcp_logger
            .warn("This is only suitable when using a TLS-terminating proxy like Tailscale Funnel");
        tcp_server
    };

    println!("TCP Server starting on {}", bind_addr);
    tcp_logger.info(&format!("TCP Server starting on {}", bind_addr));

    if let Err(e) = tcp_server.start(&bind_addr) {
        tcp_logger.error(&format!("TCP server error: {}", e));
        std::process::exit(1);
    }
}
