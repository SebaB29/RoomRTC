// Main logic thread coordinator

mod audio_thread;
mod camera_thread;
mod receive_thread;
mod state;
mod utils;
mod webrtc_handler;

use crate::events::{LogicCommand, LogicEvent};
use audio_thread::run_audio_thread;
use camera_thread::run_camera_thread;
use receive_thread::run_receive_thread;
use state::LogicState;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::{Arc, Mutex};
use webrtc_handler::{handle_generate_answer, handle_generate_offer, handle_process_answer};

/// Main function of the logic thread.
/// Receives `LogicCommand`s and sends `LogicEvent`s back to the UI thread.
pub fn run_logic_thread(
    cmd_rx: Receiver<LogicCommand>,
    evt_tx: Sender<LogicEvent>,
    logger: logging::Logger,
) {
    let mut state = LogicState::new();

    // Main loop: blocking wait for commands
    for command in cmd_rx {
        match command {
            LogicCommand::GenerateOffer => {
                handle_generate_offer(&mut state, &evt_tx);
            }

            LogicCommand::GenerateAnswer { offer_sdp } => {
                handle_generate_answer(&mut state, offer_sdp, &evt_tx);
            }

            LogicCommand::ProcessAnswer { answer_sdp } => {
                handle_process_answer(&mut state, answer_sdp, &evt_tx);
            }

            LogicCommand::AddIceCandidate {
                candidate,
                sdp_mid,
                sdp_mline_index,
            } => {
                handle_add_ice_candidate(&mut state, candidate, sdp_mid, sdp_mline_index, &evt_tx);
            }

            LogicCommand::StartConnection { participant } => {
                if let Some(conn) = state.pending_connection.take() {
                    handle_start_connection(conn, participant, &mut state, &evt_tx, logger.clone());
                } else {
                    let _ = evt_tx.send(LogicEvent::Error(
                        "No pending connection to start".to_string(),
                    ));
                }
            }

            LogicCommand::StartCamera { device_id, fps } => {
                handle_start_camera(device_id, fps, &state, &evt_tx);
            }

            LogicCommand::StopCamera => {
                handle_stop_camera(&state, &evt_tx);
            }

            LogicCommand::StartAudio {
                sample_rate,
                channels,
            } => {
                handle_start_audio(sample_rate, channels, &state, &evt_tx);
            }

            LogicCommand::ToggleMute => {
                handle_toggle_mute(&state, &evt_tx);
            }

            LogicCommand::ClearVideoBuffers => {
                handle_clear_video_buffers(&state);
            }

            LogicCommand::SendDisconnect { is_owner } => {
                handle_send_disconnect_message(&state.webrtc, &evt_tx, is_owner);
            }

            LogicCommand::StopConnection => {
                state.cleanup();
            }

            // --- File Transfer Commands ---
            LogicCommand::SendFile { path } => {
                if let Some(ref logger) = state.logger {
                    logger.info(&format!("[FILE] SendFile command received: {:?}", path));
                }

                // Validate file exists
                if !path.exists() {
                    let _ = evt_tx.send(LogicEvent::Error(format!("File not found: {:?}", path)));
                    continue;
                }

                if !path.is_file() {
                    let _ =
                        evt_tx.send(LogicEvent::Error("Selected path is not a file".to_string()));
                    continue;
                }

                // Get file info
                let filename = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();
                let file_size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);

                if let Some(ref logger) = state.logger {
                    logger.info(&format!(
                        "[FILE] File validated: {} ({} bytes)",
                        filename, file_size
                    ));
                }

                // Check if we have an active WebRTC connection
                if state.webrtc.is_none() {
                    let _ = evt_tx.send(LogicEvent::Error(
                        "Cannot send file: No active call".to_string(),
                    ));
                    continue;
                }

                // Send file through WebRTC connection
                let webrtc = state.webrtc.as_ref().unwrap();
                if let Ok(conn) = webrtc.lock() {
                    match conn.send_file(&path) {
                        Ok(transfer_id) => {
                            if let Some(ref logger) = state.logger {
                                logger.info(&format!(
                                    "[FILE] ✓ File transfer initiated: {} ({} bytes), transfer_id={}",
                                    filename, file_size, transfer_id
                                ));
                            }
                            // File transfer initiated successfully - no event needed, will be handled by SCTP
                        }
                        Err(e) => {
                            if let Some(ref logger) = state.logger {
                                logger.error(&format!("[FILE] Failed to send file: {}", e));
                            }
                            let _ = evt_tx
                                .send(LogicEvent::Error(format!("Failed to send file: {}", e)));
                        }
                    }
                } else {
                    let _ = evt_tx.send(LogicEvent::Error("WebRTC connection locked".to_string()));
                }
            }

            LogicCommand::AcceptFileTransfer {
                transfer_id,
                save_path,
            } => {
                if let Some(ref logger) = state.logger {
                    logger.info(&format!(
                        "[FILE] AcceptFileTransfer command: id={}, save_path={:?}",
                        transfer_id, save_path
                    ));
                }

                if let Some(ref webrtc) = state.webrtc {
                    if let Ok(conn) = webrtc.lock() {
                        match conn.accept_file_transfer(transfer_id, &save_path) {
                            Ok(_) => {
                                if let Some(ref logger) = state.logger {
                                    logger.info(&format!(
                                        "[FILE] ✓ File transfer accepted: id={}",
                                        transfer_id
                                    ));
                                }
                                let _ =
                                    evt_tx.send(LogicEvent::FileTransferAccepted { transfer_id });
                            }
                            Err(e) => {
                                if let Some(ref logger) = state.logger {
                                    logger.error(&format!("[FILE] Failed to accept file: {}", e));
                                }
                                let _ = evt_tx.send(LogicEvent::Error(format!(
                                    "Failed to accept file: {}",
                                    e
                                )));
                            }
                        }
                    }
                } else {
                    let _ = evt_tx.send(LogicEvent::Error("No active connection".to_string()));
                }
            }

            LogicCommand::RejectFileTransfer {
                transfer_id,
                reason,
            } => {
                if let Some(ref logger) = state.logger {
                    logger.info(&format!(
                        "[FILE] RejectFileTransfer command: id={}, reason={}",
                        transfer_id, reason
                    ));
                }

                if let Some(ref webrtc) = state.webrtc {
                    if let Ok(conn) = webrtc.lock() {
                        match conn.reject_file_transfer(transfer_id, &reason) {
                            Ok(_) => {
                                if let Some(ref logger) = state.logger {
                                    logger.info(&format!(
                                        "[FILE] ✓ File transfer rejected: id={}",
                                        transfer_id
                                    ));
                                }
                                let _ = evt_tx.send(LogicEvent::FileTransferRejected {
                                    transfer_id,
                                    reason: reason.clone(),
                                });
                            }
                            Err(e) => {
                                if let Some(ref logger) = state.logger {
                                    logger.error(&format!("[FILE] Failed to reject file: {}", e));
                                }
                                let _ = evt_tx.send(LogicEvent::Error(format!(
                                    "Failed to reject file: {}",
                                    e
                                )));
                            }
                        }
                    }
                } else {
                    let _ = evt_tx.send(LogicEvent::Error("No active connection".to_string()));
                }
            }

            LogicCommand::CancelFileTransfer { transfer_id } => {
                if let Some(ref logger) = state.logger {
                    logger.info(&format!(
                        "[FILE] CancelFileTransfer command: id={}",
                        transfer_id
                    ));
                }

                if let Some(ref webrtc) = state.webrtc {
                    if let Ok(conn) = webrtc.lock() {
                        match conn.cancel_file_transfer(transfer_id, "User cancelled") {
                            Ok(_) => {
                                if let Some(ref logger) = state.logger {
                                    logger.info(&format!(
                                        "[FILE] ✓ File transfer cancelled: id={}",
                                        transfer_id
                                    ));
                                }
                                let _ = evt_tx.send(LogicEvent::FileTransferFailed {
                                    transfer_id,
                                    reason: "User cancelled".to_string(),
                                });
                            }
                            Err(e) => {
                                if let Some(ref logger) = state.logger {
                                    logger.error(&format!("[FILE] Failed to cancel file: {}", e));
                                }
                                let _ = evt_tx.send(LogicEvent::Error(format!(
                                    "Failed to cancel file: {}",
                                    e
                                )));
                            }
                        }
                    }
                } else {
                    let _ = evt_tx.send(LogicEvent::Error("No active connection".to_string()));
                }
            }
        }
    }

    // Cleanup on exit
    state.cleanup();
}

