//! H.264 RTP Depacketizer Implementation
//!
//! Reconstructs H.264 NAL units from RTP packets according to RFC 6184.
//!
//! # Depacketization Modes
//! This implementation handles both packetization modes:
//!
//! ## Single NAL Unit Mode
//! Complete NAL units received in a single RTP packet are returned immediately
//! with an Annex B start code prepended.
//!
//! ## FU-A Fragmentation Mode
//! Fragmented NAL units are reassembled across multiple RTP packets:
//! 1. First fragment (S bit set): Initialize buffer with start code + NAL header
//! 2. Middle fragments: Append payload to buffer
//! 3. Last fragment (E bit set): Return complete NAL unit
//!
//! # Packet Loss Handling
//! - If a fragment is lost mid-frame, the next start fragment discards incomplete data
//! - Timestamp changes also trigger buffer reset
//! - Out-of-order packets are NOT reordered (assumes ordered transport or external reordering)
//!
//! # Examples
//! ```rust,ignore
//! use network::{H264RtpDepacketizer, RtpDepacketizer, RtpPacket};
//!
//! let mut depacketizer = H264RtpDepacketizer::new();
//!
//! // Process incoming RTP packets
//! for packet_data in udp_transport.receive() {
//!     let packet = RtpPacket::from_bytes(&packet_data)?;
//!     
//!     if let Some(nal_unit) = depacketizer.process_packet(&packet) {
//!         // Complete NAL unit ready for decoder
//!         // nal_unit starts with 0x00000001 (Annex B start code)
//!         h264_decoder.decode(&nal_unit)?;
//!     }
//! }
//! ```

use crate::codec::rtp::RtpPacket;
use crate::traits::RtpDepacketizer;

/// FU-A fragmentation unit type (RFC 6184 Section 5.8)
const FU_A_TYPE: u8 = 28;
/// H.264 NAL unit start code (Annex B format)
const NAL_START_CODE: &[u8] = &[0x00, 0x00, 0x00, 0x01];

/// Represents an H.264 RTP depacketizer
///
/// Handles both Single NAL Unit and FU-A Fragmentation modes
/// according to RFC 6184 specification.
pub struct H264RtpDepacketizer {
    /// Current RTP timestamp for tracking frame boundaries
    current_timestamp: Option<u32>,
    /// Buffer for reassembling fragmented NAL units
    nal_buffer: Vec<u8>,
}

impl H264RtpDepacketizer {
    /// Create a new H.264 RTP depacketizer
    ///
    /// # Returns
    /// New depacketizer instance with empty buffers
    pub fn new() -> Self {
        H264RtpDepacketizer {
            current_timestamp: None,
            nal_buffer: Vec::new(),
        }
    }

    /// Process a single NAL unit packet
    ///
    /// Handles packets where the entire NAL unit fits in one RTP packet.
    /// Prepends Annex B start code and returns immediately.
    ///
    /// # Arguments
    /// * `payload` - RTP payload containing complete NAL unit
    /// * `timestamp` - RTP timestamp
    ///
    /// # Returns
    /// Complete NAL unit with start code prepended
    fn process_single_nal(&mut self, payload: &[u8], timestamp: u32) -> Vec<u8> {
        let mut complete_nal = Vec::with_capacity(NAL_START_CODE.len() + payload.len());
        complete_nal.extend_from_slice(NAL_START_CODE);
        complete_nal.extend_from_slice(payload);
        self.current_timestamp = Some(timestamp);
        complete_nal
    }

