//! TURN message structure and utilities.
//!
//! This module provides the core TURN message structure and helper functions
//! for building, parsing, and manipulating TURN messages.

use crate::turn_message_type::TurnMessageType;

/// STUN magic cookie constant (RFC 5389)
const MAGIC_COOKIE: u32 = 0x2112A442;

/// TURN message structure.
///
/// TURN messages follow the STUN message format with TURN-specific
/// message types and attributes.
#[derive(Debug, Clone)]
pub struct TurnMessage {
    /// The type of TURN message
    pub message_type: TurnMessageType,
    /// 96-bit transaction identifier
    pub transaction_id: [u8; 12],
    /// Raw message bytes (including header and attributes)
    pub raw_bytes: Vec<u8>,
}

impl TurnMessage {
    /// Creates a new TURN message with the specified type and transaction ID.
    ///
    /// # Arguments
    /// * `message_type` - The type of TURN message to create
    /// * `transaction_id` - 96-bit transaction identifier
    ///
    /// # Returns
    /// A new `TurnMessage` instance with an empty attributes section
    pub fn new(message_type: TurnMessageType, transaction_id: [u8; 12]) -> Self {
        let raw_bytes = build_turn_message(message_type, transaction_id);
        Self {
            message_type,
            transaction_id,
            raw_bytes,
        }
    }

    /// Returns a reference to the raw message bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw_bytes
    }

    /// Adds an attribute to this TURN message.
    ///
    /// # Arguments
    /// * `attr_type` - The attribute type code
    /// * `value` - The attribute value bytes
    pub fn add_attribute(&mut self, attr_type: u16, value: &[u8]) {
        add_turn_attribute(&mut self.raw_bytes, attr_type, value);
    }
}

/// Generates a random 96-bit transaction ID.
///
/// # Returns
/// A 12-byte array containing a unique transaction identifier
pub fn generate_transaction_id() -> [u8; 12] {
    use std::time::{SystemTime, UNIX_EPOCH};

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before UNIX_EPOCH - clock may be incorrect")
        .as_nanos();

    let mut id = [0u8; 12];
    let bytes = timestamp.to_be_bytes();
    id[..12].copy_from_slice(&bytes[4..16]);
    id
}

/// Builds a raw TURN message with the specified message type.
///
/// # Arguments
/// * `message_type` - The TURN message type
/// * `transaction_id` - The 96-bit transaction identifier
///
/// # Returns
/// A vector containing the 20-byte TURN message header
pub fn build_turn_message(message_type: TurnMessageType, transaction_id: [u8; 12]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(20);

    // Message type (2 bytes)
    bytes.extend_from_slice(&message_type.to_u16().to_be_bytes());

    // Message length (2 bytes) - initially 0, will be updated when attributes are added
    bytes.extend_from_slice(&0u16.to_be_bytes());

    // Magic cookie (4 bytes)
    bytes.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());

    // Transaction ID (12 bytes)
    bytes.extend_from_slice(&transaction_id);

    bytes
}

/// Adds an attribute to a TURN message.
///
/// Attributes are added in TLV (Type-Length-Value) format and padded
/// to 4-byte boundaries as required by the STUN/TURN protocol.
///
/// # Arguments
/// * `message` - The message buffer to append the attribute to
/// * `attr_type` - The attribute type code (16-bit)
/// * `value` - The attribute value bytes
pub fn add_turn_attribute(message: &mut Vec<u8>, attr_type: u16, value: &[u8]) {
    // Calculate padding to align to 4-byte boundary
    let padding = (4 - (value.len() % 4)) % 4;

    // Attribute type (2 bytes)
    message.extend_from_slice(&attr_type.to_be_bytes());

    // Attribute length (2 bytes) - unpadded length
    let length = value.len() as u16;
    message.extend_from_slice(&length.to_be_bytes());

    // Attribute value
    message.extend_from_slice(value);

    // Padding to 4-byte boundary
    if padding > 0 {
        message.resize(message.len() + padding, 0);
    }

    // Update message length in header (bytes 2-3)
    // Message length = total length - 20 byte header
    let attr_len = (message.len() - 20) as u16;
    message[2..4].copy_from_slice(&attr_len.to_be_bytes());
}

/// Parses a TURN message type from raw message bytes.
///
/// # Arguments
/// * `bytes` - The raw message bytes
///
/// # Returns
/// * `Some(TurnMessageType)` - If a valid TURN message type is found
/// * `None` - If the bytes are too short or contain an invalid message type
pub fn parse_turn_message_type(bytes: &[u8]) -> Option<TurnMessageType> {
    if bytes.len() < 2 {
        return None;
    }

    let type_value = u16::from_be_bytes([bytes[0], bytes[1]]);
    TurnMessageType::from_u16(type_value)
}

