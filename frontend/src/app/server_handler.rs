//! Handles incoming messages from the TCP server.
//!
//! This module processes all server messages and dispatches them to
//! specialized handlers organized by domain (lobby, calls, signaling).

use super::state::App;
use crate::models::protocol::ServerMessage;

impl App {
    /// Handle incoming messages from TCP server
    pub(super) fn handle_server_message(&mut self, message: ServerMessage) {
        // Log all incoming server messages with consistent format
        self.log_server_message(&message);

        match message {
            // User list and state updates
            ServerMessage::UserListResponse { users } => {
                self.logger.info(&format!(
                    "[USER_STATE] Updating lobby with {} users",
                    users.len()
                ));
                self.lobby.update_users(users);
            }

            ServerMessage::UserStateUpdate {
                user_id,
                username,
                state,
            } => {
                self.logger.info(&format!(
                    "[USER_STATE] User '{}' (id: {}) changed state to '{}'",
                    username, user_id, state
                ));
                self.lobby.update_user_state(&user_id, &username, &state);
            }

            // Call lifecycle events (dispatched to call_handlers)
            ServerMessage::CallNotification {
                call_id,
                from_user_id,
                from_username,
            } => {
                self.logger.info(&format!(
                    "[CALL] Incoming call notification - call_id: {}, from: {} (id: {})",
                    call_id, from_username, from_user_id
                ));
                self.handle_call_notification(call_id, from_user_id, from_username);
            }

            ServerMessage::CallAccepted {
                call_id,
                peer_user_id,
                peer_username,
            } => {
                self.logger.info(&format!(
                    "[CALL] Call accepted - call_id: {}, peer: {} (id: {})",
                    call_id, peer_username, peer_user_id
                ));
                self.handle_call_accepted(call_id, peer_user_id, peer_username);
            }

            ServerMessage::CallDeclined { peer_username } => {
                self.logger
                    .info(&format!("[CALL] Call declined by user '{}'", peer_username));
                self.handle_call_declined(peer_username);
            }

            ServerMessage::Hangup { call_id } => {
                self.logger
                    .info(&format!("[CALL] Call hangup - call_id: {}", call_id));
                self.handle_hangup(call_id);
            }

            // WebRTC signaling (dispatched to signaling_handlers)
            ServerMessage::SdpOffer {
                call_id,
                from_user_id,
                sdp,
            } => {
                self.logger.info(&format!(
                    "[SIGNALING] SDP Offer received - call_id: {}, from_user_id: {}, sdp_length: {}",
                    call_id, from_user_id, sdp.len()
                ));
                self.handle_sdp_offer(call_id, sdp);
            }

            ServerMessage::SdpAnswer {
                call_id,
                from_user_id,
                sdp,
            } => {
                self.logger.info(&format!(
                    "[SIGNALING] SDP Answer received - call_id: {}, from_user_id: {}, sdp_length: {}",
                    call_id, from_user_id, sdp.len()
                ));
                self.handle_sdp_answer(sdp);
            }

            ServerMessage::IceCandidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
            } => {
                self.logger.info(&format!(
                    "[SIGNALING] ICE Candidate received - mid: {}, mline_index: {}, candidate_length: {}",
                    sdp_mid, sdp_mline_index, candidate.len()
                ));
                self.handle_ice_candidate(candidate, sdp_mid, sdp_mline_index);
            }

            // Error handling
            ServerMessage::Error { message } => {
                self.logger.error(&format!("[SERVER_ERROR] {}", message));
                self.show_error(format!("Server error: {}", message));
            }

            // Authentication responses
            ServerMessage::LoginResponse {
                success,
                user_id,
                username,
                error,
            } => {
                self.logger.info(&format!(
                    "[AUTH] Login response received: success={}",
                    success
                ));
                self.handle_login_response(
                    &username.unwrap_or_default().to_string(),
                    success,
                    user_id,
                    error,
                );
            }

            ServerMessage::RegisterResponse {
                success,
                user_id,
                username,
                error,
            } => {
                self.logger.info(&format!(
                    "[AUTH] Register response received: success={}",
                    success
                ));
                self.handle_register_response(
                    &username.unwrap_or_default().to_string(),
                    success,
                    user_id,
                    error,
                );
            }

            ServerMessage::LogoutResponse { success, error } => {
                self.logger.info(&format!(
                    "[AUTH] Logout response received - success: {}",
                    success
                ));
                self.handle_logout_response(
                    &self.user_context.get_name().unwrap_or_default().to_string(),
                    success,
                    error,
                );
            }
        }
    }

    /// Logs the incoming server message type with consistent formatting
    fn log_server_message(&self, message: &ServerMessage) {
        self.logger
            .info(&format!("[SERVER_MSG] Received: {}", message));
    }
}
