//! H.264 RTP Packetizer Implementation
//!
//! Implements RFC 6184 - RTP Payload Format for H.264 Video.
//!
//! # Packetization Modes
//! This implementation supports two packetization modes:
//!
//! ## Single NAL Unit Mode (Section 5.6)
//! Used when a complete NAL unit fits within the MTU. The NAL unit is sent
//! as-is in the RTP payload without modification.
//!
//! ```text
//! RTP Packet:
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  RTP Header (12 bytes)        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  NAL Unit (complete)          |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```
//!
//! ## FU-A Fragmentation Mode (Section 5.8)
//! Used when a NAL unit exceeds the MTU and must be fragmented. Each fragment
//! contains a FU indicator and FU header for reassembly.
//!
//! ```text
//! FU-A Packet:
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  RTP Header (12 bytes)        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  FU Indicator (1 byte)        |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  FU Header (1 byte)           |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |  Fragment Payload             |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```
//!
//! FU Indicator (1 byte):
//! ```text
//!  0 1 2 3 4 5 6 7
//! +-+-+-+-+-+-+-+-+
//! |F|NRI|  Type   |
//! +-+-+-+-+-+-+-+-+
//! ```
//! - F: Forbidden zero bit (copied from NAL header)
//! - NRI: NAL reference indicator (copied from NAL header)
//! - Type: 28 for FU-A
//!
//! FU Header (1 byte):
//! ```text
//!  0 1 2 3 4 5 6 7
//! +-+-+-+-+-+-+-+-+
//! |S|E|R|  Type   |
//! +-+-+-+-+-+-+-+-+
//! ```
//! - S: Start bit (1 for first fragment)
//! - E: End bit (1 for last fragment)
//! - R: Reserved (must be 0)
//! - Type: NAL unit type from original NAL header

use crate::codec::rtp::{RtpHeader, RtpPacket};
use crate::traits::RtpPacketizer;
use rand::Rng;

/// FU-A fragmentation unit type (RFC 6184 Section 5.8)
const FU_A_TYPE: u8 = 28;
/// H.264 NAL unit start codes (Annex B format)
const NAL_START_CODE_4: &[u8] = &[0x00, 0x00, 0x00, 0x01];
const NAL_START_CODE_3: &[u8] = &[0x00, 0x00, 0x01];

/// Represents an H.264 RTP packetizer
///
/// Handles both Single NAL Unit and FU-A Fragmentation modes
/// according to RFC 6184 specification.
pub struct H264RtpPacketizer {
    /// Synchronization source identifier (randomly generated)
    ssrc: u32,
    /// RTP sequence number (incremented per packet)
    sequence_number: u16,
    /// RTP timestamp in 90 kHz clock units
    timestamp: u32,
    /// RTP payload type (typically 96 for dynamic H.264)
    payload_type: u8,
    /// Maximum RTP payload size in bytes (MTU - RTP header)
    max_payload_size: usize,
    /// Timestamp increment per frame (90000 / fps)
    timestamp_increment: u32,
}

impl H264RtpPacketizer {
    /// Create a new H.264 RTP packetizer
    ///
    /// # Arguments
    /// * `payload_type` - RTP payload type (96-127 for dynamic mappings)
    /// * `max_payload_size` - Maximum payload size in bytes (typically MTU - 40 for IP/UDP/RTP headers)
    /// * `fps` - Video frame rate for timestamp calculation
    ///
    /// # Returns
    /// New packetizer instance with randomly generated SSRC and sequence number
    pub fn new(payload_type: u8, max_payload_size: usize, fps: f64) -> Self {
        let mut rng = rand::thread_rng();

        // Calculate timestamp increment for 90 kHz RTP clock
        // Example: 30 fps -> 90000 / 30 = 3000 units per frame
        let timestamp_increment = (90000.0 / fps).round() as u32;

        H264RtpPacketizer {
            ssrc: rng.gen_range(0..=u32::MAX),
            sequence_number: rng.gen_range(0..=u16::MAX),
            timestamp: 0,
            payload_type,
            max_payload_size,
            timestamp_increment,
        }
    }

    /// Extract NAL units from H.264 Annex B byte stream
    ///
    /// Searches for NAL unit start codes (0x000001 or 0x00000001) and
    /// extracts individual NAL units. If no start codes are found,
    /// treats the entire input as a single NAL unit.
    ///
    /// # Arguments
    /// * `data` - H.264 byte stream with start codes
    ///
    /// # Returns
    /// Vector of NAL units (without start codes)
    ///
    /// # NAL Unit Structure
    /// Each NAL unit consists of:
    /// - NAL header (1 byte): F(1) + NRI(2) + Type(5)
    /// - NAL payload (variable length)
    fn extract_nal_units(&self, data: &[u8]) -> Vec<Vec<u8>> {
        let mut nal_units = Vec::new();
        let mut start = 0;

        // Search for all start codes (0x00000001 or 0x000001)
        let mut i = 0;
        while i < data.len() {
            // Check for 3-byte start code
            if i + 3 <= data.len() && &data[i..i + 3] == NAL_START_CODE_3 {
                if start > 0 {
                    nal_units.push(data[start..i].to_vec());
                }
                start = i + 3;
                i += 3;
            }
            // Check for 4-byte start code
            else if i + 4 <= data.len() && &data[i..i + 4] == NAL_START_CODE_4 {
                if start > 0 {
                    nal_units.push(data[start..i].to_vec());
                }
                start = i + 4;
                i += 4;
            } else {
                i += 1;
            }
        }

        // Add last NAL unit
        if start < data.len() {
            nal_units.push(data[start..].to_vec());
        }

        // If no start codes found, treat entire input as single NAL unit
        if nal_units.is_empty() && !data.is_empty() {
            nal_units.push(data.to_vec());
        }

        nal_units
    }

