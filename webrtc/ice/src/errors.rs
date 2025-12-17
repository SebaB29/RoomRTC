//! Error types for ICE operations.
//!
//! This module defines all possible errors that can occur during
//! Interactive Connectivity Establishment (ICE) operations.

/// Errors that can occur during ICE operations.
#[derive(Debug, Clone, PartialEq)]
pub enum IceError {
    /// Invalid candidate format
    InvalidCandidateFormat,
    /// Invalid candidate type
    InvalidCandidateType(String),
    /// Invalid transport protocol
    InvalidTransportProtocol,
    /// Invalid priority value
    InvalidPriority,
    /// Invalid port number
    InvalidPort,
    /// Invalid IP address
    InvalidIpAddress,
    /// Invalid foundation
    InvalidFoundation,
    /// Missing required field
    MissingRequiredField(&'static str),
    /// No candidates available
    NoCandidates,
    /// Invalid component ID (must be 1 for RTP or 2 for RTCP)
    InvalidComponentId,
    /// Socket binding error
    SocketBindError(String),
    /// Socket operation error
    SocketError(String),
    /// Connectivity check failed
    ConnectivityCheckFailed,
    /// STUN query failed
    StunQueryFailed,
    /// Configuration error
    Configuration(String),
}

impl std::fmt::Display for IceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IceError::InvalidCandidateFormat => write!(f, "Invalid candidate format"),
            IceError::InvalidCandidateType(t) => write!(f, "Invalid candidate type: {}", t),
            IceError::InvalidTransportProtocol => write!(f, "Invalid transport protocol"),
            IceError::InvalidPriority => write!(f, "Invalid priority value"),
            IceError::InvalidPort => write!(f, "Invalid port number"),
            IceError::InvalidIpAddress => write!(f, "Invalid IP address"),
            IceError::InvalidFoundation => write!(f, "Invalid foundation"),
            IceError::MissingRequiredField(field) => write!(f, "Missing required field: {}", field),
            IceError::NoCandidates => write!(f, "No candidates available"),
            IceError::InvalidComponentId => write!(f, "Invalid component ID (must be 1 or 2)"),
            IceError::SocketBindError(e) => write!(f, "Socket bind error: {}", e),
            IceError::StunQueryFailed => write!(f, "STUN query failed"),
            IceError::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            IceError::SocketError(e) => write!(f, "Socket error: {}", e),
            IceError::ConnectivityCheckFailed => write!(f, "Connectivity check failed"),
        }
    }
}

impl std::error::Error for IceError {}
