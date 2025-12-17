//! Error types for media operations.
//!
//! This module defines all possible errors that can occur during
//! media capture, encoding, and decoding operations.

use std::fmt;
use std::io;

pub type Result<T> = std::result::Result<T, MediaError>;

/// Error type for media operations
#[derive(Debug)]
pub enum MediaError {
    /// Configuration error
    Config(String),
    /// I/O error
    Io(io::Error),
    /// Camera error
    Camera(String),
    /// Audio error
    Audio(String),
    /// Processing error
    Processing(String),
    /// Logging error
    Logging(String),
    /// OpenCV error
    OpenCv(opencv::Error),
    /// Codec error
    Codec(String),
    /// Network error
    Network(String),
}

impl fmt::Display for MediaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MediaError::Config(msg) => write!(f, "Config error: {}", msg),
            MediaError::Io(err) => write!(f, "I/O error: {}", err),
            MediaError::Camera(msg) => write!(f, "Camera error: {}", msg),
            MediaError::Audio(msg) => write!(f, "Audio error: {}", msg),
            MediaError::Processing(msg) => write!(f, "Processing error: {}", msg),
            MediaError::Logging(msg) => write!(f, "Logging error: {}", msg),
            MediaError::OpenCv(err) => write!(f, "OpenCV error: {}", err),
            MediaError::Codec(msg) => write!(f, "Codec error: {}", msg),
            MediaError::Network(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for MediaError {}

impl From<io::Error> for MediaError {
    fn from(err: io::Error) -> Self {
        MediaError::Io(err)
    }
}

impl From<opencv::Error> for MediaError {
    fn from(err: opencv::Error) -> Self {
        MediaError::OpenCv(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_config() {
        let err = MediaError::Config("Invalid setting".to_string());
        assert_eq!(err.to_string(), "Config error: Invalid setting");
    }

    #[test]
    fn test_error_display_camera() {
        let err = MediaError::Camera("Device not found".to_string());
        assert_eq!(err.to_string(), "Camera error: Device not found");
    }

    #[test]
    fn test_error_display_codec() {
        let err = MediaError::Codec("Encoding failed".to_string());
        assert_eq!(err.to_string(), "Codec error: Encoding failed");
    }

    #[test]
    fn test_error_display_processing() {
        let err = MediaError::Processing("Invalid frame".to_string());
        assert_eq!(err.to_string(), "Processing error: Invalid frame");
    }

    #[test]
    fn test_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let media_err: MediaError = io_err.into();

        match media_err {
            MediaError::Io(_) => (),
            _ => panic!("Expected MediaError::Io"),
        }
    }

    #[test]
    fn test_error_is_error_trait() {
        let err = MediaError::Config("test".to_string());
        let _: &dyn std::error::Error = &err;
    }
}
