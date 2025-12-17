//! Call Event Handlers
//!
//! Handles call-related server messages: notifications, accept/decline, hangup

use crate::app::state::App;
use crate::events::LogicCommand;
use crate::pages::Page;

impl App {
    /// Handles incoming call notification
    pub(in crate::app) fn handle_call_notification(
        &mut self,
        call_id: String,
        from_user_id: String,
        from_username: String,
    ) {
        // Check for outgoing calls: auto-decline incoming call if we're already calling someone
        if let Some(ref outgoing_to) = self.user_context.outgoing_call_to {
            if outgoing_to == &from_user_id {
                self.logger.warn(&format!(
                    "[CALL] Race condition detected: We're calling '{}' who is also calling us. Auto-declining their call.",
                    from_username
                ));

                if let Some(ref client) = self.tcp_client
                    && let Err(e) = client.respond_to_call(&call_id, false) {
                        self.logger
                            .error(&format!("Failed to auto-decline call: {}", e));
                    }
                return;
            } else {
                self.logger.warn(&format!(
                    "[CALL] Already calling '{}', auto-declining incoming call from '{}'.",
                    outgoing_to, from_username
                ));

                if let Some(ref client) = self.tcp_client
                    && let Err(e) = client.respond_to_call(&call_id, false) {
                        self.logger
                            .error(&format!("Failed to auto-decline call: {}", e));
                    }
                return;
            }
        }

        // Check if there's already a pending incoming call - auto-decline it
        if let Some(existing_call) = self.lobby.incoming_call.take() {
            self.logger.warn(&format!(
                "[CALL] Already have pending call from '{}', auto-declining to accept new call from '{}'",
                existing_call.from_username, from_username
            ));

            if let Some(ref client) = self.tcp_client
                && let Err(e) = client.respond_to_call(&existing_call.call_id, false) {
                    self.logger
                        .error(&format!("Failed to auto-decline existing call: {}", e));
                }
        }

        // Normal incoming call - show dialog
        self.lobby
            .set_incoming_call(call_id, from_user_id, from_username);
    }

    /// Handles call accepted by peer (caller side)
    pub(in crate::app) fn handle_call_accepted(
        &mut self,
        call_id: String,
        peer_user_id: String,
        peer_username: String,
    ) {
        let user_name = self.user_context.get_name().unwrap_or_default().to_string();

        self.logger.info(&format!(
            "[CALL] Creating room for call - call_id: {}, caller: '{}', peer: '{}'",
            call_id, user_name, peer_username
        ));

        // Store peer info in context
        self.user_context.peer_user_id = Some(peer_user_id.clone());

        // Mark peer as busy in lobby
        self.lobby.update_user_state(&peer_user_id, "", "Busy");

        // Create room data using call_id as room_id
        let room_id = call_id.clone();
        self.create_room(room_id.clone(), user_name, peer_username.clone());

        // Set current room
        self.user_context.current_room_id = Some(room_id.clone());
        self.current_page = Page::Room;

        self.user_context.outgoing_call_to = None;

        self.logger.info(&format!(
            "[CALL] Room created - room_id: {}, participants: 2, generating SDP offer",
            room_id
        ));

        self.show_success(format!("Connected to {}!", peer_username));

        // Generate offer (auto_start_connection will handle StartConnection after offer is ready)
        let _ = self.logic_cmd_tx.send(LogicCommand::GenerateOffer);
    }

    /// Handles call declined by peer
    pub(in crate::app) fn handle_call_declined(&mut self, peer_username: String) {
        self.user_context.outgoing_call_to = None;
        self.show_warning(format!("{} declined the call", peer_username));
    }

    /// Handles call hangup
    pub(in crate::app) fn handle_hangup(&mut self, call_id: String) {
        // Mark peer as available again
        if let Some(ref peer_id) = self.user_context.peer_user_id {
            self.lobby.update_user_state(peer_id, "", "Available");
        }

        if let Some(ref user_id) = self.user_context.user_id
            && let Some(username) = self.user_context.get_name() {
                self.lobby.update_user_state(user_id, username, "Available");
            }

        // End call, return to lobby
        if self.user_context.current_room_id.is_some() {
            let room_id = self.user_context.current_room_id.take().unwrap();
            self.logger.info(&format!(
                "[CALL] Cleaning up room '{}' after hangup",
                room_id
            ));
            self.current_room = None;
            self.current_room_state = None;
        }

        self.current_page = Page::Lobby;
        self.logger.info(&format!(
            "[CALL] Call ended, returned to lobby - call_id: {}",
            call_id
        ));
        self.show_success("Call ended".to_string());

        self.user_context.outgoing_call_to = None;

        // Stop WebRTC connection
        let _ = self.logic_cmd_tx.send(LogicCommand::StopConnection);
    }
}
