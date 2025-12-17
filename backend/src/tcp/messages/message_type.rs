#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    LoginRequest = 0x01,
    LoginResponse = 0x02,
    RegisterRequest = 0x03,
    RegisterResponse = 0x04,
    UserListRequest = 0x05,
    UserListResponse = 0x06,
    UserStateUpdate = 0x07,
    CallRequest = 0x08,
    CallNotification = 0x09,
    CallResponse = 0x0A,
    CallAccepted = 0x0B,
    CallDeclined = 0x0C,
    SdpOffer = 0x0D,
    SdpAnswer = 0x0E,
    IceCandidate = 0x0F,
    Hangup = 0x10,
    Heartbeat = 0x11,
    Error = 0x12,
    LogoutRequest = 0x13,
    LogoutResponse = 0x14,
}

impl MessageType {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0x01 => Some(MessageType::LoginRequest),
            0x02 => Some(MessageType::LoginResponse),
            0x03 => Some(MessageType::RegisterRequest),
            0x04 => Some(MessageType::RegisterResponse),
            0x05 => Some(MessageType::UserListRequest),
            0x06 => Some(MessageType::UserListResponse),
            0x07 => Some(MessageType::UserStateUpdate),
            0x08 => Some(MessageType::CallRequest),
            0x09 => Some(MessageType::CallNotification),
            0x0A => Some(MessageType::CallResponse),
            0x0B => Some(MessageType::CallAccepted),
            0x0C => Some(MessageType::CallDeclined),
            0x0D => Some(MessageType::SdpOffer),
            0x0E => Some(MessageType::SdpAnswer),
            0x0F => Some(MessageType::IceCandidate),
            0x10 => Some(MessageType::Hangup),
            0x11 => Some(MessageType::Heartbeat),
            0x12 => Some(MessageType::Error),
            0x13 => Some(MessageType::LogoutRequest),
            0x14 => Some(MessageType::LogoutResponse),
            _ => None,
        }
    }
}
