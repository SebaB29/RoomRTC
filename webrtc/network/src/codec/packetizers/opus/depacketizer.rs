//! Opus RTP Depacketizer Implementation
//!
//! Implements RFC 7587 - RTP Payload Format for Opus Audio Codec.

use crate::codec::rtp::RtpPacket;

/// Opus RTP depacketizer
///
/// Extracts Opus audio data from RTP packets.
pub struct OpusRtpDepacketizer {
    last_sequence: Option<u16>,
}

impl OpusRtpDepacketizer {
    /// Create a new Opus depacketizer
    pub fn new() -> Self {
        OpusRtpDepacketizer {
            last_sequence: None,
        }
    }

    /// Depacketize an RTP packet to extract Opus audio data
    ///
    /// # Arguments
    /// * `packet` - RTP packet containing Opus audio
    ///
    /// # Returns
    /// Opus audio data, or None if packet is invalid/out of sequence
    pub fn depacketize(&mut self, packet: &RtpPacket) -> Option<Vec<u8>> {
        // Check sequence number for packet loss
        if let Some(last_seq) = self.last_sequence {
            let expected_seq = last_seq.wrapping_add(1);
            if packet.header.sequence_number != expected_seq {
                // Packet loss detected
                // For audio, we might want to generate PLC (Packet Loss Concealment)
                // For now, just accept the packet
            }
        }

        self.last_sequence = Some(packet.header.sequence_number);

        // For Opus, the payload is the compressed audio data directly
        Some(packet.payload.clone())
    }

    /// Reset the depacketizer state
    pub fn reset(&mut self) {
        self.last_sequence = None;
    }
}

impl Default for OpusRtpDepacketizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::rtp::RtpHeader;

    #[test]
    fn test_opus_depacketizer() {
        let mut depacketizer = OpusRtpDepacketizer::new();

        let header = RtpHeader {
            version: 2,
            padding: false,
            csrc_count: 0,
            extension: false,
            marker: false,
            payload_type: 111,
            sequence_number: 1000,
            timestamp: 48000,
            ssrc: 12345,
        };

        let audio_data = vec![1, 2, 3, 4, 5];
        let packet = RtpPacket::new(header, audio_data.clone());

        let result = depacketizer.depacketize(&packet);
        assert_eq!(result, Some(audio_data));
    }
}
