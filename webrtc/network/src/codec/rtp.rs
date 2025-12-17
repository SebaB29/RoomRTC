//! RTP packet structure and serialization
//!
//! This module implements RFC 3550 (RTP: A Transport Protocol for Real-Time Applications).
//! It provides structures for RTP headers and packets with serialization/deserialization
//! capabilities.
//!
//! # RTP Header Format (RFC 3550 Section 5.1)
//!
//! ```text
//! 0                   1                   2                   3
//! 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |V=2|P|X|  CC   |M|     PT      |       Sequence Number         |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                           Timestamp                           |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |           SSRC (Synchronization Source)                       |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```
//!
//! # Example
//!
//! ```rust,ignore
//! use network::codec::rtp::{RtpHeader, RtpPacket};
//!
//! // Create RTP header
//! let mut header = RtpHeader::new(96, 12345);
//! header.sequence_number = 100;
//! header.timestamp = 90000;
//! header.marker = true;
//!
//! // Create packet
//! let payload = vec![1, 2, 3, 4, 5];
//! let packet = RtpPacket::new(header, payload);
//!
//! // Serialize
//! let bytes = packet.to_bytes();
//!
//! // Deserialize
//! let decoded = RtpPacket::from_bytes(&bytes)?;
//! ```

use crate::error::{NetworkError, Result};

/// Helper functions to eliminate repetitive byte parsing
pub(crate) fn parse_u16_be(data: &[u8], offset: usize) -> u16 {
    u16::from_be_bytes([data[offset], data[offset + 1]])
}

pub(crate) fn parse_u32_be(data: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// RTP payload types for application control messages
pub mod control_payload {
    /// Payload type for control messages (uses unused payload type range)
    pub const CONTROL: u8 = 127;

    /// Control message types
    pub const CAMERA_OFF: u8 = 1;
    pub const CAMERA_ON: u8 = 2;
    pub const PARTICIPANT_DISCONNECTED: u8 = 3;
    pub const PARTICIPANT_NAME: u8 = 4;
    pub const OWNER_DISCONNECTED: u8 = 5;
    pub const AUDIO_ON: u8 = 6;
    pub const AUDIO_OFF: u8 = 7;
    pub const AUDIO_MUTED: u8 = 8;
    pub const AUDIO_UNMUTED: u8 = 9;
}

/// RTP packet header according to RFC 3550.
#[derive(Debug, Clone)]
pub struct RtpHeader {
    /// RTP version (always 2)
    pub version: u8,
    /// Padding flag
    pub padding: bool,
    /// Extension flag
    pub extension: bool,
    /// CSRC count (0-15)
    pub csrc_count: u8,
    /// Marker bit (interpretation depends on payload type)
    pub marker: bool,
    /// Payload type (0-127)
    pub payload_type: u8,
    /// Sequence number (increments by 1 for each packet)
    pub sequence_number: u16,
    /// Timestamp (90kHz clock for video)
    pub timestamp: u32,
    /// Synchronization source identifier
    pub ssrc: u32,
}

impl RtpHeader {
    /// Fixed size of RTP header in bytes.
    const HEADER_SIZE: usize = 12;

    /// Creates a new RTP header with default values.
    ///
    /// # Arguments
    /// * `payload_type` - RTP payload type (e.g., 96 for dynamic H.264)
    /// * `ssrc` - Synchronization source identifier
    ///
    /// # Returns
    /// A new `RtpHeader` with version 2, no padding/extension, and zero sequence/timestamp.
    pub fn new(payload_type: u8, ssrc: u32) -> Self {
        RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type,
            sequence_number: 0,
            timestamp: 0,
            ssrc,
        }
    }

    /// Serializes the RTP header to bytes.
    ///
    /// Converts the header structure to a 12-byte array following RFC 3550 format.
    ///
    /// # Returns
    /// A `Vec<u8>` containing the serialized header (12 bytes).
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(Self::HEADER_SIZE);

        // Byte 0: V(2) + P(1) + X(1) + CC(4)
        let byte0 = (self.version << 6)
            | ((self.padding as u8) << 5)
            | ((self.extension as u8) << 4)
            | self.csrc_count;
        bytes.push(byte0);

        // Byte 1: M(1) + PT(7)
        let byte1 = ((self.marker as u8) << 7) | self.payload_type;
        bytes.push(byte1);

        // Bytes 2-3: Sequence number
        bytes.extend_from_slice(&self.sequence_number.to_be_bytes());

        // Bytes 4-7: Timestamp
        bytes.extend_from_slice(&self.timestamp.to_be_bytes());

        // Bytes 8-11: SSRC
        bytes.extend_from_slice(&self.ssrc.to_be_bytes());

        bytes
    }

    /// Deserialization header from bytes
    ///
    /// # Arguments
    /// * `data` - Byte slice containing the serialized RTP header
    ///
    /// # Returns
    /// A `Result` containing the deserialized `RtpHeader` or an error
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        if data.len() < Self::HEADER_SIZE {
            return Err(NetworkError::Rtp("Header too short".to_string()));
        }

        let version = (data[0] >> 6) & 0x03;
        let padding = ((data[0] >> 5) & 0x01) == 1;
        let extension = ((data[0] >> 4) & 0x01) == 1;
        let csrc_count = data[0] & 0x0F;

        let marker = ((data[1] >> 7) & 0x01) == 1;
        let payload_type = data[1] & 0x7F;

        let sequence_number = parse_u16_be(data, 2);
        let timestamp = parse_u32_be(data, 4);
        let ssrc = parse_u32_be(data, 8);

        Ok(RtpHeader {
            version,
            padding,
            extension,
            csrc_count,
            marker,
            payload_type,
            sequence_number,
            timestamp,
            ssrc,
        })
    }
}

