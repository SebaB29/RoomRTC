use json_parser::JsonValue;
use std::collections::HashMap;

use super::{
    CallAcceptedMsg, CallDeclinedMsg, CallNotificationMsg, CallRequest, CallResponseMsg, ErrorMsg,
    HangupMsg, HeartbeatMsg, IceCandidateMsg, LoginRequest, LoginResponse, LogoutRequest,
    LogoutResponse, MessageType, RegisterRequest, RegisterResponse, SdpAnswerMsg, SdpOfferMsg,
    UserListResponse, UserStateUpdateMsg,
};

#[derive(Debug, Clone)]
pub enum Message {
    // Client → Server
    LoginRequest(LoginRequest),
    RegisterRequest(RegisterRequest),
    LogoutRequest(LogoutRequest),
    UserListRequest,
    CallRequest(CallRequest),
    CallResponse(CallResponseMsg),
    SdpOffer(SdpOfferMsg),
    SdpAnswer(SdpAnswerMsg),
    IceCandidate(IceCandidateMsg),
    Hangup(HangupMsg),
    Heartbeat(HeartbeatMsg),

    // Server → Client
    LoginResponse(LoginResponse),
    RegisterResponse(RegisterResponse),
    LogoutResponse(LogoutResponse),
    UserListResponse(UserListResponse),
    UserStateUpdate(UserStateUpdateMsg),
    CallNotification(CallNotificationMsg),
    CallAccepted(CallAcceptedMsg),
    CallDeclined(CallDeclinedMsg),
    Error(ErrorMsg),
}

impl Message {
    pub fn message_type(&self) -> MessageType {
        match self {
            Message::LoginRequest(_) => MessageType::LoginRequest,
            Message::LoginResponse(_) => MessageType::LoginResponse,
            Message::RegisterRequest(_) => MessageType::RegisterRequest,
            Message::RegisterResponse(_) => MessageType::RegisterResponse,
            Message::LogoutRequest(_) => MessageType::LogoutRequest,
            Message::LogoutResponse(_) => MessageType::LogoutResponse,
            Message::UserListRequest => MessageType::UserListRequest,
            Message::UserListResponse(_) => MessageType::UserListResponse,
            Message::UserStateUpdate(_) => MessageType::UserStateUpdate,
            Message::CallRequest(_) => MessageType::CallRequest,
            Message::CallNotification(_) => MessageType::CallNotification,
            Message::CallResponse(_) => MessageType::CallResponse,
            Message::CallAccepted(_) => MessageType::CallAccepted,
            Message::CallDeclined(_) => MessageType::CallDeclined,
            Message::SdpOffer(_) => MessageType::SdpOffer,
            Message::SdpAnswer(_) => MessageType::SdpAnswer,
            Message::IceCandidate(_) => MessageType::IceCandidate,
            Message::Hangup(_) => MessageType::Hangup,
            Message::Heartbeat(_) => MessageType::Heartbeat,
            Message::Error(_) => MessageType::Error,
        }
    }

    pub fn to_json(&self) -> JsonValue {
        match self {
            Message::LoginResponse(r) => r.to_json(),
            Message::RegisterResponse(r) => r.to_json(),
            Message::LogoutResponse(r) => r.to_json(),
            Message::UserListResponse(r) => r.to_json(),
            Message::UserStateUpdate(u) => u.to_json(),
            Message::CallNotification(n) => n.to_json(),
            Message::CallAccepted(a) => a.to_json(),
            Message::CallDeclined(d) => d.to_json(),
            Message::SdpOffer(o) => o.to_json(),
            Message::SdpAnswer(a) => a.to_json(),
            Message::IceCandidate(c) => c.to_json(),
            Message::Hangup(h) => h.to_json(),
            Message::Error(e) => e.to_json(),
            _ => JsonValue::Object(HashMap::new()),
        }
    }
}