    /// Process a FU-A fragmentation unit
    ///
    /// Handles fragmented NAL units across multiple RTP packets.
    /// Reassembles fragments and returns complete NAL unit when last fragment arrives.
    ///
    /// # Arguments
    /// * `payload` - RTP payload containing FU-A fragment
    /// * `timestamp` - RTP timestamp
    ///
    /// # Returns
    /// - `Some(Vec<u8>)` - Complete NAL unit when last fragment (E bit) arrives
    /// - `None` - Waiting for more fragments
    ///
    /// # FU-A Packet Structure
    /// ```text
    /// payload[0]: FU Indicator
    /// payload[1]: FU Header
    /// payload[2..]: Fragment data
    /// ```
    fn process_fu_a(&mut self, payload: &[u8], timestamp: u32) -> Option<Vec<u8>> {
        if payload.len() < 2 {
            return None;
        }

        let fu_indicator = payload[0];
        let fu_header = payload[1];
        let (is_start, is_end, nal_type) = parse_fu_header(fu_header);

        if is_start {
            self.start_new_fragment(timestamp, fu_indicator, nal_type);
        }

        self.nal_buffer.extend_from_slice(&payload[2..]);

        if is_end {
            Some(self.nal_buffer.clone())
        } else {
            None
        }
    }

    fn start_new_fragment(&mut self, timestamp: u32, fu_indicator: u8, nal_type: u8) {
        self.current_timestamp = Some(timestamp);
        self.nal_buffer.clear();
        self.nal_buffer.extend_from_slice(NAL_START_CODE);

        let nri = (fu_indicator >> 5) & 0x03;
        let nal_header = (nri << 5) | nal_type;
        self.nal_buffer.push(nal_header);
    }
}

fn parse_fu_header(fu_header: u8) -> (bool, bool, u8) {
    let is_start = (fu_header & 0x80) != 0;
    let is_end = (fu_header & 0x40) != 0;
    let nal_type = fu_header & 0x1F;
    (is_start, is_end, nal_type)
}

impl RtpDepacketizer for H264RtpDepacketizer {
    fn process_packet(&mut self, packet: &RtpPacket) -> Option<Vec<u8>> {
        let timestamp = packet.header.timestamp;
        let payload = &packet.payload;

        if payload.is_empty() {
            return None;
        }

        // Detect timestamp change (indicates new frame or packet loss recovery)
        if let Some(current_ts) = self.current_timestamp
            && timestamp != current_ts
            && !self.nal_buffer.is_empty()
        {
            // Timestamp changed with incomplete buffer - discard stale data
            self.nal_buffer.clear();
        }

        // Check NAL unit type from first byte
        let nal_type = payload[0] & 0x1F;

        if nal_type == FU_A_TYPE {
            // FU-A Fragmentation Mode
            self.process_fu_a(payload, timestamp)
        } else {
            // Single NAL Unit Mode
            Some(self.process_single_nal(payload, timestamp))
        }
    }

    fn reset(&mut self) {
        self.current_timestamp = None;
        self.nal_buffer.clear();
    }

    fn has_pending_data(&self) -> bool {
        !self.nal_buffer.is_empty()
    }
}

