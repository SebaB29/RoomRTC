//! Participant Models
//!
//! Defines participant data structures and roles for video call rooms.

use json_parser::{impl_json, impl_json_enum};

// Default participant settings
const DEFAULT_CAMERA_STATE: bool = false;
const DEFAULT_CAMERA_DEVICE: i32 = 0;
const DEFAULT_CAMERA_FPS: f64 = 30.0;
const DEFAULT_AUDIO_STATE: bool = false;
const DEFAULT_AUDIO_MUTED: bool = false;

/// Participant role in a room
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticipantRole {
    /// Room creator, can manage room settings
    Owner,
    /// Invited participant
    Guest,
}

impl_json_enum! {
    ParticipantRole {
        Owner,
        Guest,
    }
}

/// Participant information
#[derive(Clone, Debug)]
pub struct Participant {
    pub name: String,
    pub role: ParticipantRole,
    pub camera_on: bool,
    pub selected_camera_device: i32,
    pub camera_fps: f64,
    pub audio_on: bool,
    pub audio_muted: bool,
}

impl_json! {
    Participant {
        name: String,
        role: ParticipantRole,
        camera_on: bool,
        selected_camera_device: i32,
        camera_fps: f64,
        audio_on: bool,
        audio_muted: bool,
    }
}

impl Default for Participant {
    fn default() -> Self {
        Self {
            name: String::new(),
            role: ParticipantRole::Guest,
            camera_on: DEFAULT_CAMERA_STATE,
            selected_camera_device: DEFAULT_CAMERA_DEVICE,
            camera_fps: DEFAULT_CAMERA_FPS,
            audio_on: DEFAULT_AUDIO_STATE,
            audio_muted: DEFAULT_AUDIO_MUTED,
        }
    }
}

impl Participant {
    /// Creates a new participant with default camera settings
    pub fn new(name: String, role: ParticipantRole) -> Self {
        Self {
            name,
            role,
            camera_on: DEFAULT_CAMERA_STATE,
            selected_camera_device: DEFAULT_CAMERA_DEVICE,
            camera_fps: DEFAULT_CAMERA_FPS,
            audio_on: DEFAULT_AUDIO_STATE,
            audio_muted: DEFAULT_AUDIO_MUTED,
        }
    }
}
