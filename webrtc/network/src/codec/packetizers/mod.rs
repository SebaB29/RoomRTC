//! RTP Packetizers for different codecs
//!
//! This module contains codec-specific RTP packetizer implementations.
//! Each codec follows its respective RFC specification for RTP payload format.
pub mod h264;
pub mod opus;

pub use h264::{H264RtpDepacketizer, H264RtpPacketizer};
pub use opus::{OpusRtpDepacketizer, OpusRtpPacketizer};
