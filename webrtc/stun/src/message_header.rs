//! STUN message header
//!
//! This module implements the STUN message header structure according to RFC 5389.
//! The header is a fixed 20-byte structure that precedes all STUN message bodies.
//!
//! # Header Format (RFC 5389 Section 6)
//!
//! ```text
//!  0                   1                   2                   3
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |0 0|     STUN Message Type     |         Message Length        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                         Magic Cookie                          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                                                               |
//! |                     Transaction ID (96 bits)                  |
//! |                                                               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

use crate::errors::StunError;
use crate::message_type::MessageType;

/// Magic cookie value defined in RFC 5389.
/// This value is used to identify STUN messages and for XOR operations.
pub const MAGIC_COOKIE: u32 = 0x2112A442;

/// STUN message header according to RFC 5389.
///
/// The header is 20 bytes and contains:
/// - Message Type (2 bytes)
/// - Message Length (2 bytes)
/// - Magic Cookie (4 bytes)
/// - Transaction ID (12 bytes)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageHeader {
    /// The type of the message
    pub message_type: MessageType,
    /// Length of the message body (excluding the header)
    pub message_length: u16,
    /// Unique transaction identifier
    pub transaction_id: [u8; 12],
}

impl MessageHeader {
    /// Size of the STUN message header in bytes.
    pub const SIZE: usize = 20;

    /// Creates a new message header.
    ///
    /// # Arguments
    /// * `message_type` - The message type
    /// * `transaction_id` - The transaction ID (12 bytes)
    ///
    /// # Returns
    /// A new MessageHeader with message_length initialized to 0
    pub fn new(message_type: MessageType, transaction_id: [u8; 12]) -> Self {
        Self {
            message_type,
            message_length: 0,
            transaction_id,
        }
    }

    /// Encodes the header to bytes.
    ///
    /// # Returns
    /// A vector containing the 20-byte encoded header
    pub fn encode(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::SIZE);

        // Message type (2 bytes)
        bytes.extend_from_slice(&self.message_type.to_u16().to_be_bytes());

        // Message length (2 bytes)
        bytes.extend_from_slice(&self.message_length.to_be_bytes());

        // Magic cookie (4 bytes)
        bytes.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());

        // Transaction ID (12 bytes)
        bytes.extend_from_slice(&self.transaction_id);

        bytes
    }

    /// Decodes a header from bytes.
    ///
    /// # Arguments
    /// * `bytes` - The bytes to decode (must be at least 20 bytes)
    ///
    /// # Returns
    /// * `Ok(MessageHeader)` - If decoding succeeds
    /// * `Err(StunError)` - If the format is invalid
    pub fn decode(bytes: &[u8]) -> Result<Self, StunError> {
        if bytes.len() < Self::SIZE {
            return Err(StunError::MessageTooShort);
        }

        // Parse message type
        let msg_type_value = u16::from_be_bytes([bytes[0], bytes[1]]);
        let message_type = MessageType::from_u16(msg_type_value)?;

        // Parse message length
        let message_length = u16::from_be_bytes([bytes[2], bytes[3]]);

        // Verify magic cookie
        let magic = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        if magic != MAGIC_COOKIE {
            return Err(StunError::InvalidMagicCookie);
        }

        // Parse transaction ID
        let mut transaction_id = [0u8; 12];
        transaction_id.copy_from_slice(&bytes[8..20]);

        Ok(Self {
            message_type,
            message_length,
            transaction_id,
        })
    }

    /// Updates the message length field.
    ///
    /// # Arguments
    /// * `length` - The new message length
    pub fn set_message_length(&mut self, length: u16) {
        self.message_length = length;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_header_encode_decode() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let header = MessageHeader::new(MessageType::Request, transaction_id);

        let encoded = header.encode();
        assert_eq!(encoded.len(), MessageHeader::SIZE);

        let decoded = MessageHeader::decode(&encoded).unwrap();
        assert_eq!(decoded.message_type, MessageType::Request);
        assert_eq!(decoded.transaction_id, transaction_id);
        assert_eq!(decoded.message_length, 0);
    }

    #[test]
    fn test_message_header_decode_invalid_magic_cookie() {
        let mut bytes = vec![0u8; 20];
        // Valid message type
        bytes[0] = 0x00;
        bytes[1] = 0x01;
        // Invalid magic cookie
        bytes[4] = 0xFF;
        bytes[5] = 0xFF;
        bytes[6] = 0xFF;
        bytes[7] = 0xFF;

        let result = MessageHeader::decode(&bytes);
        assert!(matches!(result, Err(StunError::InvalidMagicCookie)));
    }

    #[test]
    fn test_message_header_decode_too_short() {
        let bytes = vec![0u8; 10];
        let result = MessageHeader::decode(&bytes);
        assert!(matches!(result, Err(StunError::MessageTooShort)));
    }

    #[test]
    fn test_set_message_length() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut header = MessageHeader::new(MessageType::Request, transaction_id);

        header.set_message_length(100);
        assert_eq!(header.message_length, 100);
    }
}
