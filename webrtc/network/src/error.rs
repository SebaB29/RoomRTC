//! Network error types
//!
//! This module defines the error types used throughout the network module.
//! All errors are wrapped in `NetworkError` enum for consistent error handling.

pub use media::MediaError;
use std::fmt;
pub type Result<T> = std::result::Result<T, NetworkError>;

/// Network-related errors
#[derive(Debug)]
pub enum NetworkError {
    Config(String),
    Logging(String),
    Network(String),
    Rtp(String),
    Media(MediaError),
    CryptoError(String),
    InvalidPacket(String),
    SecurityError(String),
    TransportError(String),
    ThreadError(String),
    ChannelError(String),
    WouldBlock,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkError::Config(msg) => write!(f, "Config error: {}", msg),
            NetworkError::Logging(msg) => write!(f, "Logging error: {}", msg),
            NetworkError::Network(msg) => write!(f, "Network error: {}", msg),
            NetworkError::Rtp(msg) => write!(f, "RTP error: {}", msg),
            NetworkError::Media(msg) => write!(f, "{}", msg),
            NetworkError::CryptoError(msg) => write!(f, "Crypto error: {}", msg),
            NetworkError::InvalidPacket(msg) => write!(f, "Invalid packet: {}", msg),
            NetworkError::SecurityError(msg) => write!(f, "Security error: {}", msg),
            NetworkError::TransportError(msg) => write!(f, "Transport error: {}", msg),
            NetworkError::ThreadError(msg) => write!(f, "Thread error: {}", msg),
            NetworkError::ChannelError(msg) => write!(f, "Channel error: {}", msg),
            NetworkError::WouldBlock => write!(f, "Operation would block"),
        }
    }
}

impl std::error::Error for NetworkError {}
impl From<MediaError> for NetworkError {
    fn from(err: MediaError) -> Self {
        NetworkError::Media(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_config() {
        let err = NetworkError::Config("Invalid MTU size".to_string());
        assert_eq!(err.to_string(), "Config error: Invalid MTU size");
    }

    #[test]
    fn test_error_display_logging() {
        let err = NetworkError::Logging("Failed to open log file".to_string());
        assert_eq!(err.to_string(), "Logging error: Failed to open log file");
    }

    #[test]
    fn test_error_display_network() {
        let err = NetworkError::Network("Connection refused".to_string());
        assert_eq!(err.to_string(), "Network error: Connection refused");
    }

    #[test]
    fn test_error_display_rtp() {
        let err = NetworkError::Rtp("Invalid packet format".to_string());
        assert_eq!(err.to_string(), "RTP error: Invalid packet format");
    }

    #[test]
    fn test_error_from_media_error() {
        let media_err = media::MediaError::Camera("Camera not found".to_string());
        let network_err: NetworkError = media_err.into();

        match network_err {
            NetworkError::Media(_) => {}
            _ => panic!("Expected NetworkError::Media"),
        }
    }

    #[test]
    fn test_error_is_error_trait() {
        let err = NetworkError::Network("Test".to_string());
        let _: &dyn std::error::Error = &err;
    }
}
