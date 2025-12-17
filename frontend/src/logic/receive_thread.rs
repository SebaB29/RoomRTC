//! Remote Frame Reception Thread
//!
//! This module handles continuous reception of video frames from the remote peer.
//! Frames are received via RTP/UDP, decoded, and sent to the UI for display.

use crate::components::CallStats;
use crate::events::LogicEvent;
use crate::logic::utils::rgb_to_color_image;
use logging::Logger;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use webrtc::WebRtcConnection;

const FRAME_RECEIVE_INTERVAL_MS: u64 = 1;
const ERROR_RETRY_INTERVAL_MS: u64 = 100;
const STATS_UPDATE_INTERVAL_MS: u64 = 1000;

/// Tracks bitrate over a sliding time window
struct BitrateTracker {
    last_bytes_sent: u64,
    last_update: Instant,
}

impl BitrateTracker {
    fn new() -> Self {
        Self {
            last_bytes_sent: 0,
            last_update: Instant::now(),
        }
    }

    /// Calculate bitrate in Mbps based on bytes sent since last call
    fn calculate_bitrate(&mut self, current_bytes_sent: u64) -> f64 {
        let elapsed = self.last_update.elapsed().as_secs_f64();

        if elapsed >= 1.0 {
            let bytes_delta = current_bytes_sent.saturating_sub(self.last_bytes_sent);
            let mbps = (bytes_delta as f64 * 8.0) / elapsed / 1_000_000.0;

            self.last_bytes_sent = current_bytes_sent;
            self.last_update = Instant::now();

            mbps
        } else {
            // Not enough time elapsed
            0.0
        }
    }
}

/// Runs the frame reception loop in a dedicated thread.
///
/// The thread continuously receives frames from the remote peer through the WebRTC
/// connection and sends them to the UI for display. Also polls for control messages
/// to synchronize camera state between peers.
pub fn run_receive_thread(
    webrtc_arc: Arc<Mutex<WebRtcConnection>>,
    evt_tx: Sender<LogicEvent>,
    logger: Logger,
) {
    let mut last_stats_update = Instant::now();
    let mut bitrate_tracker = BitrateTracker::new();

    loop {
        poll_control_messages(&webrtc_arc, &evt_tx, &logger);
        poll_video_frames(&webrtc_arc, &evt_tx, &logger);
        poll_audio_frames(&webrtc_arc, &logger);
        poll_sctp(&webrtc_arc, &evt_tx, &logger);

        // Poll stats periodically
        if last_stats_update.elapsed() >= Duration::from_millis(STATS_UPDATE_INTERVAL_MS) {
            poll_statistics(&webrtc_arc, &evt_tx, &mut bitrate_tracker, &logger);
            last_stats_update = Instant::now();
        }
    }
}

/// Polls for control messages from the remote peer
fn poll_control_messages(
    webrtc_arc: &Arc<Mutex<WebRtcConnection>>,
    evt_tx: &Sender<LogicEvent>,
    logger: &Logger,
) {
    let control_result = {
        let conn = match webrtc_arc.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                logger.error("WebRTC mutex poisoned in poll_control_messages, recovering");
                poisoned.into_inner()
            }
        };
        conn.receive_control_message()
    };

    if let Ok(Some(control_msg)) = control_result {
        let event = match control_msg {
            webrtc::ControlMessage::CameraOn => LogicEvent::RemoteCameraOn,
            webrtc::ControlMessage::CameraOff => LogicEvent::RemoteCameraOff,
            webrtc::ControlMessage::AudioOn => LogicEvent::RemoteAudioOn,
            webrtc::ControlMessage::AudioOff => LogicEvent::RemoteAudioOff,
            webrtc::ControlMessage::AudioMuted => LogicEvent::RemoteAudioMuted,
            webrtc::ControlMessage::AudioUnmuted => LogicEvent::RemoteAudioUnmuted,
            webrtc::ControlMessage::ParticipantDisconnected => LogicEvent::ParticipantDisconnected,
            webrtc::ControlMessage::OwnerDisconnected => LogicEvent::OwnerDisconnected,
            webrtc::ControlMessage::ParticipantName(name) => {
                LogicEvent::RemoteParticipantName(name)
            }
        };
        let _ = evt_tx.send(event);
    }
}

/// Polls for video frames from the remote peer
fn poll_video_frames(
    webrtc_arc: &Arc<Mutex<WebRtcConnection>>,
    evt_tx: &Sender<LogicEvent>,
    logger: &Logger,
) {
    let result = {
        let conn = match webrtc_arc.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                logger.error("WebRTC mutex poisoned in poll_video_frames, recovering");
                poisoned.into_inner()
            }
        };
        conn.receive_frame()
    };

    match result {
        Ok(Some((width, height, rgb_pixels))) => {
            let color_image = rgb_to_color_image(width, height, rgb_pixels);
            let _ = evt_tx.send(LogicEvent::RemoteFrame(color_image));
        }
        Ok(None) => {
            std::thread::sleep(std::time::Duration::from_millis(FRAME_RECEIVE_INTERVAL_MS));
        }
        Err(_) => {
            std::thread::sleep(std::time::Duration::from_millis(ERROR_RETRY_INTERVAL_MS));
        }
    }
}

