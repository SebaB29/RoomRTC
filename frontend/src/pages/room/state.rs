//! Room State Management
//!
//! This module defines the per-room state for video textures.
//! WebRTC connections and frame processing are handled by the logic thread.

use egui::TextureHandle;

/// State for each room (video textures only)
/// WebRTC connection is managed in logic thread, not here
pub struct RoomState {
    pub my_texture: Option<TextureHandle>,
    pub other_texture: Option<TextureHandle>,
}

impl RoomState {
    /// Creates a new empty RoomState
    pub fn new() -> Self {
        Self {
            my_texture: None,
            other_texture: None,
        }
    }
}

impl Default for RoomState {
    fn default() -> Self {
        Self::new()
    }
}
