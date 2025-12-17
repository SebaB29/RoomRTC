use json_parser::impl_json;

/// Logging configuration
#[derive(Debug, Clone)]
pub struct LoggingConfig {
    pub log_file_path: String,
    pub log_level: String,
    pub enable_console: bool,
    pub enable_file: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        LoggingConfig {
            log_file_path: "roomrtc-server.log".to_string(),
            log_level: "info".to_string(),
            enable_console: true,
            enable_file: true,
        }
    }
}

impl_json! {
    LoggingConfig {
        log_file_path: String,
        log_level: String,
        enable_console: bool,
        enable_file: bool,
    }
}
