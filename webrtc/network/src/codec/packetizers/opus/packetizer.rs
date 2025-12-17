//! Opus RTP Packetizer Implementation
//!
//! Implements RFC 7587 - RTP Payload Format for Opus Audio Codec.
//!
//! Opus packets typically fit within a single RTP packet (< MTU),
//! so fragmentation is rarely needed.

use crate::codec::rtp::{RtpHeader, RtpPacket};
use crate::traits::RtpPacketizer;
use rand::Rng;

/// Opus RTP packetizer
///
/// Handles packetization of Opus audio frames into RTP packets.
/// Following RFC 7587 specification.
pub struct OpusRtpPacketizer {
    /// Synchronization source identifier
    ssrc: u32,
    /// RTP sequence number
    sequence_number: u16,
    /// RTP timestamp (in 48 kHz units for Opus)
    timestamp: u32,
    /// RTP payload type (typically 111 for Opus)
    payload_type: u8,
    /// Maximum payload size
    max_payload_size: usize,
    /// Timestamp increment per frame (based on sample rate and frame size)
    /// For 48kHz with 20ms frames: 48000 * 0.02 = 960 samples
    timestamp_increment: u32,
}

impl OpusRtpPacketizer {
    /// Create a new Opus RTP packetizer
    ///
    /// # Arguments
    /// * `payload_type` - RTP payload type (typically 111 for Opus)
    /// * `max_payload_size` - Maximum payload size in bytes
    /// * `sample_rate` - Audio sample rate (typically 48000 Hz for Opus)
    /// * `frame_duration_ms` - Frame duration in milliseconds (typically 20ms)
    ///
    /// # Returns
    /// New packetizer instance
    pub fn new(
        payload_type: u8,
        max_payload_size: usize,
        sample_rate: u32,
        frame_duration_ms: u32,
    ) -> Self {
        let mut rng = rand::thread_rng();

        // Calculate timestamp increment
        // Example: 48000 Hz * 0.020 sec = 960 samples per 20ms frame
        let timestamp_increment = (sample_rate * frame_duration_ms) / 1000;

        OpusRtpPacketizer {
            ssrc: rng.gen_range(0..=u32::MAX),
            sequence_number: rng.gen_range(0..=u16::MAX),
            timestamp: 0,
            payload_type,
            max_payload_size,
            timestamp_increment,
        }
    }

    /// Packetize Opus audio data
    ///
    /// Opus frames typically fit in a single RTP packet.
    /// If the frame exceeds MTU, it will be split (rare case).
    fn packetize_audio(&mut self, data: &[u8]) -> Vec<RtpPacket> {
        let mut packets = Vec::new();

        if data.len() <= self.max_payload_size {
            // Single packet (normal case)
            let mut header = RtpHeader::new(self.payload_type, self.ssrc);
            header.sequence_number = self.sequence_number;
            header.timestamp = self.timestamp;
            header.marker = false; // Audio doesn't typically use marker bit

            self.sequence_number = self.sequence_number.wrapping_add(1);

            packets.push(RtpPacket::new(header, data.to_vec()));
        } else {
            // Split into multiple packets (rare for Opus)
            let mut offset = 0;
            while offset < data.len() {
                let chunk_size = (data.len() - offset).min(self.max_payload_size);
                let chunk = data[offset..offset + chunk_size].to_vec();

                let mut header = RtpHeader::new(self.payload_type, self.ssrc);
                header.sequence_number = self.sequence_number;
                header.timestamp = self.timestamp;
                header.marker = false;

                self.sequence_number = self.sequence_number.wrapping_add(1);

                packets.push(RtpPacket::new(header, chunk));
                offset += chunk_size;
            }
        }

        packets
    }
}

impl RtpPacketizer for OpusRtpPacketizer {
    fn packetize(&mut self, data: &[u8]) -> Vec<RtpPacket> {
        let packets = self.packetize_audio(data);

        // Increment timestamp for next frame
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
    fn test_opus_packetizer_small_frame() {
        let mut packetizer = OpusRtpPacketizer::new(111, 1400, 48000, 20);

        // Small Opus frame (typical size is 20-200 bytes)
        let audio_data = vec![0u8; 100];
        let packets = packetizer.packetize(&audio_data);

        assert_eq!(packets.len(), 1);
        assert_eq!(packets[0].payload.len(), 100);
    }

    #[test]
    fn test_opus_timestamp_increment() {
        let mut packetizer = OpusRtpPacketizer::new(111, 1400, 48000, 20);

        let audio_data = vec![0u8; 100];
        let initial_ts = packetizer.timestamp;

        packetizer.packetize(&audio_data);

        // For 48kHz with 20ms frames: increment should be 960
        assert_eq!(packetizer.timestamp, initial_ts + 960);
    }

    #[test]
    fn test_opus_sequence_number() {
        let mut packetizer = OpusRtpPacketizer::new(111, 1400, 48000, 20);

        let audio_data = vec![0u8; 100];
        let initial_seq = packetizer.sequence_number;

        let packets = packetizer.packetize(&audio_data);

        assert_eq!(packets[0].header.sequence_number, initial_seq);
        assert_eq!(packetizer.sequence_number, initial_seq.wrapping_add(1));
    }
}
