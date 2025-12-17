//! STUN message types
//!
//! This module defines the message types used in STUN protocol according to RFC 5389.
//! STUN messages are categorized into requests, success responses, and error responses.

use crate::errors::StunError;

/// STUN message types according to RFC 5389.
///
/// # Message Classes
/// - Request: 0x000
/// - Indication: 0x010
/// - Success Response: 0x100
/// - Error Response: 0x110
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    /// Binding Request (0x0001)
    Request,
    /// Binding Response (0x0101)
    Response,
    /// Binding Error Response (0x0111)
    ErrorResponse,
}

impl MessageType {
    /// Converts the message type to its RFC 5389 value.
    ///
    /// # Returns
    /// The u16 representation of the message type
    pub fn to_u16(self) -> u16 {
        match self {
            MessageType::Request => 0x0001,
            MessageType::Response => 0x0101,
            MessageType::ErrorResponse => 0x0111,
        }
    }

    /// Parses a message type from its RFC 5389 value.
    ///
    /// # Arguments
    /// * `value` - The u16 value to parse
    ///
    /// # Returns
    /// * `Ok(MessageType)` - If the value is valid
    /// * `Err(StunError)` - If the value is not recognized
    pub fn from_u16(value: u16) -> Result<Self, StunError> {
        match value {
            0x0001 => Ok(MessageType::Request),
            0x0101 => Ok(MessageType::Response),
            0x0111 => Ok(MessageType::ErrorResponse),
            _ => Err(StunError::InvalidMessageType(value)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_to_u16() {
        assert_eq!(MessageType::Request.to_u16(), 0x0001);
        assert_eq!(MessageType::Response.to_u16(), 0x0101);
        assert_eq!(MessageType::ErrorResponse.to_u16(), 0x0111);
    }

    #[test]
    fn test_message_type_from_u16() {
        assert_eq!(MessageType::from_u16(0x0001).unwrap(), MessageType::Request);
        assert_eq!(
            MessageType::from_u16(0x0101).unwrap(),
            MessageType::Response
        );
        assert_eq!(
            MessageType::from_u16(0x0111).unwrap(),
            MessageType::ErrorResponse
        );
        assert!(MessageType::from_u16(0x9999).is_err());
    }
}
