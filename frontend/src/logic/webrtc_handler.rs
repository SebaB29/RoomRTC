//! WebRTC connection setup and SDP exchange.

use crate::events::LogicEvent;
use crate::logic::state::LogicState;
use logging::LogLevel;
use std::sync::mpsc::Sender;
use webrtc::WebRtcConnection;

/// Helper to send error events
fn send_error(evt_tx: &Sender<LogicEvent>, message: String) {
    let _ = evt_tx.send(LogicEvent::Error(message));
}

/// Generate WebRTC offer
pub fn handle_generate_offer(state: &mut LogicState, evt_tx: &Sender<LogicEvent>) {
    let logger = match logging::Logger::with_component(
        "room_setup.log".into(),
        LogLevel::Info,
        "WebRTC-Offer".to_string(),
        false,
    ) {
        Ok(l) => l,
        Err(e) => return send_error(evt_tx, format!("Error creating logger: {}", e)),
    };

    match WebRtcConnection::create_offer_from_new(logger) {
        Ok((conn, offer)) => {
            // Store connection in LogicState temporarily
            state.pending_connection = Some(conn);
            let _ = evt_tx.send(LogicEvent::OfferGenerated(offer));
        }
        Err(e) => send_error(evt_tx, format!("Error creating offer: {}", e)),
    }
}

/// Generate WebRTC answer
pub fn handle_generate_answer(
    state: &mut LogicState,
    offer_sdp: String,
    evt_tx: &Sender<LogicEvent>,
) {
    let logger = match logging::Logger::with_component(
        "room_setup.log".into(),
        LogLevel::Info,
        "WebRTC-Answer".to_string(),
        false,
    ) {
        Ok(l) => l,
        Err(e) => return send_error(evt_tx, format!("Error creating logger: {}", e)),
    };

    logger.info("[WEBRTC] Creating answer from offer SDP");
    match WebRtcConnection::create_answer_from_new(&offer_sdp, logger.clone()) {
        Ok((conn, answer)) => {
            logger.info(&format!(
                "[WEBRTC] Answer created successfully - sdp_len: {} bytes",
                answer.len()
            ));
            // Store connection in LogicState temporarily
            state.pending_connection = Some(conn);
            let _ = evt_tx.send(LogicEvent::AnswerGenerated(answer));
        }
        Err(e) => {
            logger.error(&format!("[WEBRTC] Failed to create answer: {}", e));
            send_error(evt_tx, format!("Error creating answer: {}", e));
        }
    }
}

/// Process answer from remote peer
pub fn handle_process_answer(
    state: &mut LogicState,
    answer_sdp: String,
    evt_tx: &Sender<LogicEvent>,
) {
    // Get pending connection from state
    let Some(mut conn) = state.pending_connection.take() else {
        return send_error(
            evt_tx,
            "No pending connection to process answer".to_string(),
        );
    };

    match conn.set_remote_answer(&answer_sdp) {
        Ok(_) => {
            // Put connection back in pending state until StartConnection
            state.pending_connection = Some(conn);
            let _ = evt_tx.send(LogicEvent::ConnectionReady);
        }
        Err(e) => {
            send_error(evt_tx, format!("Error setting remote answer: {}", e));
        }
    }
}
