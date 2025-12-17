//! Handles events from the background WebRTC logic thread.

use super::state::App;
use crate::events::LogicCommand;
use crate::events::LogicEvent;
use crate::infrastructure::TcpClient;

impl App {
    /// Processes events from the logic thread
    /// Updates application state based on background operations
    pub(super) fn handle_logic_event(&mut self, ctx: &egui::Context, event: LogicEvent) {
        match event {
            LogicEvent::OfferGenerated(sdp) => {
                self.handle_offer_generated(sdp);
            }

            LogicEvent::AnswerGenerated(sdp) => {
                self.handle_answer_generated(sdp);
            }

            LogicEvent::ConnectionReady => {
                self.handle_connection_ready();
            }

            LogicEvent::LocalFrame(color_image) => {
                self.handle_local_frame(ctx, color_image);
            }

            LogicEvent::RemoteFrame(color_image) => {
                self.handle_remote_frame(ctx, color_image);
            }

            LogicEvent::CameraStarted => {
                self.handle_camera_started();
            }

            LogicEvent::CameraStopped => {
                self.handle_camera_stopped();
            }

            LogicEvent::RemoteCameraOn => {
                self.handle_remote_camera_on();
            }

            LogicEvent::RemoteCameraOff => {
                self.handle_remote_camera_off();
            }

            LogicEvent::AudioStarted => {
                self.handle_audio_started();
            }

            LogicEvent::AudioMuted => {
                self.handle_audio_muted();
            }

            LogicEvent::AudioUnmuted => {
                self.handle_audio_unmuted();
            }

            LogicEvent::RemoteAudioOn => {
                self.handle_remote_audio_on();
            }

            LogicEvent::RemoteAudioOff => {
                self.handle_remote_audio_off();
            }

            LogicEvent::RemoteAudioMuted => {
                self.handle_remote_audio_muted();
            }

            LogicEvent::RemoteAudioUnmuted => {
                self.handle_remote_audio_unmuted();
            }

            LogicEvent::RemoteParticipantName(name) => {
                self.handle_remote_participant_name(name);
            }

            LogicEvent::ParticipantDisconnected => {
                self.handle_participant_disconnected();
            }

            LogicEvent::OwnerDisconnected => {
                self.handle_owner_disconnected();
            }

            LogicEvent::StatsUpdated(stats) => {
                self.handle_stats_updated(stats);
            }

            LogicEvent::Error(msg) => {
                self.logger.error(&format!("Logic error: {}", msg));
                self.show_error(format!("Error: {}", msg));
            }

            // --- File Transfer Events ---
            LogicEvent::FileChannelReady => {
                self.logger
                    .info("[FILE] File channel ready - can now send/receive files");
                self.show_success("Ready to transfer files!".to_string());
            }

            LogicEvent::FileOfferReceived {
                transfer_id,
                filename,
                size,
            } => {
                self.logger.info(&format!(
                    "[FILE] Incoming file offer: {} ({} bytes), id: {}",
                    filename, size, transfer_id
                ));
                // Show file offer dialog to user
                self.incoming_file_offer = Some(crate::app::state::IncomingFileOffer {
                    transfer_id,
                    filename,
                    size,
                });
            }

            LogicEvent::FileTransferAccepted { transfer_id } => {
                self.logger
                    .info(&format!("[FILE] Transfer {} accepted", transfer_id));
                self.show_success("File transfer accepted".to_string());
            }

            LogicEvent::FileTransferRejected {
                transfer_id,
                reason,
            } => {
                self.logger.info(&format!(
                    "[FILE] Transfer {} rejected: {}",
                    transfer_id, reason
                ));
                self.show_warning(format!("File transfer rejected: {}", reason));
            }

            LogicEvent::FileTransferProgress {
                transfer_id,
                bytes_transferred,
                total_bytes,
            } => {
                // Update existing active transfer if it matches
                if let Some(transfer) = &mut self.active_file_transfer {
                    if transfer.transfer_id == transfer_id {
                        transfer.bytes_transferred = bytes_transferred;
                        // Only update total_size if it's larger (to handle initial estimates)
                        if total_bytes > transfer.total_size {
                            transfer.total_size = total_bytes;
                        }

                        let percent = if total_bytes > 0 {
                            (bytes_transferred * 100) / total_bytes
                        } else {
                            0
                        };

                        // Log progress at milestones (every 10%)
                        if percent % 10 == 0 && bytes_transferred > 0 {
                            self.logger.info(&format!(
                                "[FILE] Transfer {} progress: {}% ({}/{} bytes)",
                                transfer_id, percent, bytes_transferred, total_bytes
                            ));
                        }

                        // Request UI repaint so progress bar updates visually
                        ctx.request_repaint();
                    } else {
                        self.logger.warn(&format!(
                            "[FILE] Progress event for transfer {} but active transfer is {}",
                            transfer_id, transfer.transfer_id
                        ));
                    }
                } else {
                    // No active transfer yet - create one
                    // For sender: use pending_send_file info
                    // For receiver: this shouldn't happen (transfer starts on Accept)
                    let (filename, size) = self.pending_send_file.take()
                        .unwrap_or_else(|| {
                            self.logger.warn(&format!(
                                "[FILE] Progress event {} without pending_send_file or active_file_transfer",
                                transfer_id
                            ));
                            ("unknown".to_string(), total_bytes)
                        });

                    self.active_file_transfer = Some(crate::app::state::ActiveFileTransfer {
                        transfer_id,
                        filename: filename.clone(),
                        total_size: size,
                        bytes_transferred,
                        is_sending: true,
                    });

                    self.logger.info(&format!(
                        "[FILE] Started tracking sender transfer: {} ({}) - initial progress: {}/{}",
                        filename, transfer_id, bytes_transferred, size
                    ));
                }
            }

            LogicEvent::FileTransferCompleted { transfer_id, path } => {
                self.logger.info(&format!(
                    "[FILE] Transfer {} completed: {:?}",
                    transfer_id, path
                ));

                // Update progress to 100% BEFORE clearing
                if let Some(transfer) = &mut self.active_file_transfer
                    && transfer.transfer_id == transfer_id {
                        transfer.bytes_transferred = transfer.total_size;
                        self.logger.info(&format!(
                            "[FILE] Set progress to 100% ({}/{} bytes)",
                            transfer.total_size, transfer.total_size
                        ));
                    }

                // Force UI update to show 100%
                ctx.request_repaint();

                self.show_success(format!("File saved: {:?}", path));
            }

            LogicEvent::FileTransferFailed {
                transfer_id,
                reason,
            } => {
                self.logger.error(&format!(
                    "[FILE] Transfer {} failed: {}",
                    transfer_id, reason
                ));
                // Clear active transfer popup
                if let Some(transfer) = &self.active_file_transfer
                    && transfer.transfer_id == transfer_id {
                        self.active_file_transfer = None;
                    }
                self.show_error(format!("File transfer failed: {}", reason));
            }
        }
    }

