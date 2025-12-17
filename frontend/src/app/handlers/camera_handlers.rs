//! Camera and Audio Operation Handlers
//!
//! Handles camera toggling, settings updates, and audio mute/unmute.

use crate::app::state::App;
use crate::events::LogicCommand;

impl App {
    /// Toggles the camera on/off for the current user
    pub(in crate::app) fn handle_toggle_camera(&mut self) {
        let room_id = match &self.user_context.current_room_id {
            Some(id) => id.clone(),
            None => {
                self.logger
                    .warn("[CAMERA] Toggle camera called but no current room");
                return;
            }
        };

        let user_name = match self.user_context.get_name() {
            Some(name) => name.to_string(),
            None => {
                self.logger
                    .warn("[CAMERA] Toggle camera failed - could not get user name");
                return;
            }
        };

        // Get participant and extract camera settings in a single scope
        let (camera_on, device_id, fps) = {
            let room = match self.current_room.as_mut() {
                Some(r) => r,
                None => {
                    self.logger.warn(&format!(
                        "[CAMERA] Toggle camera failed - room '{}' not found",
                        room_id
                    ));
                    return;
                }
            };

            let participant = match room.get_participant_mut(&user_name) {
                Some(p) => p,
                None => {
                    self.logger.warn(&format!(
                        "[CAMERA] Toggle camera failed - participant '{}' not found in room '{}'",
                        user_name, room_id
                    ));
                    return;
                }
            };

            // Toggle camera state and extract values
            participant.camera_on = !participant.camera_on;
            (
                participant.camera_on,
                participant.selected_camera_device,
                participant.camera_fps,
            )
        };

        self.logger.info(&format!(
            "[CAMERA] User '{}' toggled camera to {} in room '{}'",
            user_name,
            if camera_on { "ON" } else { "OFF" },
            room_id
        ));

        // Send command to logic thread
        if camera_on {
            self.logger.info(&format!(
                "[CAMERA] Starting camera - user: '{}', device: {}, fps: {}",
                user_name, device_id, fps
            ));
            self.logic_cmd_tx
                .send(LogicCommand::StartCamera { device_id, fps })
                .expect("Logic thread disconnected: failed to send StartCamera command");
        } else {
            self.logger
                .info(&format!("[CAMERA] Stopping camera - user: '{}'", user_name));
            self.logic_cmd_tx
                .send(LogicCommand::StopCamera)
                .expect("Logic thread disconnected: failed to send StopCamera command");
        }
    }

    /// Updates camera settings (device ID and FPS) for the current user
    pub(in crate::app) fn handle_update_camera_settings(&mut self, device_id: i32, fps: f64) {
        let user_name = match self.user_context.get_name() {
            Some(name) => name.to_string(),
            None => {
                self.logger
                    .warn("[CAMERA] Update camera settings failed - could not get user name");
                return;
            }
        };

        self.logger.info(&format!(
            "[CAMERA] Updating camera settings for user '{}' - device: {}, fps: {}",
            user_name, device_id, fps
        ));

        if let Some(participant) = self.get_current_participant_mut() {
            participant.selected_camera_device = device_id;
            participant.camera_fps = fps;
        } else {
            self.logger.warn(&format!(
                "[CAMERA] Update camera settings failed - participant '{}' not found",
                user_name
            ));
        }
    }

    /// Toggles the microphone on/off for the current user
    pub(in crate::app) fn handle_toggle_mute(&mut self) {
        let room_id = match &self.user_context.current_room_id {
            Some(id) => id.clone(),
            None => {
                self.logger
                    .warn("[AUDIO] Toggle microphone called but no current room");
                return;
            }
        };

        let user_name = match self.user_context.get_name() {
            Some(name) => name.to_string(),
            None => {
                self.logger
                    .warn("[AUDIO] Toggle microphone failed - could not get user name");
                return;
            }
        };

        // Determine action based on current state
        let action = {
            let room = match self.current_room.as_mut() {
                Some(r) => r,
                None => {
                    self.logger.warn(&format!(
                        "[AUDIO] Toggle microphone failed - room '{}' not found",
                        room_id
                    ));
                    return;
                }
            };

            let participant = match room.get_participant_mut(&user_name) {
                Some(p) => p,
                None => {
                    self.logger.warn(&format!(
                        "[AUDIO] Toggle microphone failed - participant '{}' not found in room '{}'",
                        user_name, room_id
                    ));
                    return;
                }
            };

            if participant.audio_on {
                // Audio is ON, so we toggle MUTE state (Soft Mute)
                participant.audio_muted = !participant.audio_muted;
                if participant.audio_muted {
                    "MUTE"
                } else {
                    "UNMUTE"
                }
            } else {
                // Audio is OFF, so we turn it ON (and ensure unmuted)
                participant.audio_on = true;
                participant.audio_muted = false;
                "START"
            }
        };

        self.logger.info(&format!(
            "[AUDIO] User '{}' action: {} in room '{}'",
            user_name, action, room_id
        ));

        // Dispatch command based on action
        match action {
            "START" => {
                self.logger.info(&format!(
                    "[AUDIO] Starting microphone - user: '{}'",
                    user_name
                ));
                self.logic_cmd_tx
                    .send(LogicCommand::StartAudio {
                        sample_rate: 48000,
                        channels: 2,
                    })
                    .expect("Logic thread disconnected: failed to send StartAudio command");
            }
            "MUTE" | "UNMUTE" => {
                self.logger.info(&format!(
                    "[AUDIO] Toggling mute (val: {}) - user: '{}'",
                    action, user_name
                ));
                self.logic_cmd_tx
                    .send(LogicCommand::ToggleMute)
                    .expect("Logic thread disconnected: failed to send ToggleMute command");
            }
            _ => {}
        }
    }
}
