//! SCTP chunk types and structures
//!
//! SCTP packets contain one or more chunks. Each chunk has a type, flags, length, and payload.
//! This module defines the chunk types used in WebRTC data channels.

use std::io;

/// SCTP chunk type identifiers (RFC 4960)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SctpChunkType {
    /// Payload data
    Data = 0,
    /// Initiate association
    Init = 1,
    /// Initiate acknowledgment
    InitAck = 2,
    /// Selective acknowledgment
    Sack = 3,
    /// Heartbeat request
    Heartbeat = 4,
    /// Heartbeat acknowledgment
    HeartbeatAck = 5,
    /// Abort association
    Abort = 6,
    /// Shutdown association
    Shutdown = 7,
    /// Shutdown acknowledgment
    ShutdownAck = 8,
    /// Operation error
    Error = 9,
    /// State cookie
    CookieEcho = 10,
    /// Cookie acknowledgment
    CookieAck = 11,
    /// Shutdown complete
    ShutdownComplete = 14,
    /// Forward TSN (RFC 3758)
    ForwardTsn = 192,
}

impl SctpChunkType {
    /// Parse chunk type from byte
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Data),
            1 => Some(Self::Init),
            2 => Some(Self::InitAck),
            3 => Some(Self::Sack),
            4 => Some(Self::Heartbeat),
            5 => Some(Self::HeartbeatAck),
            6 => Some(Self::Abort),
            7 => Some(Self::Shutdown),
            8 => Some(Self::ShutdownAck),
            9 => Some(Self::Error),
            10 => Some(Self::CookieEcho),
            11 => Some(Self::CookieAck),
            14 => Some(Self::ShutdownComplete),
            192 => Some(Self::ForwardTsn),
            _ => None,
        }
    }
}

/// DATA chunk for transmitting user data
///
/// ```text
///  0                   1                   2                   3
///  0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |   Type = 0    | Reserved|U|B|E|         Length                |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                              TSN                              |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |      Stream Identifier        |   Stream Sequence Number      |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// |                  Payload Protocol Identifier                  |
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// \                                                               \
/// /                           User Data                           /
/// \                                                               \
/// +-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataChunk {
    /// Unordered flag - if true, can be delivered out of order
    pub unordered: bool,
    /// Beginning fragment flag
    pub beginning: bool,
    /// Ending fragment flag  
    pub ending: bool,
    /// Transmission Sequence Number
    pub tsn: u32,
    /// Stream identifier
    pub stream_id: u16,
    /// Stream sequence number
    pub stream_seq: u16,
    /// Payload protocol identifier (PPID)
    pub ppid: u32,
    /// User data
    pub data: Vec<u8>,
}

/// Payload Protocol Identifiers for WebRTC
pub mod ppid {
    /// WebRTC DCEP (Data Channel Establishment Protocol)
    pub const DCEP: u32 = 50;
    /// WebRTC String (UTF-8)
    pub const STRING: u32 = 51;
    /// WebRTC Binary
    pub const BINARY: u32 = 53;
    /// WebRTC String Empty
    pub const STRING_EMPTY: u32 = 56;
    /// WebRTC Binary Empty
    pub const BINARY_EMPTY: u32 = 57;
}