impl Default for H264RtpDepacketizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::rtp::RtpHeader;

    #[test]
    fn test_single_nal_unit() {
        let mut depacketizer = H264RtpDepacketizer::new();

        // Create packet with single NAL unit (SPS)
        let mut header = RtpHeader::new(96, 12345);
        header.timestamp = 1000;
        header.marker = true;

        let nal_data = vec![0x67, 0x01, 0x02, 0x03]; // SPS NAL unit
        let packet = RtpPacket::new(header, nal_data);

        let result = depacketizer.process_packet(&packet);
        assert!(result.is_some());

        let nal = result.unwrap();

        // Verify start code prepended
        assert_eq!(&nal[0..4], NAL_START_CODE);

        // Verify NAL data
        assert_eq!(&nal[4..], &[0x67, 0x01, 0x02, 0x03]);
    }

    #[test]
    fn test_fu_a_fragmentation() {
        let mut depacketizer = H264RtpDepacketizer::new();

        let timestamp = 2000;
        let ssrc = 54321;
        let payload_type = 96;

        // Simulate FU-A fragmented IDR slice (NAL type 5, NRI 3)
        let original_nal_type = 0x05;
        let nri = 3;
        let fu_indicator = (nri << 5) | FU_A_TYPE;

        // First fragment (Start bit set)
        let mut header1 = RtpHeader::new(payload_type, ssrc);
        header1.timestamp = timestamp;
        header1.sequence_number = 100;

        let mut payload1 = Vec::new();
        payload1.push(fu_indicator);
        payload1.push(0x80 | original_nal_type); // Start bit + NAL type
        payload1.extend(vec![0xAA; 100]); // Fragment data

        let packet1 = RtpPacket::new(header1, payload1);
        let result1 = depacketizer.process_packet(&packet1);
        assert!(result1.is_none()); // Not complete yet

        // Middle fragment
        let mut header2 = RtpHeader::new(payload_type, ssrc);
        header2.timestamp = timestamp;
        header2.sequence_number = 101;

        let mut payload2 = Vec::new();
        payload2.push(fu_indicator);
        payload2.push(original_nal_type); // No Start/End bits
        payload2.extend(vec![0xBB; 100]);

        let packet2 = RtpPacket::new(header2, payload2);
        let result2 = depacketizer.process_packet(&packet2);
        assert!(result2.is_none()); // Still not complete

        // Last fragment (End bit set)
        let mut header3 = RtpHeader::new(payload_type, ssrc);
        header3.timestamp = timestamp;
        header3.sequence_number = 102;
        header3.marker = true;

        let mut payload3 = Vec::new();
        payload3.push(fu_indicator);
        payload3.push(0x40 | original_nal_type); // End bit + NAL type
        payload3.extend(vec![0xCC; 100]);

        let packet3 = RtpPacket::new(header3, payload3);
        let result3 = depacketizer.process_packet(&packet3);
        assert!(result3.is_some()); // Complete!

        let nal = result3.unwrap();

        // Verify start code
        assert_eq!(&nal[0..4], NAL_START_CODE);

        // Verify reconstructed NAL header
        let reconstructed_nal_header = (nri << 5) | original_nal_type;
        assert_eq!(nal[4], reconstructed_nal_header);

        // Verify payload size (3 fragments × 100 bytes + 1 NAL header + 4 start code)
        assert_eq!(nal.len(), 4 + 1 + 300);
    }

    #[test]
    fn test_timestamp_change_discards_incomplete_data() {
        let mut depacketizer = H264RtpDepacketizer::new();

        // Start FU-A fragment with timestamp 1000
        let mut header1 = RtpHeader::new(96, 12345);
        header1.timestamp = 1000;

        let payload1 = vec![
            (3 << 5) | FU_A_TYPE, // FU indicator
            0x80 | 0x05,          // Start bit + NAL type 5
            0xAA,
            0xBB, // Fragment data
        ];

        let packet1 = RtpPacket::new(header1, payload1);
        depacketizer.process_packet(&packet1);

        assert!(depacketizer.has_pending_data()); // Buffer has data

        // New packet with different timestamp (simulates packet loss or new frame)
        let mut header2 = RtpHeader::new(96, 12345);
        header2.timestamp = 2000; // Different timestamp
        header2.marker = true;

        let payload2 = vec![0x67, 0x01, 0x02]; // New single NAL unit
        let packet2 = RtpPacket::new(header2, payload2);

        let result = depacketizer.process_packet(&packet2);
        assert!(result.is_some()); // New NAL unit returned

        // Old incomplete data should be discarded
        let nal = result.unwrap();
        assert_eq!(&nal[4..], &[0x67, 0x01, 0x02]); // New NAL data
    }

    #[test]
    fn test_reset() {
        let mut depacketizer = H264RtpDepacketizer::new();

        // Add some data
        let mut header = RtpHeader::new(96, 12345);
        header.timestamp = 1000;

        let payload = vec![(3 << 5) | FU_A_TYPE, 0x80 | 0x05, 0xAA];

        let packet = RtpPacket::new(header, payload);
        depacketizer.process_packet(&packet);

        assert!(depacketizer.has_pending_data());

        // Reset clears everything
        depacketizer.reset();

        assert!(!depacketizer.has_pending_data());
        assert!(depacketizer.current_timestamp.is_none());
    }
}