/// Polls statistics from WebRTC connection
fn poll_statistics(
    webrtc_arc: &Arc<Mutex<WebRtcConnection>>,
    evt_tx: &Sender<LogicEvent>,
    bitrate_tracker: &mut BitrateTracker,
    logger: &Logger,
) {
    let conn = match webrtc_arc.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            logger.error("WebRTC mutex poisoned in poll_statistics, recovering");
            poisoned.into_inner()
        }
    };

    let packet_stats = conn.get_packet_stats();
    let jitter_stats = conn.get_jitter_stats();

    // Calculate bitrate from RTCP stats
    let bitrate_mbps = if let Some(rtcp_stats) = conn.get_rtcp_stats() {
        bitrate_tracker.calculate_bitrate(rtcp_stats.bytes_sent)
    } else {
        0.0
    };

    let packets_sent = conn.get_rtcp_stats().map(|s| s.packets_sent).unwrap_or(0);

    // Calculate RTT from RTCP (convert seconds to milliseconds)
    let rtt_ms = conn
        .get_rtcp_stats()
        .and_then(|s| s.calculate_rtt())
        .map(|rtt_secs| rtt_secs * 1000.0)
        .unwrap_or(0.0);

    let stats = CallStats {
        bitrate_mbps,
        packet_loss_percent: packet_stats.loss_rate * 100.0,
        jitter_ms: jitter_stats.jitter_ms,
        rtt_ms,
        packets_sent,
        packets_received: packet_stats.packets_received as u32,
    };

    let _ = evt_tx.send(LogicEvent::StatsUpdated(stats));
}

/// Polls for outgoing SCTP packets and processes incoming SCTP data
fn poll_sctp(
    webrtc_arc: &Arc<Mutex<WebRtcConnection>>,
    evt_tx: &std::sync::mpsc::Sender<LogicEvent>,
    logger: &Logger,
) {
    let mut conn = match webrtc_arc.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            logger.error("WebRTC mutex poisoned in poll_sctp, recovering");
            poisoned.into_inner()
        }
    };

    // Poll for outgoing SCTP packets log if it's not a "no data" type error
    if let Err(e) = conn.poll_sctp()
        && !e.to_string().contains("would block") {
            logger.debug(&format!("SCTP poll send: {}", e));
        }

    if let Err(e) = conn.receive_sctp() {
        let err_str = e.to_string();
        if !err_str.contains("would block") && !err_str.contains("Empty") {
            logger.debug(&format!("SCTP receive: {}", err_str));
        }
    }

    if conn.check_and_emit_file_channel_ready() {
        logger.info("[FILE] File channel ready - emitting FileChannelReady event");
        let _ = evt_tx.send(LogicEvent::FileChannelReady);
    }

    // Poll for file transfer events
    while let Some(event) = conn.poll_file_event() {
        use webrtc::FileTransferEvent;
        match event {
            FileTransferEvent::IncomingOffer { id, filename, size } => {
                logger.info(&format!(
                    "Incoming file offer: {} ({} bytes)",
                    filename, size
                ));
                let _ = evt_tx.send(LogicEvent::FileOfferReceived {
                    transfer_id: id,
                    filename,
                    size,
                });
            }
            FileTransferEvent::Accepted { id } => {
                logger.info(&format!("File transfer {} accepted", id));
                let _ = evt_tx.send(LogicEvent::FileTransferAccepted { transfer_id: id });
            }
            FileTransferEvent::Rejected { id, reason } => {
                logger.info(&format!("File transfer {} rejected: {}", id, reason));
                let _ = evt_tx.send(LogicEvent::FileTransferRejected {
                    transfer_id: id,
                    reason,
                });
            }
            FileTransferEvent::Progress { id, bytes, total } => {
                let _ = evt_tx.send(LogicEvent::FileTransferProgress {
                    transfer_id: id,
                    bytes_transferred: bytes,
                    total_bytes: total,
                });
            }
            FileTransferEvent::Completed { id, path } => {
                logger.info(&format!("File transfer {} completed: {:?}", id, path));
                let _ = evt_tx.send(LogicEvent::FileTransferCompleted {
                    transfer_id: id,
                    path,
                });
            }
            FileTransferEvent::Failed { id, reason } => {
                logger.error(&format!("File transfer {} failed: {}", id, reason));
                let _ = evt_tx.send(LogicEvent::FileTransferFailed {
                    transfer_id: id,
                    reason,
                });
            }
        }
    }
}

/// Polls for audio frames from the remote peer and plays them
fn poll_audio_frames(webrtc_arc: &Arc<Mutex<WebRtcConnection>>, logger: &Logger) {
    let result = {
        let mut conn = match webrtc_arc.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                logger.error("WebRTC mutex poisoned in poll_audio_frames, recovering");
                poisoned.into_inner()
            }
        };
        conn.receive_audio()
    };

    if let Err(e) = result {
        // Only log serious errors, not "no data available"
        if !e.to_string().contains("No data") {
            logger.debug(&format!("Audio receive error: {}", e));
        }
    }
}