/// Establishes the WebRTC connection and spawns media threads for camera and frame reception.
fn handle_start_connection(
    mut conn: webrtc::WebRtcConnection,
    participant: crate::models::Participant,
    state: &mut LogicState,
    evt_tx: &Sender<LogicEvent>,
    logger: logging::Logger,
) {
    logger.info(&format!(
        "[LOGIC] Starting connection for participant '{}'",
        participant.name
    ));

    if let Err(e) = conn.establish_connection() {
        logger.error(&format!(
            "[LOGIC] Failed to establish connection for '{}': {}",
            participant.name, e
        ));
        let _ = evt_tx.send(LogicEvent::Error(format!(
            "Failed to establish connection: {}",
            e
        )));
        return;
    }

    logger.info(&format!(
        "[LOGIC] Connection established successfully for '{}', sending participant name",
        participant.name
    ));

    if let Err(e) = conn.send_participant_name(&participant.name) {
        logger.error(&format!(
            "[LOGIC] Failed to send participant name for '{}': {}",
            participant.name, e
        ));
        let _ = evt_tx.send(LogicEvent::Error(format!(
            "Failed to send participant name: {}",
            e
        )));
    }

    logger.info(&format!(
        "[LOGIC] Spawning media threads for '{}'",
        participant.name
    ));
    spawn_media_threads(conn, &participant.name, state, evt_tx, logger.clone());

    if participant.camera_on {
        logger.info(&format!(
            "[LOGIC] Auto-starting camera for '{}' (camera_on=true)",
            participant.name
        ));
        start_participant_camera_async(&participant, &state.webrtc, evt_tx);
    } else {
        logger.info(&format!(
            "[LOGIC] Camera not started for '{}' (camera_on=false)",
            participant.name
        ));
    }
}