/// Complete RTP Packet
#[derive(Debug, Clone)]
pub struct RtpPacket {
    pub header: RtpHeader,
    pub payload: Vec<u8>,
}

impl RtpPacket {
    /// Create new RTP Packet
    pub fn new(header: RtpHeader, payload: Vec<u8>) -> Self {
        RtpPacket { header, payload }
    }

    /// Serialization Packet to Bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.header.to_bytes();
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    /// Deserialization Packet from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self> {
        let header = RtpHeader::from_bytes(data)?;
        let payload = data[RtpHeader::HEADER_SIZE..].to_vec();
        Ok(RtpPacket { header, payload })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rtp_header_serialization() {
        let header = RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: true,
            payload_type: 96,
            sequence_number: 1234,
            timestamp: 5678,
            ssrc: 9999,
        };

        let bytes = header.to_bytes();
        let decoded = RtpHeader::from_bytes(&bytes).unwrap();

        assert_eq!(header.version, decoded.version);
        assert_eq!(header.marker, decoded.marker);
        assert_eq!(header.payload_type, decoded.payload_type);
        assert_eq!(header.sequence_number, decoded.sequence_number);
        assert_eq!(header.timestamp, decoded.timestamp);
        assert_eq!(header.ssrc, decoded.ssrc);
    }

    #[test]
    fn test_rtp_header_new() {
        let header = RtpHeader::new(96, 12345);

        assert_eq!(header.version, 2);
        assert_eq!(header.payload_type, 96);
        assert_eq!(header.ssrc, 12345);
        assert_eq!(header.sequence_number, 0);
        assert_eq!(header.timestamp, 0);
        assert!(!header.padding);
        assert!(!header.extension);
        assert!(!header.marker);
        assert_eq!(header.csrc_count, 0);
    }

    #[test]
    fn test_rtp_header_size() {
        let header = RtpHeader::new(96, 1000);
        let bytes = header.to_bytes();
        assert_eq!(bytes.len(), 12);
    }

    #[test]
    fn test_rtp_header_from_bytes_short() {
        let short_data = vec![0x80, 0x60, 0x12, 0x34];
        let result = RtpHeader::from_bytes(&short_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_rtp_header_version() {
        let header = RtpHeader::new(96, 1000);
        let bytes = header.to_bytes();

        // Version is in top 2 bits of first byte
        assert_eq!((bytes[0] >> 6) & 0x03, 2);
    }

    #[test]
    fn test_rtp_header_marker_bit() {
        let mut header = RtpHeader::new(96, 1000);
        header.marker = true;
        let bytes = header.to_bytes();

        // Marker is top bit of second byte
        assert_eq!((bytes[1] >> 7) & 0x01, 1);
    }

    #[test]
    fn test_rtp_header_payload_type() {
        let header = RtpHeader::new(127, 1000);
        let bytes = header.to_bytes();

        // Payload type is lower 7 bits of second byte
        assert_eq!(bytes[1] & 0x7F, 127);
    }

    #[test]
    fn test_rtp_header_with_padding() {
        let mut header = RtpHeader::new(96, 1000);
        header.padding = true;

        let bytes = header.to_bytes();
        let decoded = RtpHeader::from_bytes(&bytes).unwrap();

        assert!(decoded.padding);
    }

    #[test]
    fn test_rtp_header_with_extension() {
        let mut header = RtpHeader::new(96, 1000);
        header.extension = true;

        let bytes = header.to_bytes();
        let decoded = RtpHeader::from_bytes(&bytes).unwrap();

        assert!(decoded.extension);
    }

    #[test]
    fn test_rtp_header_csrc_count() {
        let mut header = RtpHeader::new(96, 1000);
        header.csrc_count = 15; // Max value

        let bytes = header.to_bytes();
        let decoded = RtpHeader::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.csrc_count, 15);
    }

    #[test]
    fn test_rtp_packet_new() {
        let header = RtpHeader::new(96, 1000);
        let payload = vec![1, 2, 3, 4, 5];
        let packet = RtpPacket::new(header.clone(), payload.clone());

        assert_eq!(packet.header.ssrc, 1000);
        assert_eq!(packet.payload, payload);
    }

    #[test]
    fn test_rtp_packet_to_bytes() {
        let header = RtpHeader::new(96, 1000);
        let payload = vec![10, 20, 30];
        let packet = RtpPacket::new(header, payload.clone());

        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), 12 + payload.len());

        // Check payload is at the end
        assert_eq!(&bytes[12..], &payload[..]);
    }

    #[test]
    fn test_rtp_packet_from_bytes() {
        let header = RtpHeader::new(96, 5555);
        let payload = vec![100, 101, 102];
        let packet = RtpPacket::new(header, payload.clone());

        let bytes = packet.to_bytes();
        let decoded = RtpPacket::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.header.ssrc, 5555);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_rtp_packet_roundtrip() {
        let mut header = RtpHeader::new(96, 12345);
        header.sequence_number = 999;
        header.timestamp = 90000;
        header.marker = true;

        let payload = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        let packet = RtpPacket::new(header, payload.clone());

        let bytes = packet.to_bytes();
        let decoded = RtpPacket::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.header.payload_type, 96);
        assert_eq!(decoded.header.ssrc, 12345);
        assert_eq!(decoded.header.sequence_number, 999);
        assert_eq!(decoded.header.timestamp, 90000);
        assert!(decoded.header.marker);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_rtp_packet_empty_payload() {
        let header = RtpHeader::new(96, 1000);
        let packet = RtpPacket::new(header, vec![]);

        let bytes = packet.to_bytes();
        assert_eq!(bytes.len(), 12); // Only header

        let decoded = RtpPacket::from_bytes(&bytes).unwrap();
        assert!(decoded.payload.is_empty());
    }

    #[test]
    fn test_rtp_packet_large_payload() {
        let header = RtpHeader::new(96, 1000);
        let payload = vec![0xFF; 1400]; // Large payload
        let packet = RtpPacket::new(header, payload.clone());

        let bytes = packet.to_bytes();
        let decoded = RtpPacket::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.payload.len(), 1400);
        assert_eq!(decoded.payload, payload);
    }

    #[test]
    fn test_sequence_number_wrap() {
        let mut header = RtpHeader::new(96, 1000);
        header.sequence_number = 65535; // Max u16

        let bytes = header.to_bytes();
        let decoded = RtpHeader::from_bytes(&bytes).unwrap();

        assert_eq!(decoded.sequence_number, 65535);
    }

    #[test]
    fn test_timestamp_values() {
        let test_timestamps = vec![0, 1, 90000, 180000, u32::MAX];

        for ts in test_timestamps {
            let mut header = RtpHeader::new(96, 1000);
            header.timestamp = ts;

            let bytes = header.to_bytes();
            let decoded = RtpHeader::from_bytes(&bytes).unwrap();

            assert_eq!(decoded.timestamp, ts);
        }
    }
}