    /// Packetize a single NAL unit that fits within MTU
    ///
    /// Implements Single NAL Unit Mode (RFC 6184 Section 5.6).
    /// The NAL unit is sent as-is without fragmentation.
    ///
    /// # Arguments
    /// * `nal_unit` - Complete NAL unit (with header)
    /// * `is_last` - Whether this is the last NAL unit in the frame
    ///
    /// # Returns
    /// Single RTP packet containing the complete NAL unit
    fn packetize_single_nal(&mut self, nal_unit: &[u8], is_last: bool) -> Vec<RtpPacket> {
        let mut header = RtpHeader::new(self.payload_type, self.ssrc);
        header.sequence_number = self.sequence_number;
        header.timestamp = self.timestamp;
        header.marker = is_last; // Mark last NAL unit of frame

        self.sequence_number = self.sequence_number.wrapping_add(1);

        vec![RtpPacket::new(header, nal_unit.to_vec())]
    }

    /// Packetize a large NAL unit using FU-A fragmentation
    ///
    /// Implements FU-A Fragmentation Mode (RFC 6184 Section 5.8).
    /// Splits the NAL unit into multiple fragments, each with FU indicator
    /// and FU header for proper reassembly.
    ///
    /// # Arguments
    /// * `nal_unit` - NAL unit to fragment (must include NAL header)
    /// * `is_last_nal` - Whether this is the last NAL unit in the frame
    ///
    /// # Returns
    /// Vector of RTP packets containing fragmented NAL unit
    ///
    /// # FU-A Structure
    /// Each fragment packet contains:
    /// - FU Indicator (1 byte): Preserves NRI from original NAL header
    /// - FU Header (1 byte): Start/End flags + original NAL type
    /// - Fragment payload (max_payload_size - 2 bytes)
    fn packetize_fu_a(&mut self, nal_unit: &[u8], is_last_nal: bool) -> Vec<RtpPacket> {
        if nal_unit.is_empty() {
            return Vec::new();
        }

        let (nal_type, nri) = parse_nal_header(nal_unit[0]);
        let fu_indicator = (nri << 5) | FU_A_TYPE;
        let payload = &nal_unit[1..];
        let fragment_size = self.max_payload_size - 2;

        self.create_fu_a_packets(payload, fu_indicator, nal_type, is_last_nal, fragment_size)
    }

    fn create_fu_a_packets(
        &mut self,
        payload: &[u8],
        fu_indicator: u8,
        nal_type: u8,
        is_last_nal: bool,
        fragment_size: usize,
    ) -> Vec<RtpPacket> {
        let fragments: Vec<&[u8]> = payload.chunks(fragment_size).collect();
        let mut packets = Vec::with_capacity(fragments.len());

        for (i, fragment) in fragments.iter().enumerate() {
            let is_first = i == 0;
            let is_last_fragment = i == fragments.len() - 1;
            let fu_header = build_fu_header(nal_type, is_first, is_last_fragment);
            let rtp_payload = build_fu_a_payload(fu_indicator, fu_header, fragment);

            let packet = self.create_rtp_packet(rtp_payload, is_last_nal && is_last_fragment);
            packets.push(packet);
        }

        packets
    }

    fn create_rtp_packet(&mut self, payload: Vec<u8>, marker: bool) -> RtpPacket {
        let mut header = RtpHeader::new(self.payload_type, self.ssrc);
        header.sequence_number = self.sequence_number;
        header.timestamp = self.timestamp;
        header.marker = marker;

        self.sequence_number = self.sequence_number.wrapping_add(1);
        RtpPacket::new(header, payload)
    }
}

fn parse_nal_header(nal_header: u8) -> (u8, u8) {
    let nal_type = nal_header & 0x1F;
    let nri = (nal_header >> 5) & 0x03;
    (nal_type, nri)
}

fn build_fu_header(nal_type: u8, is_first: bool, is_last: bool) -> u8 {
    let mut fu_header = nal_type;
    if is_first {
        fu_header |= 0x80;
    }
    if is_last {
        fu_header |= 0x40;
    }
    fu_header
}

fn build_fu_a_payload(fu_indicator: u8, fu_header: u8, fragment: &[u8]) -> Vec<u8> {
    let mut payload = Vec::with_capacity(fragment.len() + 2);
    payload.push(fu_indicator);
    payload.push(fu_header);
    payload.extend_from_slice(fragment);
    payload
}