    // --- Event Handlers ---

    fn handle_stats_updated(&mut self, stats: crate::components::CallStats) {
        // Update stats in current room
        if let Some(room) = self.current_room.as_mut() {
            room.stats = Some(stats);
        }
    }

    /// Sends SDP offer/answer via TCP with error handling
    fn send_sdp_via_tcp(
        &mut self,
        sdp: &str,
        sdp_type: &str,
        send_fn: impl FnOnce(&TcpClient, &str, &str, &str, &str) -> Result<(), String>,
    ) -> bool {
        // Get call_id from current room (room_id IS the call_id)
        let call_id = match &self.user_context.current_room_id {
            Some(id) => id.as_str(),
            None => {
                self.logger.error(&format!(
                    "[WEBRTC] Cannot send SDP {} - no current room",
                    sdp_type
                ));
                self.show_error("Connection error: not in a room".to_string());
                return false;
            }
        };

        // Get peer_id from context
        let peer_id = match &self.user_context.peer_user_id {
            Some(id) => id.as_str(),
            None => {
                self.logger.error(&format!(
                    "[WEBRTC] Cannot send SDP {} - no peer user ID in context",
                    sdp_type
                ));
                self.show_error("Connection error: peer not found".to_string());
                return false;
            }
        };

        let Some(ref client) = self.tcp_client else {
            self.logger.error(&format!(
                "[WEBRTC] Cannot send SDP {} - not connected to server",
                sdp_type
            ));
            self.show_error("Connection error: not connected to server".to_string());
            return false;
        };

        let user_id = self.user_context.get_user_id().unwrap_or("");
        self.logger.info(&format!(
            "[WEBRTC] Sending SDP {} to peer '{}' for call '{}'",
            sdp_type, peer_id, call_id
        ));

        match send_fn(client, call_id, peer_id, user_id, sdp) {
            Ok(()) => {
                self.logger.info(&format!(
                    "[WEBRTC] SDP {} sent successfully - call_id: {}",
                    sdp_type, call_id
                ));
                true
            }
            Err(e) => {
                self.logger.error(&format!(
                    "[WEBRTC] Failed to send SDP {} for call '{}': {}",
                    sdp_type, call_id, e
                ));
                self.show_error(format!("Failed to send {}: {}", sdp_type, e));
                false
            }
        }
    }