/// Extracts the transaction ID from a TURN message.
///
/// # Arguments
/// * `bytes` - The raw message bytes (must be at least 20 bytes)
///
/// # Returns
/// * `Some([u8; 12])` - The extracted transaction ID
/// * `None` - If the message is too short
pub fn extract_transaction_id(bytes: &[u8]) -> Option<[u8; 12]> {
    if bytes.len() < 20 {
        return None;
    }

    let mut id = [0u8; 12];
    id.copy_from_slice(&bytes[8..20]);
    Some(id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_transaction_id() {
        let id1 = generate_transaction_id();
        // Small delay to ensure different timestamps
        std::thread::sleep(std::time::Duration::from_nanos(100));
        let id2 = generate_transaction_id();

        assert_eq!(id1.len(), 12);
        assert_eq!(id2.len(), 12);
        // IDs should be different (with high probability due to timing)
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_build_turn_message() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let message = build_turn_message(TurnMessageType::AllocateRequest, transaction_id);

        assert_eq!(message.len(), 20);
        // Check message type
        assert_eq!(u16::from_be_bytes([message[0], message[1]]), 0x0003);
        // Check message length (should be 0 initially)
        assert_eq!(u16::from_be_bytes([message[2], message[3]]), 0);
        // Check magic cookie
        assert_eq!(
            u32::from_be_bytes([message[4], message[5], message[6], message[7]]),
            MAGIC_COOKIE
        );
        // Check transaction ID
        assert_eq!(&message[8..20], &transaction_id);
    }

    #[test]
    fn test_add_turn_attribute() {
        let transaction_id = [0u8; 12];
        let mut message = build_turn_message(TurnMessageType::AllocateRequest, transaction_id);

        // Add a 4-byte attribute (no padding needed)
        let value = [1, 2, 3, 4];
        add_turn_attribute(&mut message, 0x000D, &value);

        // Header (20) + Type (2) + Length (2) + Value (4) = 28 bytes
        assert_eq!(message.len(), 28);

        // Check message length was updated (should be 8: type+length+value)
        let msg_len = u16::from_be_bytes([message[2], message[3]]);
        assert_eq!(msg_len, 8);
    }

    #[test]
    fn test_add_turn_attribute_with_padding() {
        let transaction_id = [0u8; 12];
        let mut message = build_turn_message(TurnMessageType::AllocateRequest, transaction_id);

        // Add a 3-byte attribute (needs 1 byte padding)
        let value = [1, 2, 3];
        add_turn_attribute(&mut message, 0x000D, &value);

        // Header (20) + Type (2) + Length (2) + Value (3) + Padding (1) = 28 bytes
        assert_eq!(message.len(), 28);

        // Check message length was updated (should be 8: type+length+value+padding)
        let msg_len = u16::from_be_bytes([message[2], message[3]]);
        assert_eq!(msg_len, 8);
    }

    #[test]
    fn test_parse_turn_message_type() {
        let bytes = vec![0x00, 0x03, 0x00, 0x00]; // AllocateRequest
        assert_eq!(
            parse_turn_message_type(&bytes),
            Some(TurnMessageType::AllocateRequest)
        );

        let bytes = vec![0x01, 0x04]; // RefreshResponse
        assert_eq!(
            parse_turn_message_type(&bytes),
            Some(TurnMessageType::RefreshResponse)
        );

        let bytes = vec![0xFF]; // Too short
        assert_eq!(parse_turn_message_type(&bytes), None);
    }

    #[test]
    fn test_extract_transaction_id() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let message = build_turn_message(TurnMessageType::AllocateRequest, transaction_id);

        assert_eq!(extract_transaction_id(&message), Some(transaction_id));

        let short_message = vec![0u8; 10]; // Too short
        assert_eq!(extract_transaction_id(&short_message), None);
    }

    #[test]
    fn test_turn_message_new() {
        let transaction_id = [0u8; 12];
        let message = TurnMessage::new(TurnMessageType::AllocateRequest, transaction_id);

        assert_eq!(message.message_type, TurnMessageType::AllocateRequest);
        assert_eq!(message.transaction_id, transaction_id);
        assert_eq!(message.as_bytes().len(), 20);
    }

    #[test]
    fn test_turn_message_add_attribute() {
        let transaction_id = [0u8; 12];
        let mut message = TurnMessage::new(TurnMessageType::AllocateRequest, transaction_id);

        let lifetime = 600u32.to_be_bytes();
        message.add_attribute(0x000D, &lifetime);

        assert!(message.as_bytes().len() > 20);
    }
}
