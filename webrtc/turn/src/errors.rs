//! Error types for TURN operations.

use std::fmt;
use std::io;

/// Result type for TURN operations.
pub type Result<T> = std::result::Result<T, TurnError>;

/// Errors that can occur during TURN operations.
#[derive(Debug)]
pub enum TurnError {
    /// IO error during socket operations
    Io(io::Error),
    /// STUN error from underlying STUN layer
    Stun(stun::StunError),
    /// Allocation request failed
    AllocationFailed(String),
    /// Permission creation failed
    PermissionFailed(String),
    /// Channel bind failed
    ChannelBindFailed(String),
    /// Refresh failed
    RefreshFailed(String),
    /// Invalid response from server
    InvalidResponse,
    /// Authentication failed
    AuthenticationFailed,
    /// No allocation exists
    NoAllocation,
    /// Invalid credentials
    InvalidCredentials,
    /// Server returned an error
    ServerError(u16, String), // Error code and reason
    /// Allocation quota reached on server
    AllocationQuotaReached,
    /// Insufficient capacity on server
    InsufficientCapacity,
    /// Unsupported transport protocol
    UnsupportedTransport,
    /// Timeout waiting for response
    Timeout,
    /// Invalid message format
    InvalidMessage(String),
    /// Attribute parsing error
    AttributeError(String),
}

impl fmt::Display for TurnError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TurnError::Io(e) => write!(f, "IO error: {}", e),
            TurnError::Stun(e) => write!(f, "STUN error: {}", e),
            TurnError::AllocationFailed(msg) => write!(f, "Allocation failed: {}", msg),
            TurnError::PermissionFailed(msg) => write!(f, "Permission creation failed: {}", msg),
            TurnError::ChannelBindFailed(msg) => write!(f, "Channel bind failed: {}", msg),
            TurnError::RefreshFailed(msg) => write!(f, "Refresh failed: {}", msg),
            TurnError::InvalidResponse => write!(f, "Invalid response from TURN server"),
            TurnError::AuthenticationFailed => write!(f, "Authentication failed"),
            TurnError::NoAllocation => write!(f, "No allocation exists"),
            TurnError::InvalidCredentials => write!(f, "Invalid credentials"),
            TurnError::ServerError(code, reason) => {
                write!(f, "Server error {}: {}", code, reason)
            }
            TurnError::AllocationQuotaReached => {
                write!(f, "Allocation quota reached on TURN server")
            }
            TurnError::InsufficientCapacity => {
                write!(f, "TURN server has insufficient capacity")
            }
            TurnError::UnsupportedTransport => {
                write!(f, "Unsupported transport protocol")
            }
            TurnError::Timeout => write!(f, "Timeout waiting for response"),
            TurnError::InvalidMessage(msg) => write!(f, "Invalid message: {}", msg),
            TurnError::AttributeError(msg) => write!(f, "Attribute error: {}", msg),
        }
    }
}

impl std::error::Error for TurnError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            TurnError::Io(e) => Some(e),
            TurnError::Stun(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for TurnError {
    fn from(err: io::Error) -> Self {
        TurnError::Io(err)
    }
}

impl From<stun::StunError> for TurnError {
    fn from(err: stun::StunError) -> Self {
        TurnError::Stun(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_error_display() {
        let err = TurnError::NoAllocation;
        assert_eq!(err.to_string(), "No allocation exists");

        let err = TurnError::ServerError(438, "Stale Nonce".to_string());
        assert_eq!(err.to_string(), "Server error 438: Stale Nonce");

        let err = TurnError::AllocationFailed("Invalid transport".to_string());
        assert_eq!(err.to_string(), "Allocation failed: Invalid transport");
    }

    #[test]
    fn test_turn_error_from_io() {
        let io_err = io::Error::new(io::ErrorKind::TimedOut, "timeout");
        let turn_err: TurnError = io_err.into();

        match turn_err {
            TurnError::Io(_) => (),
            _ => panic!("Expected TurnError::Io"),
        }
    }
}