    fn handle_offer_generated(&mut self, sdp: String) {
        self.logger.info(&format!(
            "[WEBRTC] SDP Offer generated - length: {} bytes",
            sdp.len()
        ));

        if self.send_sdp_via_tcp(&sdp, "offer", TcpClient::send_sdp_offer) {
            self.show_success("Offer sent to peer!".to_string());
        }
    }

    fn handle_answer_generated(&mut self, sdp: String) {
        let my_name = self
            .user_context
            .get_name()
            .unwrap_or("UNKNOWN")
            .to_string();
        self.logger.info(&format!(
            "[WEBRTC] SDP Answer generated for user '{}' - length: {} bytes",
            my_name,
            sdp.len()
        ));

        if !self.send_sdp_via_tcp(&sdp, "answer", TcpClient::send_sdp_answer) {
            return;
        }

        self.logger.info(&format!(
            "[WEBRTC] SDP answer sent successfully for user '{}'",
            my_name
        ));
        self.show_success("Answer sent! Connecting...".to_string());

        // Auto-start connection
        self.logger.info(&format!(
            "[WEBRTC] Auto-starting connection for user '{}'",
            my_name
        ));
        self.auto_start_connection();
    }

    fn auto_start_connection(&self) {
        self.logger.info("[WEBRTC] Auto-start connection initiated");

        // Validate prerequisites
        let Some(room_id) = &self.user_context.current_room_id else {
            self.logger
                .error("[WEBRTC] Auto-start failed - no current room_id");
            return;
        };

        let Some(room) = self.current_room.as_ref() else {
            self.logger
                .error(&format!("[WEBRTC] Room '{}' not found", room_id));
            return;
        };

        let Some(user_name) = self.user_context.get_name() else {
            self.logger
                .error("[WEBRTC] Auto-start failed - no user name in context");
            return;
        };

        let Some(participant) = room.get_participant(user_name) else {
            self.logger.error(&format!(
                "[WEBRTC] Participant '{}' not found in room '{}'",
                user_name, room_id
            ));
            return;
        };

        self.logger.info(&format!(
            "[WEBRTC] Starting connection for '{}' in room '{}' ({} participants)",
            participant.name,
            room_id,
            room.participants.len()
        ));

        // Send start connection command
        if let Err(e) = self.logic_cmd_tx.send(LogicCommand::StartConnection {
            participant: participant.clone(),
        }) {
            self.logger.error(&format!(
                "[WEBRTC] Failed to send StartConnection command: {}",
                e
            ));
        }
    }

