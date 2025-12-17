//! Error types for logging operations.

use std::fmt;
use std::io;

/// Result type for logging operations.
pub type Result<T> = std::result::Result<T, LoggingError>;

/// Errors that can occur during logging.
#[derive(Debug)]
pub enum LoggingError {
    /// I/O error from file operations.
    Io(io::Error),
    /// General logging error.
    Logging(String),
}

impl fmt::Display for LoggingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LoggingError::Io(err) => write!(f, "I/O error: {}", err),
            LoggingError::Logging(msg) => write!(f, "Logging error: {}", msg),
        }
    }
}

impl std::error::Error for LoggingError {}

impl From<io::Error> for LoggingError {
    fn from(err: io::Error) -> Self {
        LoggingError::Io(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Error, ErrorKind};

    #[test]
    fn test_logging_error_display() {
        let err = LoggingError::Logging("Write failed".to_string());
        assert_eq!(err.to_string(), "Logging error: Write failed");
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = Error::new(ErrorKind::NotFound, "file not found");
        let logging_err: LoggingError = io_err.into();

        match logging_err {
            LoggingError::Io(_) => {}
            _ => panic!("Expected LoggingError::Io"),
        }
    }
}
