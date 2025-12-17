//! Opus RTP packetization module

mod depacketizer;
mod packetizer;

pub use depacketizer::OpusRtpDepacketizer;
pub use packetizer::OpusRtpPacketizer;