    fn handle_connection_ready(&mut self) {
        let my_name = self.user_context.get_name().unwrap_or("UNKNOWN");
        self.logger
            .info(&format!("[WEBRTC] Connection ready for user '{}'", my_name));

        // If we're already on the Room page (automatic flow), start the connection automatically
        if self.current_page == crate::pages::Page::Room {
            self.logger.info(&format!(
                "[WEBRTC] User '{}' on Room page, auto-starting connection",
                my_name
            ));
            // Get current room and participant info
            if let Some(room_id) = &self.user_context.current_room_id
                && let Some(room) = self.current_room.as_ref()
                && let Some(user_name) = self.user_context.get_name()
                && let Some(participant) = room.get_participant(user_name)
            {
                self.logger.info(&format!(
                    "[WEBRTC] Automatically starting connection for participant '{}' in room '{}'",
                    participant.name, room_id
                ));
                if let Err(e) =
                    self.logic_cmd_tx
                        .send(crate::events::LogicCommand::StartConnection {
                            participant: participant.clone(),
                        })
                {
                    self.logger.error(&format!(
                        "[WEBRTC] Failed to send StartConnection command: {}",
                        e
                    ));
                    self.show_error("Failed to start connection automatically".to_string());
                } else {
                    self.logger.info(&format!(
                        "[WEBRTC] WebRTC connection established for '{}'",
                        participant.name
                    ));
                    self.show_success("WebRTC connection established!".to_string());
                }
                return;
            }
            self.logger
                .warn("[WEBRTC] Failed to get participant info for automatic connection");
            self.show_error("Failed to get participant info for automatic connection".to_string());
        } else {
            // Manual flow: user needs to click "Join Room"
            self.logger.info(&format!(
                "[WEBRTC] Connection ready for user '{}', waiting for manual join",
                my_name
            ));
            self.show_success("Connection ready! You can now join the room.".to_string());
        }
    }

    fn handle_local_frame(&mut self, ctx: &egui::Context, color_image: egui::ColorImage) {
        // Only update if user is in a room
        if self.user_context.current_room_id.is_none() {
            return;
        }

        let Some(state) = &mut self.current_room_state else {
            return;
        };

        // Update or create local texture
        match &mut state.my_texture {
            Some(tex) => {
                tex.set(color_image, egui::TextureOptions::default());
            }
            None => {
                state.my_texture = Some(ctx.load_texture(
                    "local_frame",
                    color_image,
                    egui::TextureOptions::default(),
                ));
            }
        }
    }

    fn handle_remote_frame(&mut self, ctx: &egui::Context, color_image: egui::ColorImage) {
        // Only update if user is in a room
        if self.user_context.current_room_id.is_none() {
            return;
        }

        let Some(state) = &mut self.current_room_state else {
            return;
        };

        // Update or create remote texture
        match &mut state.other_texture {
            Some(tex) => {
                tex.set(color_image, egui::TextureOptions::default());
            }
            None => {
                state.other_texture = Some(ctx.load_texture(
                    "remote_frame",
                    color_image,
                    egui::TextureOptions::default(),
                ));
            }
        }
    }

    fn handle_camera_started(&mut self) {
        let user = self.user_context.get_name().unwrap_or("unknown");
        self.logger
            .info(&format!("[VIDEO] Local camera started for user '{}'", user));
        self.show_success("Camera started".to_string());
        self.update_local_camera_state(true);
    }

    fn handle_camera_stopped(&mut self) {
        let user = self.user_context.get_name().unwrap_or("unknown");
        self.logger
            .info(&format!("[VIDEO] Local camera stopped for user '{}'", user));
        self.show_warning("Camera stopped".to_string());

        // Clear local texture when camera stops
        if let Some(state) = &mut self.current_room_state {
            state.my_texture = None;
        }

        self.update_local_camera_state(false);
    }

    fn handle_remote_camera_on(&mut self) {
        self.logger.info("[VIDEO] Remote camera turned ON");
        self.update_remote_camera_state(true);
    }

    fn handle_remote_camera_off(&mut self) {
        self.logger.info("[VIDEO] Remote camera turned OFF");
        self.update_remote_camera_state(false);

        // Clear remote texture
        if let Some(state) = &mut self.current_room_state {
            state.my_texture = None;
        }

        // Flush jitter buffer and decoder to prevent delayed frames from being displayed
        self.logic_cmd_tx
            .send(crate::events::LogicCommand::ClearVideoBuffers)
            .ok();
    }

