use json_parser::{from_str, impl_json};
use std::error::Error;

use crate::config::{LoggingConfig, ServerConfig};

/// RoomRTC server configuration
#[derive(Debug, Clone, Default)]
pub struct RoomRtcConfig {
    pub server: ServerConfig,
    pub logging: LoggingConfig,
}

impl_json! {
    RoomRtcConfig {
        server: ServerConfig,
        logging: LoggingConfig,
    }
}

impl RoomRtcConfig {
    /// Load configuration from a JSON file
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn Error>> {
        let content = config_loader::load_config_file(path)?;
        from_str(&content).map_err(|e| e.into())
    }
}
