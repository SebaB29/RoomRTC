//! Application Configuration
//!
//! Manages frontend configuration including server address and logging settings.

use logging::LogLevel;
use std::fs;
use std::path::PathBuf;

/// Application configuration structure
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Signaling server address (host:port)
    pub server_address: String,
    /// Path to the log file
    pub log_path: PathBuf,
    /// Logging level
    pub log_level: LogLevel,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_address: "127.0.0.1:8080".to_string(),
            log_path: PathBuf::from("frontend.log"),
            log_level: LogLevel::Info,
        }
    }
}

impl AppConfig {
    /// Loads configuration from a .conf file
    ///
    /// Format:
    /// ```
    /// server_address=127.0.0.1:8080
    /// log_path=frontend.log
    /// log_level=Info
    /// ```
    ///
    /// # Arguments
    /// * `path` - Path to the configuration file
    ///
    /// # Returns
    /// Result with AppConfig or error message
    pub fn load_from_file(path: &str) -> Result<Self, String> {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("Failed to read config file '{}': {}", path, e))?;

        let mut config = Self::default();

        for line in content.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse key=value
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();

                match key {
                    "server_address" => {
                        config.server_address = value.to_string();
                    }
                    "log_path" => {
                        config.log_path = PathBuf::from(value);
                    }
                    "log_level" => {
                        config.log_level = value.parse().unwrap_or(logging::LogLevel::Info);
                    }
                    _ => {
                        // Ignore unknown keys for forward compatibility
                        eprintln!("Warning: Unknown configuration key '{}' ignored", key);
                    }
                }
            }
        }

        Ok(config)
    }

    /// Loads configuration from multiple possible locations
    /// Tries in order: ./config.conf, ./frontend/config.conf, ../config.conf
    /// Falls back to default configuration if no file is found
    pub fn load() -> Self {
        let config_paths = vec!["app.conf", "frontend/app.conf", "../app.conf"];

        for path in config_paths {
            match Self::load_from_file(path) {
                Ok(config) => {
                    println!("Loaded configuration from: {}", path);
                    return config;
                }
                Err(_) => continue,
            }
        }

        println!("No configuration file found, using defaults");
        Self::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.server_address, "127.0.0.1:8080");
        assert_eq!(config.log_path, PathBuf::from("frontend.log"));
    }

    #[test]
    fn test_load_from_content() {
        let content = "\
            # Test config\n\
            server_address=192.168.1.100:9000\n\
            log_path=/tmp/test.log\n\
            log_level=Debug\n\
        ";

        let temp_path = "/tmp/test_config.conf";
        std::fs::write(temp_path, content).unwrap();

        let config = AppConfig::load_from_file(temp_path).unwrap();
        assert_eq!(config.server_address, "192.168.1.100:9000");
        assert_eq!(config.log_path, PathBuf::from("/tmp/test.log"));

        std::fs::remove_file(temp_path).ok();
    }
}
