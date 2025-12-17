//! Receive thread functionality for secure P2P session

use super::control_message::ControlMessage;
use logging::Logger;
use media::{AudioFrame, OpusDecoder};
use network::codec::rtp::control_payload;
use network::{
    JitterBuffer, OpusRtpDepacketizer, PacketHandler,
    SecureUdpTransport,
};
use std::sync::{
    Arc, Mutex,
    mpsc::{Sender, SyncSender},
};
use std::thread;
use std::time::Duration;

/// Parameters for receive thread
pub struct RecvThreadParams {
    pub audio_decoder: Arc<Mutex<OpusDecoder>>,
    pub transport: Arc<Mutex<Option<SecureUdpTransport>>>,
    pub jitter_buffer: Arc<Mutex<JitterBuffer>>,
    pub packet_handler: Arc<Mutex<PacketHandler>>,
    pub audio_packet_handler: Arc<Mutex<PacketHandler>>,
    pub tx_audio_decode: SyncSender<AudioFrame>,
    pub tx_control: Sender<ControlMessage>,
    pub logger: Logger,
    pub dtls_engine: Arc<Mutex<Option<network::security::dtls::DtlsEngine>>>,
    pub file_session: Arc<Mutex<Option<super::file_session::FileSession>>>,
}

struct RecvThreadState {
    packets_received: u64,
    packets_released_from_buffer: u64,
    frames_decoded: u64,
    audio_packets_received: u64,
    audio_frames_decoded: u64,
}

