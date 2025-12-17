//! H.264 RTP Packetization (RFC 6184)
//!
//! This module implements RTP packetization and depacketization for H.264 video
//! according to RFC 6184 specification.

mod depacketizer;
mod packetizer;

pub use depacketizer::H264RtpDepacketizer;
pub use packetizer::H264RtpPacketizer;
