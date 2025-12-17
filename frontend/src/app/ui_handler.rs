//! UI Command Handler
//!
//! This module dispatches UI commands to specialized handlers.
//! Implementation details are split into domain-specific modules:
//! - room_handlers: Room creation, joining, exit
//! - webrtc_handlers: SDP offer/answer, connection setup
//! - camera_handlers: Camera toggling and settings

use super::state::App;
use crate::events::{LogicCommand, UiCommand};
use std::path::PathBuf;

impl App {
    /// Dispatches UI commands to appropriate handlers
    /// This is the main entry point for all UI actions
    pub(super) fn handle_ui_command(&mut self, command: UiCommand) {
        self.logger
            .debug(&format!("[UI] Handling command: {:?}", command));
        match command {
            // Authentication
            UiCommand::LoginWithPassword { username, password } => {
                self.handle_login_with_password(username, password)
            }
            UiCommand::Register { username, password } => self.handle_register(username, password),
            UiCommand::Logout => self.handle_logout(),

            // Lobby operations
            UiCommand::CallUser(to_user_id) => self.handle_call_user(to_user_id),
            UiCommand::AcceptCall {
                call_id,
                caller_id,
                caller_name,
            } => self.handle_accept_call(call_id, caller_id, caller_name),
            UiCommand::DeclineCall(call_id) => self.handle_decline_call(call_id),

            // Camera operations
            UiCommand::ToggleCamera => self.handle_toggle_camera(),
            UiCommand::ToggleMute => self.handle_toggle_mute(),
            UiCommand::UpdateCameraSettings(device_id, fps) => {
                self.handle_update_camera_settings(device_id, fps)
            }

            // Room management
            UiCommand::ExitRoom => self.handle_exit_room(),

            // File transfer
            UiCommand::SendFile => self.handle_send_file(),
            UiCommand::SendFileSelected(path) => self.handle_send_file_selected(path),
            UiCommand::AcceptFileTransfer {
                transfer_id,
                save_path,
            } => self.handle_accept_file_transfer(transfer_id, save_path),
            UiCommand::RejectFileTransfer { transfer_id } => {
                self.handle_reject_file_transfer(transfer_id)
            }
            UiCommand::CancelFileTransfer { transfer_id } => {
                self.handle_cancel_file_transfer(transfer_id)
            }
        }
    }

    /// Opens file send dialog (sets state for modal)
    fn handle_send_file(&mut self) {
        self.logger.info("[FILE] Opening file send dialog...");
        self.file_send_dialog_open = true;
        self.file_send_path_input.clear();
    }

    /// Handles file path submitted from dialog
    fn handle_send_file_selected(&mut self, path: PathBuf) {
        self.logger
            .info(&format!("[FILE] Sending file: {:?}", path));

        // Validate path exists
        if !path.exists() {
            self.show_error(format!("File not found: {:?}", path));
            return;
        }

        if !path.is_file() {
            self.show_error("Path is not a file".to_string());
            return;
        }

        // Extract filename and size for tracking
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

        self.pending_send_file = Some((filename, size));

        let _ = self.logic_cmd_tx.send(LogicCommand::SendFile { path });
        self.show_success("Sending file...".to_string());
        self.file_send_dialog_open = false;
    }

    /// Accepts an incoming file transfer
    fn handle_accept_file_transfer(&mut self, transfer_id: u64, save_path: PathBuf) {
        self.logger.info(&format!(
            "[FILE] Accepting transfer {} - save to: {:?}",
            transfer_id, save_path
        ));

        // Get filename and size from incoming offer BEFORE clearing it
        let (filename, size) = if let Some(offer) = &self.incoming_file_offer {
            (offer.filename.clone(), offer.size)
        } else {
            self.logger
                .warn("[FILE] Accept called but no incoming_file_offer!");
            ("unknown".to_string(), 0)
        };

        self.logger.info(&format!(
            "[FILE] Creating active_file_transfer for receiver - id: {}, file: {}, size: {}",
            transfer_id, filename, size
        ));

        // Start tracking the transfer
        self.active_file_transfer = Some(crate::app::state::ActiveFileTransfer {
            transfer_id,
            filename,
            total_size: size,
            bytes_transferred: 0,
            is_sending: false,
        });

        // Clear the incoming offer
        self.incoming_file_offer = None;

        let _ = self.logic_cmd_tx.send(LogicCommand::AcceptFileTransfer {
            transfer_id,
            save_path,
        });

        self.logger.info(&format!(
            "[FILE] AcceptFileTransfer command sent for id: {}",
            transfer_id
        ));
    }

    /// Rejects an incoming file transfer
    fn handle_reject_file_transfer(&mut self, transfer_id: u64) {
        self.logger
            .info(&format!("[FILE] Rejecting transfer {}", transfer_id));
        let _ = self.logic_cmd_tx.send(LogicCommand::RejectFileTransfer {
            transfer_id,
            reason: "User declined".to_string(),
        });
    }

    /// Cancels an active file transfer
    fn handle_cancel_file_transfer(&mut self, transfer_id: u64) {
        self.logger
            .info(&format!("[FILE] Cancelling transfer {}", transfer_id));
        let _ = self
            .logic_cmd_tx
            .send(LogicCommand::CancelFileTransfer { transfer_id });
        // Clear all transfer state immediately
        self.active_file_transfer = None;
        self.pending_send_file = None;
        self.incoming_file_offer = None;
        self.logger
            .debug("[FILE] Transfer state cleared after cancel");
    }
}