pub(super) fn run_recv_thread(params: RecvThreadParams) {
    params
        .logger
        .info("Secure RECV thread started (video + audio)");

    let mut audio_depacketizer = OpusRtpDepacketizer::new();
    let mut state = RecvThreadState {
        packets_received: 0,
        packets_released_from_buffer: 0,
        frames_decoded: 0,
        audio_packets_received: 0,
        audio_frames_decoded: 0,
    };

    loop {
        // Batch receive multiple packets to reduce overhead
        let mut packets_this_batch = 0;
        const MAX_BATCH: usize = 32;

        for _ in 0..MAX_BATCH {
            match receive_packet(&params) {
                Ok(Some(packet)) => {
                    if handle_control_message(&packet, &params) {
                        continue;
                    }

                    // Distinguish between video (96) and audio (111) based on payload type
                    if packet.header.payload_type == 111 {
                        // Audio packet
                        state.audio_packets_received += 1;

                        track_packet_stats(
                            &params.audio_packet_handler,
                            packet.header.sequence_number,
                            &params.logger,
                            "Audio",
                        );

                        process_audio_packet(&packet, &params, &mut audio_depacketizer, &mut state);
                    } else {
                        // Video packet
                        state.packets_received += 1;
                        packets_this_batch += 1;
                        log_packet_received(&params.logger, &state);

                        track_packet_stats(
                            &params.packet_handler,
                            packet.header.sequence_number,
                            &params.logger,
                            "Video",
                        );
                        add_to_jitter_buffer(&params.jitter_buffer, packet, &params.logger);
                    }
                }
                Ok(None) => {
                    break;
                }
                Err(e) => {
                    params
                        .logger
                        .error(&format!("Failed to receive encrypted packet: {}", e));
                    break;
                }
            }
        }

        if packets_this_batch == 0 {
            // Only sleep if no packets were received
            thread::sleep(Duration::from_micros(100));
        }

        if let Err(e) = process_incoming_sctp(&params) {
            params
                .logger
                .error(&format!("Failed to process SCTP: {}", e));
        }
    }
}
/// Process incoming SCTP packets from DTLS engine (integrated into existing recv loop)
fn process_incoming_sctp(params: &RecvThreadParams) -> Result<(), network::NetworkError> {
    const MAX_SCTP_BATCH: usize = 16; // Process up to 16 SCTP packets per iteration

    for _ in 0..MAX_SCTP_BATCH {
        let mut transport_guard = params.transport.lock().unwrap_or_else(|poisoned| {
            params
                .logger
                .error("Transport mutex poisoned in process_incoming_sctp");
            poisoned.into_inner()
        });

        let dtls_packet = if let Some(transport) = transport_guard.as_mut() {
            match transport.receive_dtls_packet() {
                Ok(Some(packet)) => packet,
                _ => return Ok(()),
            }
        } else {
            return Ok(());
        };

        drop(transport_guard);

        // Feed to DTLS engine for decryption
        let mut dtls_guard = params.dtls_engine.lock().unwrap();
        if let Some(dtls_engine) = dtls_guard.as_mut() {
            if let Err(e) = dtls_engine.handle_packet(&dtls_packet) {
                params
                    .logger
                    .error(&format!("Failed to process DTLS packet: {}", e));
                continue;
            }

            let sctp_packets = dtls_engine.take_incoming_sctp();
            drop(dtls_guard);

            for sctp_data in sctp_packets {
                let mut file_session_guard = params.file_session.lock().unwrap();
                if let Some(file_session) = file_session_guard.as_mut() {
                    match file_session.on_sctp_data(&sctp_data) {
                        Ok(responses) if !responses.is_empty() => {
                            drop(file_session_guard);
                            let mut dtls_guard2 = params.dtls_engine.lock().unwrap();
                            if let Some(dtls_eng) = dtls_guard2.as_mut() {
                                for response_bytes in responses {
                                    if let Err(e) = dtls_eng.send_application_data(&response_bytes)
                                    {
                                        params
                                            .logger
                                            .error(&format!("Failed to send SCTP response: {}", e));
                                    } else {
                                        let packets = dtls_eng.take_pending_packets();
                                        let mut transport_guard2 = params.transport.lock().unwrap();
                                        if let Some(transport) = transport_guard2.as_mut() {
                                            let remote_addr = transport.remote_addr();
                                            for packet in packets {
                                                let _ = transport
                                                    .socket()
                                                    .send_to(&packet, remote_addr);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Ok(_) => {}
                        Err(e) => params
                            .logger
                            .error(&format!("SCTP processing error: {}", e)),
                    }
                }
            }
        }
    }

    Ok(())
}
fn receive_packet(
    params: &RecvThreadParams,
) -> Result<Option<network::codec::rtp::RtpPacket>, network::NetworkError> {
    let mut transport_guard = params.transport.lock().unwrap_or_else(|poisoned| {
        params
            .logger
            .error("Transport mutex poisoned in receive thread, recovering");
        poisoned.into_inner()
    });

    if let Some(transport) = transport_guard.as_mut() {
        // First, check if there are buffered RTP packets (from previous unified_receive calls)
        if let Some(rtp_packet) = transport.receive_rtp()? {
            return Ok(Some(rtp_packet));
        }

        // Try to receive one packet from network
        match transport.unified_receive() {
            Ok(true) => {
                // RTP packet was processed and buffered - get it
                transport
                    .receive_rtp()
                    .map_err(network::NetworkError::Media)
            }
            Ok(false) => {
                // No RTP (was DTLS peeked, RTCP consumed, or socket empty) - yield
                Ok(None)
            }
            Err(e) => Err(network::NetworkError::Media(e)),
        }
    } else {
        params
            .logger
            .error("Transport not initialized - DTLS handshake required");
        thread::sleep(Duration::from_millis(100));
        Ok(None)
    }
}

fn handle_control_message(
    packet: &network::codec::rtp::RtpPacket,
    params: &RecvThreadParams,
) -> bool {
    if packet.header.payload_type == control_payload::CONTROL {
        if let Some(control_msg) = ControlMessage::from_bytes(&packet.payload) {
            params.logger.info(&format!(
                "Received control message via RTP: {:?}",
                control_msg
            ));

            // Handle CameraOff: reset SRTP replay protection and clear video buffers
            if matches!(control_msg, ControlMessage::CameraOff) {
                params
                    .logger
                    .info("Resetting SRTP replay protection and clearing buffers due to CameraOff");

                // Reset SRTP replay window
                if let Ok(mut transport_guard) = params.transport.lock()
                    && let Some(transport) = transport_guard.as_mut() {
                        transport.reset_srtp_receiver();
                    }

                // Clear jitter buffer to flush stale packets
                params
                    .jitter_buffer
                    .lock()
                    .unwrap_or_else(|poisoned| {
                        params
                            .logger
                            .error("Jitter buffer mutex poisoned, recovering");
                        poisoned.into_inner()
                    })
                    .clear();

                // Clear packet handler statistics
                params
                    .packet_handler
                    .lock()
                    .unwrap_or_else(|poisoned| {
                        params
                            .logger
                            .error("Packet handler mutex poisoned, recovering");
                        poisoned.into_inner()
                    })
                    .clear();
            }

            if let Err(e) = params.tx_control.send(control_msg) {
                params
                    .logger
                    .error(&format!("Failed to forward control message: {}", e));
            }
        }
        return true;
    }
    false
}

fn log_packet_received(logger: &Logger, state: &RecvThreadState) {
    if state.packets_received.is_multiple_of(100) {
        logger.info(&format!(
            "RECV: Received {} RTP packets, released {} from buffer, decoded {} frames",
            state.packets_received, state.packets_released_from_buffer, state.frames_decoded
        ));
    }
}

fn track_packet_stats(
    packet_handler: &Arc<Mutex<PacketHandler>>,
    seq_num: u16,
    logger: &Logger,
    label: &str,
) {
    let mut handler = packet_handler.lock().unwrap_or_else(|poisoned| {
        logger.error("Packet handler mutex poisoned in receive thread, recovering");
        poisoned.into_inner()
    });

    handler.process_packet(seq_num);

    // Log packet loss statistics every 100 packets
    let stats = handler.stats();
    if stats.packets_received.is_multiple_of(100) {
        logger.info(&format!(
            "{} Packet Stats: Received={}, Lost={}, Loss Rate={:.1}%, Reordered={}, Duplicates={}",
            label,
            stats.packets_received,
            stats.packets_lost,
            stats.loss_rate * 100.0,
            stats.packets_reordered,
            stats.packets_duplicate
        ));
    }
}

fn add_to_jitter_buffer(
    jitter_buffer: &Arc<Mutex<JitterBuffer>>,
    packet: network::codec::rtp::RtpPacket,
    logger: &Logger,
) {
    jitter_buffer
        .lock()
        .unwrap_or_else(|poisoned| {
            logger.error("Jitter buffer mutex poisoned on push, recovering");
            poisoned.into_inner()
        })
        .push(packet);
}

/// Process audio packet: depacketize and decode immediately (no jitter buffer for audio)
fn process_audio_packet(
    packet: &network::codec::rtp::RtpPacket,
    params: &RecvThreadParams,
    depacketizer: &mut OpusRtpDepacketizer,
    state: &mut RecvThreadState,
) {
    if let Some(opus_data) = depacketizer.depacketize(packet) {
        // params.logger.trace(&format!("Depacketized audio: {} bytes", opus_data.len()));

        let mut decoder = params.audio_decoder.lock().unwrap_or_else(|poisoned| {
            params
                .logger
                .error("Audio decoder mutex poisoned, recovering");
            poisoned.into_inner()
        });

        match decoder.decode(&opus_data) {
            Ok(Some(audio_frame)) => {
                state.audio_frames_decoded += 1;

                if state.audio_frames_decoded.is_multiple_of(10) || state.audio_frames_decoded == 1 {
                    params.logger.debug(&format!(
                        "🔊 RECV: Decoded audio frame #{} ({} samples)",
                        state.audio_frames_decoded,
                        audio_frame.samples.len()
                    ));
                }

                // Send to application
                if let Err(e) = params.tx_audio_decode.try_send(audio_frame) {
                    params
                        .logger
                        .error(&format!("Failed to send audio frame to app: {}", e));
                }
            }
            Ok(None) => {
                params
                    .logger
                    .debug("Audio decoder returned None (needs more data)");
            }
            Err(e) => {
                params.logger.error(&format!("Audio decode error: {}", e));
            }
        }
    } else {
        params.logger.warn("Failed to depacketize audio packet");
    }
}
