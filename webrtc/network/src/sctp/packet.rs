//! SCTP packet structure
//!
//! An SCTP packet consists of a common header followed by one or more chunks.
//!
//! ```text
//!  0                   1                   2                   3
//!  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |     Source Port Number        |     Destination Port Number   |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                      Verification Tag                         |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                           Checksum                            |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! |                                                               |
//! /                            Chunks                             /
//! |                                                               |
//! +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
//! ```

use super::chunk::SctpChunk;
use std::io;

/// SCTP common header size in bytes
pub const SCTP_HEADER_SIZE: usize = 12;

/// SCTP packet containing header and chunks
#[derive(Debug, Clone)]
pub struct SctpPacket {
    /// Source port
    pub source_port: u16,
    /// Destination port
    pub destination_port: u16,
    /// Verification tag
    pub verification_tag: u32,
    /// Chunks in this packet
    pub chunks: Vec<SctpChunk>,
}

impl SctpPacket {
    /// Create a new SCTP packet
    pub fn new(source_port: u16, destination_port: u16, verification_tag: u32) -> Self {
        Self {
            source_port,
            destination_port,
            verification_tag,
            chunks: Vec::new(),
        }
    }

    /// Add a chunk to the packet
    pub fn add_chunk(&mut self, chunk: SctpChunk) {
        self.chunks.push(chunk);
    }

    /// Serialize packet to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        // Calculate total size
        let chunks_size: usize = self.chunks.iter().map(|c| c.padded_len()).sum();
        let mut buf = Vec::with_capacity(SCTP_HEADER_SIZE + chunks_size);

        buf.extend_from_slice(&self.source_port.to_be_bytes());
        buf.extend_from_slice(&self.destination_port.to_be_bytes());
        buf.extend_from_slice(&self.verification_tag.to_be_bytes());
        buf.extend_from_slice(&[0u8; 4]);

        // Serialize chunks
        for chunk in &self.chunks {
            let chunk_bytes = chunk.to_bytes();
            buf.extend_from_slice(&chunk_bytes);
            // Pad to 4-byte boundary
            let padding = (4 - (chunk_bytes.len() % 4)) % 4;
            buf.extend(std::iter::repeat_n(0u8, padding));
        }

        // Calculate and insert CRC32c checksum
        let checksum = crc32c(&buf);
        buf[8..12].copy_from_slice(&checksum.to_le_bytes());

        buf
    }

    /// Parse packet from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < SCTP_HEADER_SIZE {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SCTP packet too short",
            ));
        }

        let source_port = u16::from_be_bytes([data[0], data[1]]);
        let destination_port = u16::from_be_bytes([data[2], data[3]]);
        let verification_tag = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        // Checksum at bytes 8-11 (verified separately if needed)

        let mut chunks = Vec::new();
        let mut offset = SCTP_HEADER_SIZE;

        while offset < data.len() {
            if offset + 4 > data.len() {
                break; // Not enough data for chunk header
            }

            let _chunk_type = data[offset];
            let declared_length = u16::from_be_bytes([data[offset + 2], data[offset + 3]]) as usize;

            if declared_length < 4 {
                break;
            }

            // Calculate actual chunk length to use
            let chunk_length = if offset + declared_length > data.len() {
                // Declared length exceeds packet boundary, use remaining bytes
                data.len() - offset
            } else {
                declared_length
            };

            match SctpChunk::from_bytes(&data[offset..offset + chunk_length]) {
                Ok(chunk) => {
                    chunks.push(chunk);
                }
                Err(_) => {
                    break; // Stop on parse error
                }
            }

            offset += (chunk_length + 3) & !3;
        }

        Ok(Self {
            source_port,
            destination_port,
            verification_tag,
            chunks,
        })
    }

    /// Verify packet checksum
    pub fn verify_checksum(&self, data: &[u8]) -> bool {
        if data.len() < SCTP_HEADER_SIZE {
            return false;
        }

        let received_checksum = u32::from_le_bytes([data[8], data[9], data[10], data[11]]);

        // Zero out checksum field for verification
        let mut verify_data = data.to_vec();
        verify_data[8..12].copy_from_slice(&[0u8; 4]);

        let calculated_checksum = crc32c(&verify_data);
        received_checksum == calculated_checksum
    }
}

/// Calculate CRC32c checksum (RFC 3309)
fn crc32c(data: &[u8]) -> u32 {
    // CRC32c polynomial: 0x1EDC6F41 (Castagnoli)
    const CRC32C_TABLE: [u32; 256] = generate_crc32c_table();

    let mut crc: u32 = 0xFFFFFFFF;
    for byte in data {
        let index = ((crc ^ (*byte as u32)) & 0xFF) as usize;
        crc = CRC32C_TABLE[index] ^ (crc >> 8);
    }
    !crc
}

/// Generate CRC32c lookup table at compile time
const fn generate_crc32c_table() -> [u32; 256] {
    const POLYNOMIAL: u32 = 0x82F63B78; // Reflected polynomial
    let mut table = [0u32; 256];
    let mut i = 0;
    while i < 256 {
        let mut crc = i as u32;
        let mut j = 0;
        while j < 8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ POLYNOMIAL;
            } else {
                crc >>= 1;
            }
            j += 1;
        }
        table[i] = crc;
        i += 1;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sctp::chunk::{DataChunk, ppid};

    #[test]
    fn test_packet_roundtrip() {
        let mut packet = SctpPacket::new(5000, 5000, 0x12345678);
        packet.add_chunk(SctpChunk::Data(DataChunk::new(
            1,
            0,
            0,
            ppid::BINARY,
            vec![1, 2, 3, 4],
        )));

        let bytes = packet.to_bytes();
        let parsed = SctpPacket::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.source_port, 5000);
        assert_eq!(parsed.destination_port, 5000);
        assert_eq!(parsed.verification_tag, 0x12345678);
        assert_eq!(parsed.chunks.len(), 1);
    }

    #[test]
    fn test_checksum_verification() {
        let mut packet = SctpPacket::new(5000, 5000, 0x12345678);
        packet.add_chunk(SctpChunk::CookieAck);

        let bytes = packet.to_bytes();
        assert!(packet.verify_checksum(&bytes));
    }
}
