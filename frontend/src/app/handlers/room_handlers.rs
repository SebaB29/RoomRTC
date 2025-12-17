//! Room Management Handlers
//!
//! Handles room exit operations.

use crate::app::state::App;
use crate::events::LogicCommand;

impl App {
    /// Exits the current room and cleans up resources
    pub(in crate::app) fn handle_exit_room(&mut self) {
        let Some(room_id) = self.user_context.current_room_id.take() else {
            self.logger
                .warn("[ROOM] Exit room called but no current room");
            return;
        };

        let Some(user_name) = self.user_context.get_name().map(|s| s.to_string()) else {
            self.logger.warn("[ROOM] Exit room called but no user name");
            self.user_context.current_room_id = Some(room_id); // Restore if we can't proceed
            return;
        };

        self.logger.info(&format!(
            "[ROOM] User '{}' exiting room '{}'",
            user_name, room_id
        ));

        let is_owner = self
            .current_room
            .as_ref()
            .and_then(|room| room.get_participant(&user_name))
            .map(|p| p.role == crate::models::ParticipantRole::Owner)
            .unwrap_or(false);

        self.logger.info(&format!(
            "[ROOM] Sending disconnect message - user: '{}', is_owner: {}",
            user_name, is_owner
        ));
        self.logic_cmd_tx
            .send(LogicCommand::SendDisconnect { is_owner })
            .ok();
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Send hangup message via TCP if in active call (room_id IS the call_id)
        if let Some(ref client) = self.tcp_client {
            self.logger.info(&format!(
                "[ROOM] Sending hangup message - call_id: {}",
                room_id
            ));
            if let Err(e) = client.hangup(&room_id) {
                self.logger.error(&format!(
                    "[ROOM] Failed to send hangup for call '{}': {}",
                    room_id, e
                ));
            }
        }

        // Remove participant from room
        if let Some(room) = self.current_room.as_mut() {
            room.remove_participant(&user_name);

            // Remove empty room locally
            if room.is_empty() {
                self.logger
                    .info(&format!("[ROOM] Room '{}' is now empty, removing", room_id));
                self.current_room = None;
            }
        }

        // Clean up current room state (textures)
        self.current_room_state = None;

        self.logger.info(&format!(
            "[ROOM] Cleaning up connection for user '{}'",
            user_name
        ));

        // Clear peer context
        if let Some(ref peer_id) = self.user_context.peer_user_id {
            self.lobby.update_user_state(peer_id, "", "Available");
        }

        self.user_context.peer_user_id = None;
        self.user_context.outgoing_call_to = None;

        if let Some(ref user_id) = self.user_context.user_id {
            self.lobby
                .update_user_state(user_id, &user_name, "Available");
        }

        // Clean up and stop connection
        self.logic_cmd_tx
            .send(LogicCommand::StopConnection)
            .expect("Logic thread disconnected: failed to send StopConnection command");

        // Return to lobby
        self.current_page = crate::pages::Page::Lobby;
        self.logger
            .info(&format!("[ROOM] User '{}' returned to lobby", user_name));
    }
}