impl RtpPacketizer for H264RtpPacketizer {
    fn packetize(&mut self, data: &[u8]) -> Vec<RtpPacket> {
        let mut packets = Vec::new();

        // Extract NAL units from H.264 stream
        let nal_units = self.extract_nal_units(data);

        for (i, nal_unit) in nal_units.iter().enumerate() {
            let is_last_nal = i == nal_units.len() - 1;

            if nal_unit.len() <= self.max_payload_size {
                // Single NAL Unit Mode (fits in one packet)
                packets.extend(self.packetize_single_nal(nal_unit, is_last_nal));
            } else {
                // FU-A Fragmentation Mode (requires multiple packets)
                packets.extend(self.packetize_fu_a(nal_unit, is_last_nal));
            }
        }

        // Increment timestamp for next frame (90 kHz clock)
        self.timestamp = self.timestamp.wrapping_add(self.timestamp_increment);

        packets
    }

    fn get_payload_type(&self) -> u8 {
        self.payload_type
    }

    fn get_ssrc(&self) -> u32 {
        self.ssrc
    }

    fn get_timestamp(&self) -> u32 {
        self.timestamp
    }

    fn get_sequence_number(&self) -> u16 {
        self.sequence_number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_nal_unit_mode() {
        let mut packetizer = H264RtpPacketizer::new(96, 1000, 30.0);

        // Small NAL unit that fits in single packet
        let mut nal_data = Vec::new();
        nal_data.extend_from_slice(NAL_START_CODE_4);
        nal_data.push(0x67); // SPS NAL header
        nal_data.extend(vec![0x01, 0x02, 0x03]);

        let packets = packetizer.packetize(&nal_data);

        assert_eq!(packets.len(), 1);
        assert!(packets[0].header.marker); // Last NAL unit
        assert_eq!(packets[0].payload[0] & 0x1F, 0x07); // NAL type 7 (SPS)
    }

    #[test]
    fn test_fu_a_fragmentation() {
        let mut packetizer = H264RtpPacketizer::new(96, 1000, 30.0);

        // Large NAL unit requiring fragmentation
        let mut large_nal = Vec::new();
        large_nal.extend_from_slice(NAL_START_CODE_4);
        large_nal.push(0x65); // IDR slice NAL header (type 5, NRI=3)
        large_nal.extend(vec![0xAA; 2000]); // 2000 bytes payload

        let packets = packetizer.packetize(&large_nal);

        // Should create multiple FU-A packets
        assert!(packets.len() > 1);

        // First packet: Check FU-A indicator and start bit
        assert_eq!(packets[0].payload[0] & 0x1F, FU_A_TYPE); // FU-A type
        assert_eq!((packets[0].payload[0] >> 5) & 0x03, 3); // NRI preserved
        assert!(packets[0].payload[1] & 0x80 != 0); // Start bit set
        assert_eq!(packets[0].payload[1] & 0x1F, 0x05); // Original NAL type

        // Last packet: Check end bit and marker
        let last = packets.last().unwrap();
        assert!(last.payload[1] & 0x40 != 0); // End bit set
        assert!(last.header.marker); // Marker bit set
    }

    #[test]
    fn test_multiple_nal_units() {
        let mut packetizer = H264RtpPacketizer::new(96, 1000, 30.0);

        // Multiple small NAL units (SPS + PPS + IDR)
        let mut data = Vec::new();

        // SPS
        data.extend_from_slice(NAL_START_CODE_4);
        data.push(0x67);
        data.extend(vec![0x01; 10]);

        // PPS
        data.extend_from_slice(NAL_START_CODE_3);
        data.push(0x68);
        data.extend(vec![0x02; 10]);

        // IDR slice
        data.extend_from_slice(NAL_START_CODE_4);
        data.push(0x65);
        data.extend(vec![0x03; 10]);

        let packets = packetizer.packetize(&data);

        assert_eq!(packets.len(), 3); // Three NAL units
        assert!(!packets[0].header.marker); // Not last
        assert!(!packets[1].header.marker); // Not last
        assert!(packets[2].header.marker); // Last NAL unit
    }

    #[test]
    fn test_timestamp_increment() {
        let mut packetizer = H264RtpPacketizer::new(96, 1000, 30.0);

        let initial_timestamp = packetizer.get_timestamp();

        // Packetize first frame
        let frame1 = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x01, 0x02];
        packetizer.packetize(&frame1);

        let timestamp_after_frame1 = packetizer.get_timestamp();
        assert_eq!(timestamp_after_frame1 - initial_timestamp, 3000); // 90000 / 30 fps

        // Packetize second frame
        let frame2 = vec![0x00, 0x00, 0x00, 0x01, 0x65, 0x03, 0x04];
        packetizer.packetize(&frame2);

        let timestamp_after_frame2 = packetizer.get_timestamp();
        assert_eq!(timestamp_after_frame2 - timestamp_after_frame1, 3000);
    }
}
