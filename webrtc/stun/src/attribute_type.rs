//! STUN attribute types
//!
//! This module defines the attribute types used in STUN messages according to RFC 5389.
//! Attributes provide additional information in STUN messages using Type-Length-Value (TLV) format.

/// STUN attribute types according to RFC 5389.
///
/// Attributes provide additional information in STUN messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeType {
    /// XOR-MAPPED-ADDRESS (0x0020) - XOR'd reflexive transport address (recommended)
    XorMappedAddress,
}

impl AttributeType {
    /// Converts the attribute type to its RFC 5389 value.
    ///
    /// # Returns
    /// The u16 representation of the attribute type
    pub fn to_u16(self) -> u16 {
        match self {
            AttributeType::XorMappedAddress => 0x0020,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_type_to_u16() {
        assert_eq!(AttributeType::XorMappedAddress.to_u16(), 0x0020);
    }
}