impl DataChunk {
    /// Create a new DATA chunk
    pub fn new(tsn: u32, stream_id: u16, stream_seq: u16, ppid: u32, data: Vec<u8>) -> Self {
        Self {
            unordered: false,
            beginning: true,
            ending: true,
            tsn,
            stream_id,
            stream_seq,
            ppid,
            data,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(16 + self.data.len());

        // Type
        buf.push(SctpChunkType::Data as u8);

        // Flags: reserved(5) | U | B | E
        let flags = (if self.unordered { 0x04 } else { 0 })
            | (if self.beginning { 0x02 } else { 0 })
            | (if self.ending { 0x01 } else { 0 });
        buf.push(flags);

        // Length (header + data, padded to 4 bytes)
        let length = 16 + self.data.len() as u16;
        buf.extend_from_slice(&length.to_be_bytes());

        // TSN
        buf.extend_from_slice(&self.tsn.to_be_bytes());

        // Stream ID
        buf.extend_from_slice(&self.stream_id.to_be_bytes());

        // Stream Sequence Number
        buf.extend_from_slice(&self.stream_seq.to_be_bytes());

        // PPID
        buf.extend_from_slice(&self.ppid.to_be_bytes());

        // User data
        buf.extend_from_slice(&self.data);

        // Padding to 4-byte boundary
        let padding = (4 - (self.data.len() % 4)) % 4;
        buf.extend(std::iter::repeat_n(0u8, padding));

        buf
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DATA chunk too short",
            ));
        }

        let flags = data[1];
        let unordered = (flags & 0x04) != 0;
        let beginning = (flags & 0x02) != 0;
        let ending = (flags & 0x01) != 0;

        let declared_length = u16::from_be_bytes([data[2], data[3]]) as usize;
        let tsn = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let stream_id = u16::from_be_bytes([data[8], data[9]]);
        let stream_seq = u16::from_be_bytes([data[10], data[11]]);
        let ppid = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);

        let user_data_len = if declared_length > data.len() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "DATA chunk declared length exceeds available data",
            ));
        } else {
            declared_length.saturating_sub(16)
        };

        let user_data = data[16..16 + user_data_len].to_vec();

        Ok(Self {
            unordered,
            beginning,
            ending,
            tsn,
            stream_id,
            stream_seq,
            ppid,
            data: user_data,
        })
    }
}

/// INIT chunk for initiating association
#[derive(Debug, Clone)]
pub struct InitChunk {
    /// Initiate Tag
    pub initiate_tag: u32,
    /// Advertised Receiver Window Credit
    pub a_rwnd: u32,
    /// Number of outbound streams
    pub num_outbound_streams: u16,
    /// Number of inbound streams
    pub num_inbound_streams: u16,
    /// Initial TSN
    pub initial_tsn: u32,
}

impl InitChunk {
    /// Create new INIT chunk
    pub fn new(initiate_tag: u32, initial_tsn: u32) -> Self {
        Self {
            initiate_tag,
            a_rwnd: 131072, // 128KB default window
            num_outbound_streams: 65535,
            num_inbound_streams: 65535,
            initial_tsn,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(20);

        buf.push(SctpChunkType::Init as u8);
        buf.push(0); // Flags
        buf.extend_from_slice(&20u16.to_be_bytes()); // Length

        buf.extend_from_slice(&self.initiate_tag.to_be_bytes());
        buf.extend_from_slice(&self.a_rwnd.to_be_bytes());
        buf.extend_from_slice(&self.num_outbound_streams.to_be_bytes());
        buf.extend_from_slice(&self.num_inbound_streams.to_be_bytes());
        buf.extend_from_slice(&self.initial_tsn.to_be_bytes());

        buf
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 20 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "INIT chunk too short",
            ));
        }

        Ok(Self {
            initiate_tag: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            a_rwnd: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            num_outbound_streams: u16::from_be_bytes([data[12], data[13]]),
            num_inbound_streams: u16::from_be_bytes([data[14], data[15]]),
            initial_tsn: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
        })
    }
}

/// SACK chunk for selective acknowledgment
#[derive(Debug, Clone)]
pub struct SackChunk {
    /// Cumulative TSN Ack
    pub cumulative_tsn: u32,
    /// Advertised Receiver Window Credit
    pub a_rwnd: u32,
    /// Gap Ack Blocks
    pub gap_ack_blocks: Vec<(u16, u16)>,
    /// Duplicate TSNs
    pub duplicate_tsns: Vec<u32>,
}

