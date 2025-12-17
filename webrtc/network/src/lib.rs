//! Network Module - RTP and UDP Transport
//!
//! Handles RTP packetization/depacketization and UDP socket communication

// Organized submodules
pub mod codec;
pub mod datachannel;
pub mod sctp;
pub mod security;
pub mod transport;

// Utility modules
pub mod error;
pub mod traits;
pub mod utils;

// Re-export main types from submodules for backward compatibility
pub use codec::{
    ByePacket, H264RtpDepacketizer, H264RtpPacketizer, JitterBuffer, JitterBufferConfig,
    JitterBufferStats, OpusRtpDepacketizer, OpusRtpPacketizer, PacketHandler, PacketStats,
    ReceiverReport, RtcpPacketType, RtcpStats, RtpPacket, SenderReport,
};
pub use error::NetworkError;
pub use security::{DtlsContext, SrtpContext, SrtpKeys};
pub use traits::{RtpDepacketizer, RtpPacketizer};
pub use transport::{BasicUdpTransport, SecureUdpTransport, UdpTransport};
pub use utils::find_available_port;

pub type Result<T> = std::result::Result<T, NetworkError>;
