//! Data Channel Establishment Protocol (DCEP)
//!
//! DCEP is used to negotiate data channel parameters over SCTP.
//! Defined in RFC 8832.
//!
//! ```text
//!  0                   1                   2                   3
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  Message Type |  Channel Type |            Priority           |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                    Reliability Parameter                      |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |         Label Length          |       Protocol Length         |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! \                                                               /
//! /                             Label                             \
//! \                                                               /
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! \                                                               /
//! /                            Protocol                           \
//! \                                                               /
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

use std::io;

/// DCEP message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DcepMessageType {
    /// Request to open a data channel
    DataChannelOpen = 0x03,
    /// Acknowledge data channel open
    DataChannelAck = 0x02,
}

impl DcepMessageType {
    /// Parse from byte
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x03 => Some(Self::DataChannelOpen),
            0x02 => Some(Self::DataChannelAck),
            _ => None,
        }
    }
}

/// Data channel types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ChannelType {
    /// Reliable ordered (TCP-like)
    Reliable = 0x00,
    /// Reliable unordered
    ReliableUnordered = 0x80,
    /// Partial reliable with limited retransmits
    PartialReliableRexmit = 0x01,
    /// Partial reliable unordered with limited retransmits
    PartialReliableRexmitUnordered = 0x81,
    /// Partial reliable with lifetime limit
    PartialReliableTimed = 0x02,
    /// Partial reliable unordered with lifetime limit
    PartialReliableTimedUnordered = 0x82,
}

impl ChannelType {
    /// Parse from byte
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x00 => Some(Self::Reliable),
            0x80 => Some(Self::ReliableUnordered),
            0x01 => Some(Self::PartialReliableRexmit),
            0x81 => Some(Self::PartialReliableRexmitUnordered),
            0x02 => Some(Self::PartialReliableTimed),
            0x82 => Some(Self::PartialReliableTimedUnordered),
            _ => None,
        }
    }

    /// Check if channel is ordered
    pub fn is_ordered(&self) -> bool {
        matches!(
            self,
            Self::Reliable | Self::PartialReliableRexmit | Self::PartialReliableTimed
        )
    }
}

/// DATA_CHANNEL_OPEN message
#[derive(Debug, Clone)]
pub struct DataChannelOpen {
    /// Channel type
    pub channel_type: ChannelType,
    /// Priority (0-65535)
    pub priority: u16,
    /// Reliability parameter (meaning depends on channel_type)
    pub reliability_param: u32,
    /// Channel label (human-readable name)
    pub label: String,
    /// Sub-protocol
    pub protocol: String,
}

impl DataChannelOpen {
    /// Create a new reliable ordered data channel
    pub fn new_reliable(label: String) -> Self {
        Self {
            channel_type: ChannelType::Reliable,
            priority: 0,
            reliability_param: 0,
            label,
            protocol: String::new(),
        }
    }

    /// Create a file transfer channel
    pub fn new_file_transfer() -> Self {
        Self::new_reliable("file-transfer".to_string())
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let label_bytes = self.label.as_bytes();
        let protocol_bytes = self.protocol.as_bytes();
        let total_len = 12 + label_bytes.len() + protocol_bytes.len();

        let mut buf = Vec::with_capacity(total_len);

        // Message type
        buf.push(DcepMessageType::DataChannelOpen as u8);
        // Channel type
        buf.push(self.channel_type as u8);
        // Priority
        buf.extend_from_slice(&self.priority.to_be_bytes());
        // Reliability parameter
        buf.extend_from_slice(&self.reliability_param.to_be_bytes());
        // Label length
        buf.extend_from_slice(&(label_bytes.len() as u16).to_be_bytes());
        // Protocol length
        buf.extend_from_slice(&(protocol_bytes.len() as u16).to_be_bytes());
        // Label
        buf.extend_from_slice(label_bytes);
        // Protocol
        buf.extend_from_slice(protocol_bytes);

        buf
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 12 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DATA_CHANNEL_OPEN too short",
            ));
        }

        let msg_type = data[0];
        if msg_type != DcepMessageType::DataChannelOpen as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a DATA_CHANNEL_OPEN message",
            ));
        }

        let channel_type = ChannelType::from_u8(data[1])
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Invalid channel type"))?;

        let priority = u16::from_be_bytes([data[2], data[3]]);
        let reliability_param = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let label_len = u16::from_be_bytes([data[8], data[9]]) as usize;
        let protocol_len = u16::from_be_bytes([data[10], data[11]]) as usize;

        if data.len() < 12 + label_len + protocol_len {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DATA_CHANNEL_OPEN truncated",
            ));
        }

        let label = String::from_utf8(data[12..12 + label_len].to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid label UTF-8"))?;

        let protocol =
            String::from_utf8(data[12 + label_len..12 + label_len + protocol_len].to_vec())
                .map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "Invalid protocol UTF-8")
                })?;

        Ok(Self {
            channel_type,
            priority,
            reliability_param,
            label,
            protocol,
        })
    }
}

/// DATA_CHANNEL_ACK message
#[derive(Debug, Clone, Copy, Default)]
pub struct DataChannelAck;

impl DataChannelAck {
    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        vec![DcepMessageType::DataChannelAck as u8]
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.is_empty() || data[0] != DcepMessageType::DataChannelAck as u8 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Not a DATA_CHANNEL_ACK message",
            ));
        }
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_channel_open_roundtrip() {
        let open = DataChannelOpen {
            channel_type: ChannelType::Reliable,
            priority: 256,
            reliability_param: 0,
            label: "test-channel".to_string(),
            protocol: "".to_string(),
        };

        let bytes = open.to_bytes();
        let parsed = DataChannelOpen::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.channel_type, ChannelType::Reliable);
        assert_eq!(parsed.priority, 256);
        assert_eq!(parsed.label, "test-channel");
    }

    #[test]
    fn test_file_transfer_channel() {
        let open = DataChannelOpen::new_file_transfer();
        assert_eq!(open.label, "file-transfer");
        assert_eq!(open.channel_type, ChannelType::Reliable);
    }

    #[test]
    fn test_data_channel_ack() {
        let ack = DataChannelAck;
        let bytes = ack.to_bytes();
        assert_eq!(bytes.len(), 1);
        assert!(DataChannelAck::from_bytes(&bytes).is_ok());
    }
}