impl SackChunk {
    /// Create new SACK chunk
    pub fn new(cumulative_tsn: u32, a_rwnd: u32) -> Self {
        Self {
            cumulative_tsn,
            a_rwnd,
            gap_ack_blocks: Vec::new(),
            duplicate_tsns: Vec::new(),
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let num_gap_blocks = self.gap_ack_blocks.len() as u16;
        let num_dup_tsns = self.duplicate_tsns.len() as u16;
        let length = 16 + (num_gap_blocks * 4) + (num_dup_tsns * 4);

        let mut buf = Vec::with_capacity(length as usize);

        buf.push(SctpChunkType::Sack as u8);
        buf.push(0); // Flags
        buf.extend_from_slice(&length.to_be_bytes());

        buf.extend_from_slice(&self.cumulative_tsn.to_be_bytes());
        buf.extend_from_slice(&self.a_rwnd.to_be_bytes());
        buf.extend_from_slice(&num_gap_blocks.to_be_bytes());
        buf.extend_from_slice(&num_dup_tsns.to_be_bytes());

        for (start, end) in &self.gap_ack_blocks {
            buf.extend_from_slice(&start.to_be_bytes());
            buf.extend_from_slice(&end.to_be_bytes());
        }

        for tsn in &self.duplicate_tsns {
            buf.extend_from_slice(&tsn.to_be_bytes());
        }

        buf
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 16 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "SACK chunk too short",
            ));
        }

        let cumulative_tsn = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let a_rwnd = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let num_gap_blocks = u16::from_be_bytes([data[12], data[13]]) as usize;
        let num_dup_tsns = u16::from_be_bytes([data[14], data[15]]) as usize;

        let mut gap_ack_blocks = Vec::with_capacity(num_gap_blocks);
        let mut offset = 16;

        for _ in 0..num_gap_blocks {
            if offset + 4 > data.len() {
                break;
            }
            let start = u16::from_be_bytes([data[offset], data[offset + 1]]);
            let end = u16::from_be_bytes([data[offset + 2], data[offset + 3]]);
            gap_ack_blocks.push((start, end));
            offset += 4;
        }

        let mut duplicate_tsns = Vec::with_capacity(num_dup_tsns);
        for _ in 0..num_dup_tsns {
            if offset + 4 > data.len() {
                break;
            }
            let tsn = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]);
            duplicate_tsns.push(tsn);
            offset += 4;
        }

        Ok(Self {
            cumulative_tsn,
            a_rwnd,
            gap_ack_blocks,
            duplicate_tsns,
        })
    }
}

/// Generic SCTP chunk wrapper
#[derive(Debug, Clone)]
pub enum SctpChunk {
    /// Data chunk
    Data(DataChunk),
    /// Init chunk
    Init(InitChunk),
    /// Init-Ack chunk (same format as Init)
    InitAck(InitChunk),
    /// Sack chunk
    Sack(SackChunk),
    /// Cookie Echo (opaque data)
    CookieEcho(Vec<u8>),
    /// Cookie Ack
    CookieAck,
    /// Shutdown
    Shutdown { cumulative_tsn: u32 },
    /// Shutdown Ack
    ShutdownAck,
    /// Shutdown Complete
    ShutdownComplete,
    /// Unknown chunk type
    Unknown { chunk_type: u8, data: Vec<u8> },
}

