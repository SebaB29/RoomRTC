//! STUN message builder
//!
//! This module provides a fluent builder API for constructing STUN messages.
//! The builder simplifies message creation with automatic transaction ID generation
//! and attribute management.

use crate::attribute_type::AttributeType;
use crate::errors::StunError;
use crate::message::Message;
use crate::message_type::MessageType;

/// Builder for constructing STUN messages.
///
/// Provides a fluent API for creating STUN messages with attributes.
///
/// # When to Use
///
/// Use `MessageBuilder` when:
/// - Creating STUN messages with automatic transaction ID generation
/// - Building messages with multiple attributes in a fluent way
/// - You need validation before constructing the message
/// - Working with complex message construction scenarios
///
/// # Usage in WebRTC Integration
///
/// 1. **Client-side**: When sending STUN Binding Requests
/// 2. **Server-side**: When creating responses with attributes
/// 3. **Testing**: When you need predictable transaction IDs
/// 4. **Authentication**: When adding USERNAME or MESSAGE-INTEGRITY attributes
/// ```
pub struct MessageBuilder {
    message_type: MessageType,
    transaction_id: Option<[u8; 12]>,
    attributes: Vec<(AttributeType, Vec<u8>)>,
}

impl MessageBuilder {
    /// Creates a new message builder with the specified message type.
    ///
    /// # Arguments
    /// * `message_type` - The type of message to build
    pub fn new(message_type: MessageType) -> Self {
        Self {
            message_type,
            transaction_id: None,
            attributes: Vec::new(),
        }
    }

    /// Generates a random transaction ID using system time.
    pub fn random_transaction_id(mut self) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX_EPOCH - clock may be incorrect")
            .as_nanos();

        let mut id = [0u8; 12];
        let timestamp_bytes = timestamp.to_be_bytes();

        // Use last 12 bytes for better randomness
        id.copy_from_slice(&timestamp_bytes[4..16]);

        self.transaction_id = Some(id);
        self
    }

    /// Builds the STUN message.
    ///
    /// # Returns
    /// * `Ok(Message)` - The constructed message
    /// * `Err(StunError)` - If required fields are missing
    pub fn build(self) -> Result<Message, StunError> {
        let transaction_id = self
            .transaction_id
            .ok_or(StunError::MissingRequiredField("transaction_id"))?;

        let mut message = Message::new(self.message_type, transaction_id);

        for (attr_type, value) in self.attributes {
            message.add_attribute(attr_type, &value);
        }

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_builder_random_transaction_id() {
        let message = MessageBuilder::new(MessageType::Request)
            .random_transaction_id()
            .build()
            .unwrap();

        assert_eq!(message.message_type(), MessageType::Request);
    }

    #[test]
    fn test_message_builder_missing_transaction_id() {
        let result = MessageBuilder::new(MessageType::Request).build();

        assert!(matches!(
            result,
            Err(StunError::MissingRequiredField("transaction_id"))
        ));
    }
}