/// Starts the camera for a participant asynchronously.
///
/// The camera is started in a separate thread to avoid blocking the main logic thread.
/// If the camera fails to start, the connection remains active and the camera can be
/// toggled manually from the UI.
fn start_participant_camera_async(
    participant: &crate::models::Participant,
    webrtc_opt: &Option<Arc<Mutex<webrtc::WebRtcConnection>>>,
    evt_tx: &Sender<LogicEvent>,
) {
    let Some(webrtc_arc) = webrtc_opt.as_ref() else {
        return;
    };

    let webrtc = webrtc_arc.clone();
    let device_id = participant.selected_camera_device;
    let fps = participant.camera_fps;
    let tx = evt_tx.clone();

    // Start camera in a separate thread to avoid blocking
    std::thread::spawn(move || {
        // Small delay to ensure connection is fully ready
        std::thread::sleep(std::time::Duration::from_millis(100));

        let result = {
            let mut conn = match webrtc.lock() {
                Ok(guard) => guard,
                Err(poisoned) => poisoned.into_inner(),
            };
            conn.start_camera(device_id, fps)
        };

        match result {
            Ok(_) => {
                let _ = tx.send(LogicEvent::CameraStarted);
            }
            Err(e) => {
                let _ = tx.send(LogicEvent::Error(format!(
                    "Camera failed to start: {}. You can toggle it manually.",
                    e
                )));
            }
        }
    });
}

/// Spawns camera capture and frame receiver threads
fn spawn_media_threads(
    conn: webrtc::WebRtcConnection,
    _participant_name: &str,
    state: &mut LogicState,
    evt_tx: &Sender<LogicEvent>,
    logger: logging::Logger,
) {
    let webrtc_arc = Arc::new(Mutex::new(conn));
    state.webrtc = Some(webrtc_arc.clone());

    // Start camera capture thread
    let camera_handle = std::thread::spawn({
        let webrtc = webrtc_arc.clone();
        let tx = evt_tx.clone();
        move || run_camera_thread(webrtc, tx)
    });
    state.camera_thread_handle = Some(camera_handle);

    // Start audio capture thread
    let audio_handle = std::thread::spawn({
        let webrtc = webrtc_arc.clone();
        move || run_audio_thread(webrtc)
    });
    state.audio_thread_handle = Some(audio_handle);

    // Start remote frame receiver thread
    let receive_handle = std::thread::spawn({
        let webrtc = webrtc_arc.clone();
        let tx = evt_tx.clone();
        let logger_clone = logger
            .for_component("ReceiveThread")
            .unwrap_or(logger.clone());
        move || run_receive_thread(webrtc, tx, logger_clone)
    });
    state.receive_thread_handle = Some(receive_handle);
}

/// Executes camera operations with the WebRTC connection in a separate thread.
fn handle_start_camera(device_id: i32, fps: f64, state: &LogicState, evt_tx: &Sender<LogicEvent>) {
    execute_with_webrtc(state, evt_tx.clone(), move |webrtc| {
        match webrtc.start_camera(device_id, fps) {
            Ok(_) => Some(LogicEvent::CameraStarted),
            Err(e) => Some(LogicEvent::Error(format!("Camera error: {}", e))),
        }
    });
}

