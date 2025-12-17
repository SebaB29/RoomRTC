//! STUN error types
//!
//! This module defines the error types used throughout the STUN implementation.
//! All errors are wrapped in `StunError` enum for consistent error handling across
//! STUN protocol operations.

/// Errors that can occur during STUN operations.
#[derive(Debug, Clone, PartialEq)]
pub enum StunError {
    /// Invalid message format
    InvalidMessageFormat,
    /// Invalid message type
    InvalidMessageType(u16),
    /// Invalid attribute type
    InvalidAttributeType(u16),
    /// Invalid magic cookie
    InvalidMagicCookie,
    /// Invalid transaction ID
    InvalidTransactionId,
    /// Invalid attribute format
    InvalidAttributeFormat,
    /// Invalid address family
    InvalidAddressFamily(u8),
    /// Invalid IP address
    InvalidIpAddress,
    /// Invalid port number
    InvalidPort,
    /// Message too short
    MessageTooShort,
    /// Attribute too short
    AttributeTooShort,
    /// Missing required field
    MissingRequiredField(&'static str),
    /// Socket bind error
    SocketBindError(String),
    /// Socket operation error
    SocketError(String),
    /// Timeout waiting for response
    Timeout,
    /// Transaction ID mismatch
    TransactionIdMismatch,
    /// Unexpected message type
    UnexpectedMessageType,
}

impl std::fmt::Display for StunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StunError::InvalidMessageFormat => write!(f, "Invalid message format"),
            StunError::InvalidMessageType(t) => write!(f, "Invalid message type: 0x{:04X}", t),
            StunError::InvalidAttributeType(t) => write!(f, "Invalid attribute type: 0x{:04X}", t),
            StunError::InvalidMagicCookie => write!(f, "Invalid magic cookie"),
            StunError::InvalidTransactionId => write!(f, "Invalid transaction ID"),
            StunError::InvalidAttributeFormat => write!(f, "Invalid attribute format"),
            StunError::InvalidAddressFamily(fam) => {
                write!(f, "Invalid address family: 0x{:02X}", fam)
            }
            StunError::InvalidIpAddress => write!(f, "Invalid IP address"),
            StunError::InvalidPort => write!(f, "Invalid port number"),
            StunError::MessageTooShort => write!(f, "Message too short"),
            StunError::AttributeTooShort => write!(f, "Attribute too short"),
            StunError::MissingRequiredField(field) => {
                write!(f, "Missing required field: {}", field)
            }
            StunError::SocketBindError(e) => write!(f, "Socket bind error: {}", e),
            StunError::SocketError(e) => write!(f, "Socket error: {}", e),
            StunError::Timeout => write!(f, "Timeout waiting for response"),
            StunError::TransactionIdMismatch => write!(f, "Transaction ID mismatch"),
            StunError::UnexpectedMessageType => write!(f, "Unexpected message type"),
        }
    }
}

impl std::error::Error for StunError {}

impl From<std::io::Error> for StunError {
    fn from(err: std::io::Error) -> Self {
        match err.kind() {
            std::io::ErrorKind::TimedOut => StunError::Timeout,
            _ => StunError::SocketError(err.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display_invalid_message_format() {
        let err = StunError::InvalidMessageFormat;
        assert_eq!(err.to_string(), "Invalid message format");
    }

    #[test]
    fn test_error_display_invalid_message_type() {
        let err = StunError::InvalidMessageType(0x9999);
        assert_eq!(err.to_string(), "Invalid message type: 0x9999");
    }

    #[test]
    fn test_error_display_invalid_attribute_type() {
        let err = StunError::InvalidAttributeType(0x1234);
        assert_eq!(err.to_string(), "Invalid attribute type: 0x1234");
    }

    #[test]
    fn test_error_display_invalid_magic_cookie() {
        let err = StunError::InvalidMagicCookie;
        assert_eq!(err.to_string(), "Invalid magic cookie");
    }

    #[test]
    fn test_error_display_invalid_address_family() {
        let err = StunError::InvalidAddressFamily(0x99);
        assert_eq!(err.to_string(), "Invalid address family: 0x99");
    }

    #[test]
    fn test_error_display_missing_required_field() {
        let err = StunError::MissingRequiredField("transaction_id");
        assert_eq!(err.to_string(), "Missing required field: transaction_id");
    }

    #[test]
    fn test_error_display_socket_bind_error() {
        let err = StunError::SocketBindError("Address already in use".to_string());
        assert_eq!(err.to_string(), "Socket bind error: Address already in use");
    }

    #[test]
    fn test_error_display_socket_error() {
        let err = StunError::SocketError("Connection refused".to_string());
        assert_eq!(err.to_string(), "Socket error: Connection refused");
    }

    #[test]
    fn test_error_display_timeout() {
        let err = StunError::Timeout;
        assert_eq!(err.to_string(), "Timeout waiting for response");
    }

    #[test]
    fn test_error_display_transaction_id_mismatch() {
        let err = StunError::TransactionIdMismatch;
        assert_eq!(err.to_string(), "Transaction ID mismatch");
    }

    #[test]
    fn test_error_from_io_timeout() {
        let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "timeout");
        let stun_err: StunError = io_err.into();
        assert!(matches!(stun_err, StunError::Timeout));
    }

    #[test]
    fn test_error_from_io_other() {
        let io_err = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let stun_err: StunError = io_err.into();
        assert!(matches!(stun_err, StunError::SocketError(_)));
    }

    #[test]
    fn test_error_is_error_trait() {
        let err = StunError::InvalidMessageFormat;
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_error_clone() {
        let err1 = StunError::InvalidMessageFormat;
        let err2 = err1.clone();
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_error_debug() {
        let err = StunError::InvalidMessageFormat;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("InvalidMessageFormat"));
    }
}