    fn handle_audio_started(&mut self) {
        self.logger.info("[AUDIO] Local audio started");
        self.show_success("Microphone started".to_string());
        self.update_local_audio_state(true, false);
    }

    fn handle_audio_muted(&mut self) {
        self.logger.info("[AUDIO] Local audio MUTED");
        self.show_info("Microphone muted".to_string());
        self.update_local_audio_state(true, true);
    }

    fn handle_audio_unmuted(&mut self) {
        self.logger.info("[AUDIO] Local audio UNMUTED");
        self.show_info("Microphone unmuted".to_string());
        self.update_local_audio_state(true, false);
    }

    fn handle_remote_audio_on(&mut self) {
        self.logger.info("[AUDIO] Remote audio turned ON");
        self.update_remote_audio_state(true, false);
    }

    fn handle_remote_audio_off(&mut self) {
        self.logger.info("[AUDIO] Remote audio turned OFF");
        self.update_remote_audio_state(false, false);
    }

    fn handle_remote_audio_muted(&mut self) {
        self.logger.info("[AUDIO] Remote audio MUTED");
        self.update_remote_audio_state(true, true);
    }

    fn handle_remote_audio_unmuted(&mut self) {
        self.logger.info("[AUDIO] Remote audio UNMUTED");
        self.update_remote_audio_state(true, false);
    }

    /// Handles guest participant disconnecting (owner remains in room)
    fn handle_participant_disconnected(&mut self) {
        let room_id = self
            .user_context
            .current_room_id
            .as_deref()
            .unwrap_or("unknown");
        self.logger.info(&format!(
            "[ROOM] Participant disconnected from room '{}'",
            room_id
        ));
        self.show_warning("Participant left the room".to_string());
        self.remove_other_participant();

        // Clear remote texture (will show "Waiting for participant...")
        if let Some(state) = &mut self.current_room_state {
            state.other_texture = None;
        }
    }

    /// Handles owner disconnecting (guest gets kicked out)
    fn handle_owner_disconnected(&mut self) {
        let room_id = self
            .user_context
            .current_room_id
            .as_deref()
            .unwrap_or("unknown");
        self.logger.info(&format!(
            "[ROOM] Owner disconnected from room '{}', ending call",
            room_id
        ));
        self.show_error("Room owner left. Ending call.".to_string());
        self.remove_other_participant();
        if let Some(state) = &mut self.current_room_state {
            state.other_texture = None;
        }
        self.current_room_state = None;

        self.logger
            .info("[ROOM] Stopping connection and returning to lobby");
        self.logic_cmd_tx
            .send(crate::events::LogicCommand::StopConnection)
            .ok();
        self.user_context.current_room_id = None;
        self.current_page = crate::pages::Page::Lobby;
    }

    /// Handles receiving the remote participant's name via RTP control message
    fn handle_remote_participant_name(&mut self, name: String) {
        if let Some(room_id) = &self.user_context.current_room_id
            && let Some(room) = self.current_room.as_mut()
        {
            let exists = room.participants.iter().any(|p| p.name == name);
            if !exists {
                self.logger.info(&format!(
                    "[ROOM] Participant '{}' joined room '{}'",
                    name, room_id
                ));
                let _ = room.add_participant(name.clone());
                self.show_success(format!("Participant '{}' joined", name));
            } else {
                self.logger.debug(&format!(
                    "[ROOM] Participant '{}' already in room '{}'",
                    name, room_id
                ));
            }
        }
    }

    /// Removes the other participant from room data (on disconnect)
    fn remove_other_participant(&mut self) {
        let username = match self.user_context.get_name() {
            Some(name) => name.to_string(),
            None => return,
        };

        if let Some(room) = self.get_current_room_mut() {
            // Remove the other participant (not the current user)
            room.participants.retain(|p| p.name == username);
        }
    }
}