impl SctpChunk {
    /// Parse chunk from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.len() < 4 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Chunk too short",
            ));
        }

        let chunk_type = data[0];
        let declared_length = u16::from_be_bytes([data[2], data[3]]) as usize;

        match SctpChunkType::from_u8(chunk_type) {
            Some(SctpChunkType::Data) => Ok(SctpChunk::Data(DataChunk::from_bytes(data)?)),
            Some(SctpChunkType::Init) => Ok(SctpChunk::Init(InitChunk::from_bytes(data)?)),
            Some(SctpChunkType::InitAck) => Ok(SctpChunk::InitAck(InitChunk::from_bytes(data)?)),
            Some(SctpChunkType::Sack) => Ok(SctpChunk::Sack(SackChunk::from_bytes(data)?)),
            Some(SctpChunkType::CookieEcho) => {
                let cookie_len = declared_length.min(data.len());
                let cookie_data = data[4..cookie_len].to_vec();
                Ok(SctpChunk::CookieEcho(cookie_data))
            }
            Some(SctpChunkType::CookieAck) => Ok(SctpChunk::CookieAck),
            Some(SctpChunkType::Shutdown) => {
                if data.len() < 8 {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "Shutdown too short",
                    ));
                }
                let cumulative_tsn = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
                Ok(SctpChunk::Shutdown { cumulative_tsn })
            }
            Some(SctpChunkType::ShutdownAck) => Ok(SctpChunk::ShutdownAck),
            Some(SctpChunkType::ShutdownComplete) => Ok(SctpChunk::ShutdownComplete),
            _ => {
                let chunk_len = declared_length.min(data.len());
                Ok(SctpChunk::Unknown {
                    chunk_type,
                    data: data[4..chunk_len].to_vec(),
                })
            }
        }
    }

    /// Serialize chunk to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            SctpChunk::Data(chunk) => chunk.to_bytes(),
            SctpChunk::Init(chunk) => chunk.to_bytes(),
            SctpChunk::InitAck(chunk) => {
                let mut bytes = chunk.to_bytes();
                bytes[0] = SctpChunkType::InitAck as u8;
                bytes
            }
            SctpChunk::Sack(chunk) => chunk.to_bytes(),
            SctpChunk::CookieEcho(cookie) => {
                let length = 4 + cookie.len() as u16;
                let mut buf = Vec::with_capacity(length as usize);
                buf.push(SctpChunkType::CookieEcho as u8);
                buf.push(0);
                buf.extend_from_slice(&length.to_be_bytes());
                buf.extend_from_slice(cookie);
                buf
            }
            SctpChunk::CookieAck => {
                vec![SctpChunkType::CookieAck as u8, 0, 0, 4]
            }
            SctpChunk::Shutdown { cumulative_tsn } => {
                let mut buf = vec![SctpChunkType::Shutdown as u8, 0, 0, 8];
                buf.extend_from_slice(&cumulative_tsn.to_be_bytes());
                buf
            }
            SctpChunk::ShutdownAck => {
                vec![SctpChunkType::ShutdownAck as u8, 0, 0, 4]
            }
            SctpChunk::ShutdownComplete => {
                vec![SctpChunkType::ShutdownComplete as u8, 0, 0, 4]
            }
            SctpChunk::Unknown { chunk_type, data } => {
                let length = 4 + data.len() as u16;
                let mut buf = Vec::with_capacity(length as usize);
                buf.push(*chunk_type);
                buf.push(0);
                buf.extend_from_slice(&length.to_be_bytes());
                buf.extend_from_slice(data);
                buf
            }
        }
    }

    /// Get padded length (for packet serialization)
    pub fn padded_len(&self) -> usize {
        let len = self.to_bytes().len();
        (len + 3) & !3 // Round up to 4-byte boundary
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_data_chunk_roundtrip() {
        let chunk = DataChunk::new(1234, 5, 10, ppid::BINARY, vec![1, 2, 3, 4, 5]);
        let bytes = chunk.to_bytes();
        let parsed = DataChunk::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.tsn, 1234);
        assert_eq!(parsed.stream_id, 5);
        assert_eq!(parsed.stream_seq, 10);
        assert_eq!(parsed.ppid, ppid::BINARY);
        assert_eq!(parsed.data, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_init_chunk_roundtrip() {
        let chunk = InitChunk::new(0xDEADBEEF, 1000);
        let bytes = chunk.to_bytes();
        let parsed = InitChunk::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.initiate_tag, 0xDEADBEEF);
        assert_eq!(parsed.initial_tsn, 1000);
    }

    #[test]
    fn test_sack_chunk_roundtrip() {
        let mut chunk = SackChunk::new(100, 65535);
        chunk.gap_ack_blocks.push((1, 3));
        chunk.gap_ack_blocks.push((5, 7));

        let bytes = chunk.to_bytes();
        let parsed = SackChunk::from_bytes(&bytes).unwrap();

        assert_eq!(parsed.cumulative_tsn, 100);
        assert_eq!(parsed.gap_ack_blocks.len(), 2);
    }
}