/// Stops the camera and sends a notification event.
fn handle_stop_camera(state: &LogicState, evt_tx: &Sender<LogicEvent>) {
    execute_with_webrtc(state, evt_tx.clone(), |webrtc| {
        webrtc.stop_camera();
        Some(LogicEvent::CameraStopped)
    });
}

/// Starts audio capture with the WebRTC connection
fn handle_start_audio(
    sample_rate: u32,
    channels: u32,
    state: &LogicState,
    evt_tx: &Sender<LogicEvent>,
) {
    execute_with_webrtc(state, evt_tx.clone(), move |webrtc| {
        match webrtc.start_audio_auto(sample_rate, channels) {
            Ok(_) => Some(LogicEvent::AudioStarted),
            Err(e) => Some(LogicEvent::Error(format!("Audio error: {}", e))),
        }
    });
}

/// Toggles mute state
fn handle_toggle_mute(state: &LogicState, evt_tx: &Sender<LogicEvent>) {
    execute_with_webrtc(state, evt_tx.clone(), |webrtc| match webrtc.toggle_mute() {
        Ok(is_muted) => {
            if is_muted {
                Some(LogicEvent::AudioMuted)
            } else {
                Some(LogicEvent::AudioUnmuted)
            }
        }
        Err(e) => Some(LogicEvent::Error(format!("Mute toggle error: {}", e))),
    });
}

/// Clears video buffers to flush delayed frames from jitter buffer
fn handle_clear_video_buffers(state: &LogicState) {
    if let Some(webrtc_arc) = &state.webrtc
        && let Ok(webrtc) = webrtc_arc.lock()
    {
        webrtc.clear_video_buffers();
    }
}

/// Executes an operation with the WebRTC connection in a separate thread.
///
/// This prevents blocking the main logic thread while camera operations are in progress.
fn execute_with_webrtc<F>(state: &LogicState, evt_tx: Sender<LogicEvent>, operation: F)
where
    F: FnOnce(&mut webrtc::WebRtcConnection) -> Option<LogicEvent> + Send + 'static,
{
    if let Some(webrtc_arc) = state.webrtc.clone() {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(50));
            if let Ok(mut webrtc) = webrtc_arc.lock()
                && let Some(event) = operation(&mut webrtc)
            {
                let _ = evt_tx.send(event);
            }
        });
    } else {
        let _ = evt_tx.send(LogicEvent::Error(
            "Cannot execute operation: no WebRTC connection".to_string(),
        ));
    }
}

/// Sends a disconnect message through the WebRTC connection.
fn handle_send_disconnect_message(
    webrtc_state: &Option<Arc<Mutex<webrtc::WebRtcConnection>>>,
    evt_tx: &Sender<LogicEvent>,
    is_owner: bool,
) {
    if let Some(webrtc_arc) = webrtc_state
        && let Ok(conn) = webrtc_arc.lock()
        && let Err(e) = conn.send_disconnect_message(is_owner)
    {
        let _ = evt_tx.send(LogicEvent::Error(format!(
            "Failed to send disconnect message: {}",
            e
        )));
    }
}

/// Adds a remote ICE candidate to the pending or active WebRTC connection.
fn handle_add_ice_candidate(
    state: &mut LogicState,
    candidate: String,
    sdp_mid: String,
    sdp_mline_index: u16,
    evt_tx: &Sender<LogicEvent>,
) {
    // Try pending connection first (during setup)
    if let Some(ref mut conn) = state.pending_connection {
        if let Err(e) = conn.add_ice_candidate(&candidate, &sdp_mid, sdp_mline_index) {
            let _ = evt_tx.send(LogicEvent::Error(format!(
                "Failed to add ICE candidate to pending connection: {}",
                e
            )));
        }
        return;
    }

    // Try active connection
    if let Some(webrtc_arc) = &state.webrtc {
        if let Ok(mut conn) = webrtc_arc.lock()
            && let Err(e) = conn.add_ice_candidate(&candidate, &sdp_mid, sdp_mline_index)
        {
            let _ = evt_tx.send(LogicEvent::Error(format!(
                "Failed to add ICE candidate to active connection: {}",
                e
            )));
        }
    } else {
        let _ = evt_tx.send(LogicEvent::Error(
            "Cannot add ICE candidate: no WebRTC connection available".to_string(),
        ));
    }
}
