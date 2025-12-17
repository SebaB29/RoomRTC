//! RTCP (RTP Control Protocol) Implementation
//!
//! Provides control and statistics for RTP sessions

pub mod bye;
pub mod receiver_report;
pub mod sender_report;
pub mod stats;

pub use bye::ByePacket;
pub use receiver_report::ReceiverReport;
pub use sender_report::{ReportBlock, SenderReport};
pub use stats::RtcpStats;

/// Common RTCP header writing function to eliminate duplication
pub(crate) fn write_rtcp_header(bytes: &mut Vec<u8>, packet_type: RtcpPacketType, rc: usize) {
    let version = 2u8;
    let padding = 0u8;
    bytes.push((version << 6) | (padding << 5) | rc as u8);
    bytes.push(packet_type as u8);
    let length = (1 + rc * 6) as u16;
    bytes.extend_from_slice(&length.to_be_bytes());
}

/// RTCP packet types
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RtcpPacketType {
    /// Sender Report (200)
    SR = 200,
    /// Receiver Report (201)
    RR = 201,
    /// Source Description (202)
    SDES = 202,
    /// Goodbye (203)
    BYE = 203,
    /// Application-defined (204)
    APP = 204,
}

impl RtcpPacketType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            200 => Some(RtcpPacketType::SR),
            201 => Some(RtcpPacketType::RR),
            202 => Some(RtcpPacketType::SDES),
            203 => Some(RtcpPacketType::BYE),
            204 => Some(RtcpPacketType::APP),
            _ => None,
        }
    }
}
