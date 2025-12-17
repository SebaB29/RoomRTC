//! STUN message structure
//!
//! This module implements the complete STUN message structure according to RFC 5389.
//! A STUN message consists of a 20-byte header followed by zero or more attributes.

use crate::attribute_type::AttributeType;
use crate::errors::StunError;
use crate::message_header::MessageHeader;
use crate::message_type::MessageType;

/// A complete STUN message according to RFC 5389.
///
/// A STUN message consists of a 20-byte header followed by zero or more attributes.
#[derive(Debug, Clone)]
pub struct Message {
    /// The message header
    pub header: MessageHeader,
    /// Encoded attributes (type-length-value format)
    pub attributes: Vec<u8>,
}

impl Message {
    /// Creates a new STUN message.
    ///
    /// # Arguments
    /// * `message_type` - The message type
    /// * `transaction_id` - The transaction ID (12 bytes)
    ///
    /// # Returns
    /// A new Message with no attributes
    pub fn new(message_type: MessageType, transaction_id: [u8; 12]) -> Self {
        Self {
            header: MessageHeader::new(message_type, transaction_id),
            attributes: Vec::new(),
        }
    }

    /// Adds an attribute to the message.
    ///
    /// Attributes are automatically padded to 4-byte boundaries as required by RFC 5389.
    ///
    /// # Arguments
    /// * `attr_type` - The attribute type
    /// * `value` - The attribute value
    pub fn add_attribute(&mut self, attr_type: AttributeType, value: &[u8]) {
        // Calculate total size with padding
        let padding = (4 - (value.len() % 4)) % 4;
        let total_size = 4 + value.len() + padding; // type(2) + length(2) + value + padding

        // Reserve capacity to avoid multiple allocations
        self.attributes.reserve(total_size);

        // Attribute type (2 bytes)
        self.attributes
            .extend_from_slice(&attr_type.to_u16().to_be_bytes());

        // Attribute length (2 bytes)
        let length = value.len() as u16;
        self.attributes.extend_from_slice(&length.to_be_bytes());

        // Attribute value
        self.attributes.extend_from_slice(value);

        // Padding to 4-byte boundary
        if padding > 0 {
            self.attributes.resize(self.attributes.len() + padding, 0);
        }

        // Update message length in header
        self.header.set_message_length(self.attributes.len() as u16);
    }

    /// Encodes the complete message to bytes.
    ///
    /// # Returns
    /// A vector containing the encoded message
    pub fn encode(&self) -> Vec<u8> {
        let total_len = MessageHeader::SIZE + self.attributes.len();
        let mut bytes = Vec::with_capacity(total_len);
        bytes.extend_from_slice(&self.header.encode());
        bytes.extend_from_slice(&self.attributes);
        bytes
    }

    /// Decodes a message from bytes.
    ///
    /// # Arguments
    /// * `bytes` - The bytes to decode
    ///
    /// # Returns
    /// * `Ok(Message)` - If decoding succeeds
    /// * `Err(StunError)` - If the format is invalid
    pub fn decode(bytes: &[u8]) -> Result<Self, StunError> {
        let header = MessageHeader::decode(bytes)?;

        let total_len = MessageHeader::SIZE + header.message_length as usize;
        if bytes.len() < total_len {
            return Err(StunError::MessageTooShort);
        }

        let attributes = bytes[MessageHeader::SIZE..total_len].to_vec();

        Ok(Self { header, attributes })
    }

    /// Gets the message type.
    pub fn message_type(&self) -> MessageType {
        self.header.message_type
    }

    /// Gets the transaction ID.
    pub fn transaction_id(&self) -> [u8; 12] {
        self.header.transaction_id
    }

    /// Gets the attributes as a byte slice.
    pub fn attributes_bytes(&self) -> &[u8] {
        &self.attributes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let message = Message::new(MessageType::Request, transaction_id);

        assert_eq!(message.message_type(), MessageType::Request);
        assert_eq!(message.transaction_id(), transaction_id);
        assert!(message.attributes.is_empty());
    }

    #[test]
    fn test_message_add_attribute() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut message = Message::new(MessageType::Request, transaction_id);

        let value = vec![1, 2, 3, 4];
        message.add_attribute(AttributeType::XorMappedAddress, &value);

        assert!(!message.attributes.is_empty());
        assert_eq!(
            message.header.message_length,
            message.attributes.len() as u16
        );
    }

    #[test]
    fn test_message_encode_decode() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let mut message = Message::new(MessageType::Request, transaction_id);

        let value = vec![1, 2, 3, 4];
        message.add_attribute(AttributeType::XorMappedAddress, &value);

        let encoded = message.encode();
        let decoded = Message::decode(&encoded).unwrap();

        assert_eq!(decoded.message_type(), message.message_type());
        assert_eq!(decoded.transaction_id(), message.transaction_id());
        assert_eq!(decoded.attributes, message.attributes);
    }
}
