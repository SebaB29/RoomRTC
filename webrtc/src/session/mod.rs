//! P2P session management
//!
//! This module handles the complete media pipeline for peer-to-peer
//! video streaming, including encoding, RTP packetization, and transport.

// P2P session module - handles WebRTC media pipeline
mod config;
mod control_message;
mod dtls_setup;
pub mod file_channel;
pub mod file_session;
pub mod file_transfer;
mod recv_thread;
mod secure_session;
mod send_thread;
mod video_decode_thread;

// Re-export public types
pub use control_message::ControlMessage;
pub use file_transfer::FileTransferEvent;

// Re-export internal types
pub(crate) use config::P2PConfig;
pub(crate) use secure_session::SecureP2PSession;
