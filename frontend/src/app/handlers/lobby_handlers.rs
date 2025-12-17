//! Lobby Command Handlers
//!
//! Handles all lobby-related commands: calling users, accepting/declining calls, etc.

use crate::app::state::App;
use crate::pages::Page;

impl App {
    /// Initiates a call to another user
    pub(in crate::app) fn handle_call_user(&mut self, to_user_id: String) {
        let Some(ref client) = self.tcp_client else {
            self.logger
                .warn("[LOBBY] Call attempt failed - not connected to server");
            self.show_error("Not connected to server".to_string());
            return;
        };

        // Find peer username from user list
        let peer_username = self
            .lobby
            .users
            .iter()
            .find(|u| u.user_id == to_user_id)
            .map(|u| u.username.clone())
            .unwrap_or_else(|| to_user_id.clone());

        self.logger.info(&format!(
            "[LOBBY] Initiating call to user '{}' (id: {})",
            peer_username, to_user_id
        ));

        match client.call_user(&to_user_id) {
            Ok(()) => {
                self.user_context.outgoing_call_to = Some(to_user_id.clone());
                self.logger.info(&format!(
                    "[LOBBY] Call request sent successfully to '{}', waiting for response",
                    peer_username
                ));
                self.show_success("Calling...".to_string());
            }
            Err(e) => {
                self.logger.error(&format!(
                    "[LOBBY] Failed to send call request to '{}': {}",
                    peer_username, e
                ));
                self.show_error(format!("Call failed: {}", e));
            }
        }
    }

    /// Accepts an incoming call
    pub(in crate::app) fn handle_accept_call(
        &mut self,
        call_id: String,
        caller_id: String,
        caller_name: String,
    ) {
        let Some(ref client) = self.tcp_client else {
            self.logger
                .warn("[LOBBY] Accept call failed - not connected to server");
            self.show_error("Not connected to server".to_string());
            return;
        };

        let user_name = self
            .user_context
            .get_name()
            .unwrap_or("UNKNOWN")
            .to_string();
        self.logger.info(&format!(
            "[LOBBY] User '{}' accepting call from '{}' - call_id: {}",
            user_name, caller_name, call_id
        ));

        match client.respond_to_call(&call_id, true) {
            Ok(()) => {
                // Store peer info in context
                self.user_context.peer_user_id = Some(caller_id);

                self.logger.info(&format!(
                    "[LOBBY] Creating room for accepted call - call_id: {}, participants: [{}, {}]",
                    call_id, caller_name, user_name
                ));

                // Create room immediately (use call_id as room_id)
                let room_id = call_id.clone();
                self.create_room(room_id.clone(), caller_name.clone(), user_name.to_string());

                // Set current room and transition to Room page
                self.user_context.current_room_id = Some(room_id);
                self.current_page = Page::Room;

                self.user_context.outgoing_call_to = None;

                self.logger.info(&format!(
                    "[LOBBY] Room created successfully, waiting for SDP offer from '{}'",
                    caller_name
                ));

                self.show_success("Call accepted! Waiting for connection...".to_string());

                // Clear the incoming call notification
                self.lobby.clear_incoming_call();
            }
            Err(e) => {
                self.user_context.outgoing_call_to = None;
                self.logger.error(&format!(
                    "[LOBBY] Failed to accept call from '{}': {}",
                    caller_name, e
                ));
                self.show_error(format!("Accept failed: {}", e));
            }
        }
    }

    /// Declines an incoming call
    pub(in crate::app) fn handle_decline_call(&mut self, call_id: String) {
        let Some(ref client) = self.tcp_client else {
            self.logger
                .warn("[LOBBY] Decline call failed - not connected to server");
            self.show_error("Not connected to server".to_string());
            return;
        };

        self.logger
            .info(&format!("[LOBBY] Declining call - call_id: {}", call_id));

        match client.respond_to_call(&call_id, false) {
            Ok(()) => {
                self.user_context.outgoing_call_to = None;
                self.logger.info(&format!(
                    "[LOBBY] Call declined successfully - call_id: {}",
                    call_id
                ));
                self.show_success("Call declined".to_string());
            }
            Err(e) => {
                self.user_context.outgoing_call_to = None;
                self.logger.error(&format!(
                    "[LOBBY] Failed to decline call '{}': {}",
                    call_id, e
                ));
                self.show_error(format!("Decline failed: {}", e));
            }
        }
    }
}
