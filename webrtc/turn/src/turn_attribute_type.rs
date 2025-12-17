//! TURN-specific attributes.
//!
//! TURN extends STUN with additional attributes for relay functionality.

/// TURN attribute types according to RFC 5766.
///
/// These attributes are used in TURN messages to convey
/// relay-specific information.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnAttributeType {
    /// CHANNEL-NUMBER: 0x000C
    /// The channel number for ChannelBind requests
    ChannelNumber = 0x000C,

    /// LIFETIME: 0x000D
    /// The duration for which the allocation should be maintained (in seconds)
    Lifetime = 0x000D,

    /// XOR-PEER-ADDRESS: 0x0012
    /// The peer's transport address (XOR-ed with magic cookie)
    XorPeerAddress = 0x0012,

    /// DATA: 0x0013
    /// Application data being relayed
    Data = 0x0013,

    /// XOR-RELAYED-ADDRESS: 0x0016
    /// The allocated relay address (XOR-ed with magic cookie)
    XorRelayedAddress = 0x0016,

    /// REQUESTED-TRANSPORT: 0x0019
    /// The transport protocol for the allocation (UDP=17, TCP=6)
    RequestedTransport = 0x0019,

    /// DONT-FRAGMENT: 0x001A
    /// Request that the DF flag be set on outgoing packets
    DontFragment = 0x001A,

    /// RESERVATION-TOKEN: 0x0022
    /// Token for reserving an allocation
    ReservationToken = 0x0022,
}

impl TurnAttributeType {
    /// Converts attribute type to its 16-bit numeric value.
    ///
    /// # Returns
    /// The attribute type code used in TURN message encoding
    pub fn to_u16(self) -> u16 {
        self as u16
    }

    /// Parses a TURN attribute type from its numeric value.
    ///
    /// # Arguments
    /// * `value` - The 16-bit attribute type value
    ///
    /// # Returns
    /// * `Some(TurnAttributeType)` - If the value represents a valid TURN attribute
    /// * `None` - If the value is not a recognized TURN attribute
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x000C => Some(TurnAttributeType::ChannelNumber),
            0x000D => Some(TurnAttributeType::Lifetime),
            0x0012 => Some(TurnAttributeType::XorPeerAddress),
            0x0013 => Some(TurnAttributeType::Data),
            0x0016 => Some(TurnAttributeType::XorRelayedAddress),
            0x0019 => Some(TurnAttributeType::RequestedTransport),
            0x001A => Some(TurnAttributeType::DontFragment),
            0x0022 => Some(TurnAttributeType::ReservationToken),
            _ => None,
        }
    }

    /// Returns the string representation of the attribute type.
    pub fn as_str(&self) -> &'static str {
        match self {
            TurnAttributeType::ChannelNumber => "CHANNEL-NUMBER",
            TurnAttributeType::Lifetime => "LIFETIME",
            TurnAttributeType::XorPeerAddress => "XOR-PEER-ADDRESS",
            TurnAttributeType::Data => "DATA",
            TurnAttributeType::XorRelayedAddress => "XOR-RELAYED-ADDRESS",
            TurnAttributeType::RequestedTransport => "REQUESTED-TRANSPORT",
            TurnAttributeType::DontFragment => "DONT-FRAGMENT",
            TurnAttributeType::ReservationToken => "RESERVATION-TOKEN",
        }
    }
}

/// TURN transport protocol values.
///
/// Used in the REQUESTED-TRANSPORT attribute to specify
/// the transport protocol for the allocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportProtocol {
    /// UDP transport (protocol number 17)
    Udp = 17,
    /// TCP transport (protocol number 6)
    Tcp = 6,
}

impl TransportProtocol {
    /// Converts transport protocol to its 8-bit numeric value.
    ///
    /// # Returns
    /// The IANA protocol number (17 for UDP, 6 for TCP)
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    /// Parses a transport protocol from its numeric value.
    ///
    /// # Arguments
    /// * `value` - The IANA protocol number
    ///
    /// # Returns
    /// * `Some(TransportProtocol)` - If the value is 6 (TCP) or 17 (UDP)
    /// * `None` - If the value is not recognized
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            17 => Some(TransportProtocol::Udp),
            6 => Some(TransportProtocol::Tcp),
            _ => None,
        }
    }

    /// Returns the string representation of the transport protocol.
    pub fn as_str(&self) -> &'static str {
        match self {
            TransportProtocol::Udp => "UDP",
            TransportProtocol::Tcp => "TCP",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_attribute_type_conversion() {
        assert_eq!(TurnAttributeType::Lifetime.to_u16(), 0x000D);
        assert_eq!(TurnAttributeType::XorRelayedAddress.to_u16(), 0x0016);
        assert_eq!(TurnAttributeType::Data.to_u16(), 0x0013);
        assert_eq!(TurnAttributeType::ChannelNumber.to_u16(), 0x000C);
    }

    #[test]
    fn test_turn_attribute_type_parsing() {
        assert_eq!(
            TurnAttributeType::from_u16(0x000D),
            Some(TurnAttributeType::Lifetime)
        );
        assert_eq!(
            TurnAttributeType::from_u16(0x0016),
            Some(TurnAttributeType::XorRelayedAddress)
        );
        assert_eq!(
            TurnAttributeType::from_u16(0x0013),
            Some(TurnAttributeType::Data)
        );
        assert_eq!(TurnAttributeType::from_u16(0xFFFF), None);
    }

    #[test]
    fn test_attribute_as_str() {
        assert_eq!(TurnAttributeType::Lifetime.as_str(), "LIFETIME");
        assert_eq!(
            TurnAttributeType::XorRelayedAddress.as_str(),
            "XOR-RELAYED-ADDRESS"
        );
        assert_eq!(TurnAttributeType::Data.as_str(), "DATA");
    }

    #[test]
    fn test_transport_protocol_conversion() {
        assert_eq!(TransportProtocol::Udp.to_u8(), 17);
        assert_eq!(TransportProtocol::Tcp.to_u8(), 6);
    }

    #[test]
    fn test_transport_protocol_parsing() {
        assert_eq!(TransportProtocol::from_u8(17), Some(TransportProtocol::Udp));
        assert_eq!(TransportProtocol::from_u8(6), Some(TransportProtocol::Tcp));
        assert_eq!(TransportProtocol::from_u8(99), None);
    }

    #[test]
    fn test_transport_protocol_as_str() {
        assert_eq!(TransportProtocol::Udp.as_str(), "UDP");
        assert_eq!(TransportProtocol::Tcp.as_str(), "TCP");
    }
}
