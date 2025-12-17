//! # WebRTC - Own simplified implementation
//!
//! This library implements basic WebRTC components in a modular way.
//!
//! ## Public API
//!
//! External users should ONLY use these public types:
//!
//! ### High-Level API
//! - **`WebRtcConnection`** - Main interface (backward compatible, uses plain UDP)
//! - **`SecureWebRtcConnection`** - SECURE interface with DTLS/SRTP encryption
//! - **`RgbFrame`** - Type alias for RGB frame data: `(width, height, pixel_data)`
//! - **`CameraInfo`** - Camera device information
//! - **`CameraManager`** - Camera lifecycle management
//! - **`CameraResolution`** - Camera resolution information
//! - **`AudioInfo`** - Audio device information
//! - **`AudioManager`** - Audio lifecycle management
//! - **`AudioSettings`** - Audio configuration information
//! - **`AudioFrame`** - Audio sample data
//! - **`ControlMessage`** - Control message types (CameraOn, CameraOff, ParticipantDisconnected)
//!
//! ### ICE/STUN/TURN API (for signaling servers)
//! - **`IceAgent`** - ICE candidate gathering and management
//! - **`Candidate`** - ICE candidate representation
//! - **`CandidateType`** - Candidate types (Host, Srflx, Relay)
//! - **`StunClient`** - STUN client for NAT discovery
//! - **`TurnClient`** - TURN client for relay allocation
//!
//! ### SDP API (for signaling)
//! - **`SessionDescription`** - Parsed SDP structure
//! - **`SessionDescriptionBuilder`** - Fluent SDP builder
//! - **`MediaDescription`** - SDP media line representation
//!
//! ### Configuration API
//! - **`RoomRtcConfig`** - Complete RoomRTC configuration
//! - **`ServerConfig`** - Server-specific configuration
//! - **`WebRtcConfig`** - WebRTC settings (STUN/TURN servers)
//!
//! ## Example Usage
//!
//! ```no_run
//! use webrtc::{IceAgent, SessionDescriptionBuilder, SdpType};
//!
//! // Create ICE agent and gather candidates
//! let mut agent = IceAgent::new();
//! agent.gather_host_candidates(5000).unwrap();
//!
//! // Gather reflexive candidates via STUN
//! let stun_servers = vec!["stun.l.google.com:19302".to_string()];
//! agent.gather_server_reflexive_candidates(5000, &stun_servers).unwrap();
//!
//! // Gather relay candidates via TURN
//! let turn_servers = vec![
//!     "turn:turn.example.com:3478?transport=udp&username=user&password=pass".to_string()
//! ];
//! agent.gather_relay_candidates(5000, &turn_servers).unwrap();
//!
//! // Build SDP offer with all candidates
//! let origin = webrtc::Origin::parse("user 123 1 IN IP4 127.0.0.1").unwrap();
//! let sdp = SessionDescriptionBuilder::new(SdpType::Offer)
//!     .origin(origin)
//!     .session_name("WebRTC Session")
//!     .ice_credentials(agent.get_ufrag(), agent.get_pwd())
//!     .ice_candidates(&agent.get_local_candidates_strings())
//!     .build();
//! ```

// Internal modules (not exposed publicly)
mod audio_info;
mod audio_manager;
mod camera_info;
mod camera_manager;
mod connection;
mod session;

// ===== PUBLIC API - High Level =====
pub use audio_info::AudioInfo;
pub use audio_manager::{AudioManager, AudioSettings};
pub use camera_info::CameraInfo;
pub use camera_manager::{CameraManager, CameraResolution};
pub use connection::{RgbFrame, WebRtcConnection};
pub use session::{ControlMessage, FileTransferEvent};

// ===== PUBLIC API - Audio =====
pub use media::AudioFrame;

// ===== PUBLIC API - Secure Network (DTLS/SRTP) =====
pub use network::{
    DtlsContext, JitterBufferStats, PacketStats, RtcpStats, SecureUdpTransport, SrtpContext,
    SrtpKeys,
};

// ===== PUBLIC API - ICE =====
pub use ice::{
    Candidate, CandidateBuilder, CandidatePair, CandidateType, ConnectionState, IceAgent, IceError,
    detect_local_ip,
};

// ===== PUBLIC API - STUN =====
pub use stun::{StunClient, StunError};

// ===== PUBLIC API - TURN =====
pub use turn::{TransportProtocol, TurnAttributeType, TurnClient, TurnError, TurnMessageType};

// ===== PUBLIC API - SDP =====
pub use sdp::{
    Attribute as SdpAttribute, Connection as SdpConnection, MediaDescription, Origin, SdpError,
    SdpType, SessionDescription, SessionDescriptionBuilder,
};
