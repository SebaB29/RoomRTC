//! Server Protocol Messages
//!
//! Defines all message types that can be received from the TCP server.
//! These are DTOs (Data Transfer Objects) representing the communication protocol.

use super::UserInfo;

/// Message types matching backend protocol
#[derive(Debug, Clone)]
pub enum ServerMessage {
    LoginResponse {
        success: bool,
        user_id: Option<String>,
        username: Option<String>,
        error: Option<String>,
    },
    RegisterResponse {
        success: bool,
        user_id: Option<String>,
        username: Option<String>,
        error: Option<String>,
    },
    LogoutResponse {
        success: bool,
        error: Option<String>,
    },
    UserListResponse {
        users: Vec<UserInfo>,
    },
    UserStateUpdate {
        user_id: String,
        username: String,
        state: String,
    },
    CallNotification {
        call_id: String,
        from_user_id: String,
        from_username: String,
    },
    CallAccepted {
        call_id: String,
        peer_user_id: String,
        peer_username: String,
    },
    CallDeclined {
        peer_username: String,
    },
    SdpOffer {
        call_id: String,
        from_user_id: String,
        sdp: String,
    },
    SdpAnswer {
        call_id: String,
        from_user_id: String,
        sdp: String,
    },
    IceCandidate {
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u32,
    },
    Hangup {
        call_id: String,
    },
    Error {
        message: String,
    },
}

impl std::fmt::Display for ServerMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ServerMessage::LoginResponse { .. } => write!(f, "LoginResponse"),
            ServerMessage::RegisterResponse { .. } => write!(f, "RegisterResponse"),
            ServerMessage::LogoutResponse { .. } => write!(f, "LogoutResponse"),
            ServerMessage::UserListResponse { .. } => write!(f, "UserListResponse"),
            ServerMessage::UserStateUpdate { .. } => write!(f, "UserStateUpdate"),
            ServerMessage::CallNotification { .. } => write!(f, "CallNotification"),
            ServerMessage::CallAccepted { .. } => write!(f, "CallAccepted"),
            ServerMessage::CallDeclined { .. } => write!(f, "CallDeclined"),
            ServerMessage::SdpOffer { .. } => write!(f, "SdpOffer"),
            ServerMessage::SdpAnswer { .. } => write!(f, "SdpAnswer"),
            ServerMessage::IceCandidate { .. } => write!(f, "IceCandidate"),
            ServerMessage::Hangup { .. } => write!(f, "Hangup"),
            ServerMessage::Error { .. } => write!(f, "Error"),
        }
    }
}
