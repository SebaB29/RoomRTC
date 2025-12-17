//! Control message types for P2P communication
//!
//! This module defines control messages used for signaling camera state changes,
//! participant disconnections, file transfers, and other non-video data between peers.

use network::codec::rtp::control_payload;

/// Control message types for camera/audio state and peer connection synchronization
/// File transfers use SCTP DataChannel messages, not RTP control messages
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlMessage {
    CameraOn,
    CameraOff,
    AudioOn,
    AudioOff,
    AudioMuted,
    AudioUnmuted,
    ParticipantDisconnected,
    OwnerDisconnected,
    ParticipantName(String),
}

impl ControlMessage {
    /// Serialize control message to bytes
    /// Format: `[type_byte] [payload_bytes...]`
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            ControlMessage::CameraOn => vec![control_payload::CAMERA_ON],
            ControlMessage::CameraOff => vec![control_payload::CAMERA_OFF],
            ControlMessage::AudioOn => vec![control_payload::AUDIO_ON],
            ControlMessage::AudioOff => vec![control_payload::AUDIO_OFF],
            ControlMessage::AudioMuted => vec![control_payload::AUDIO_MUTED],
            ControlMessage::AudioUnmuted => vec![control_payload::AUDIO_UNMUTED],
            ControlMessage::ParticipantDisconnected => {
                vec![control_payload::PARTICIPANT_DISCONNECTED]
            }
            ControlMessage::OwnerDisconnected => vec![control_payload::OWNER_DISCONNECTED],
            ControlMessage::ParticipantName(name) => {
                let mut bytes = vec![control_payload::PARTICIPANT_NAME];
                bytes.extend_from_slice(name.as_bytes());
                bytes
            }
        }
    }

    /// Deserialize control message from bytes
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        match data[0] {
            control_payload::CAMERA_ON => Some(ControlMessage::CameraOn),
            control_payload::CAMERA_OFF => Some(ControlMessage::CameraOff),
            control_payload::AUDIO_ON => Some(ControlMessage::AudioOn),
            control_payload::AUDIO_OFF => Some(ControlMessage::AudioOff),
            control_payload::AUDIO_MUTED => Some(ControlMessage::AudioMuted),
            control_payload::AUDIO_UNMUTED => Some(ControlMessage::AudioUnmuted),
            control_payload::PARTICIPANT_DISCONNECTED => {
                Some(ControlMessage::ParticipantDisconnected)
            }
            control_payload::OWNER_DISCONNECTED => Some(ControlMessage::OwnerDisconnected),
            control_payload::PARTICIPANT_NAME => {
                if data.len() > 1 {
                    String::from_utf8(data[1..].to_vec())
                        .ok()
                        .map(ControlMessage::ParticipantName)
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}
