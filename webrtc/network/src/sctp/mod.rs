//! SCTP (Stream Control Transmission Protocol) implementation for WebRTC data channels
//!
//! This module provides a minimal SCTP implementation suitable for WebRTC data channels.
//! SCTP runs over DTLS and provides reliable, ordered delivery with multiplexing.
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────┐
//! │     DataChannel API         │
//! ├─────────────────────────────┤
//! │     SCTP Association        │  ← This module
//! ├─────────────────────────────┤
//! │     DTLS Transport          │
//! └─────────────────────────────┘
//! ```
//!
//! ## Implemented Features
//!
//! - INIT/INIT-ACK handshake
//! - DATA chunks with fragmentation
//! - SACK (Selective Acknowledgment)
//! - DCEP (Data Channel Establishment Protocol)
//!
//! ## Not Implemented
//!
//! - Multi-homing
//! - Path MTU discovery
//! - Partial reliability extensions

pub mod association;
pub mod chunk;
pub mod dcep;
pub mod packet;

pub use association::{AssociationConfig, AssociationState, SctpAssociation};
pub use chunk::{DataChunk, SctpChunk, SctpChunkType, ppid};
pub use dcep::{ChannelType, DataChannelAck, DataChannelOpen};
pub use packet::SctpPacket;
