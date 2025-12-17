//! Secure P2P session implementation with DTLS/SRTP
use crate::session::dtls_setup;
use crate::session::file_session::FileSession;
use crate::session::recv_thread;
use crate::session::send_thread;
use crate::session::video_decode_thread;

use super::config::P2PConfig;
use super::control_message::ControlMessage;
use crate::DtlsContext;
use logging::Logger;
use media::{AudioFrame, H264Decoder, H264Encoder, OpusDecoder, OpusEncoder, VideoFrame};
use network::codec::rtp::{RtpHeader, RtpPacket};
use network::security::dtls::DtlsEngine;
use network::transport::secure::UdpTransport;
use network::{
    H264RtpPacketizer, JitterBuffer, NetworkError, OpusRtpPacketizer, PacketHandler, Result,
    RtpPacketizer, SecureUdpTransport,
};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::mpsc::{Receiver, Sender, SyncSender, channel, sync_channel};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// Type alias for transport components tuple to reduce type complexity
type TransportComponents = (
    Arc<Mutex<Option<UdpTransport>>>,
    Arc<Mutex<Option<SecureUdpTransport>>>,
);
type JoinHandlesAndSender = (
    JoinHandle<()>,
    JoinHandle<()>,
    JoinHandle<()>,
    Sender<ControlMessage>,
);
type ChannelsTuple = (
    SyncSender<VideoFrame>,
    Receiver<VideoFrame>,
    SyncSender<AudioFrame>,
    Receiver<AudioFrame>,
    Receiver<ControlMessage>,
);
/// Represents a SECURE P2P session using H264 over RTP via DTLS/SRTP transport
pub struct SecureP2PSession {
    // Video components
    encoder: Arc<Mutex<H264Encoder>>,
    decoder: Arc<Mutex<H264Decoder>>,
    packetizer: Arc<Mutex<H264RtpPacketizer>>,

    // Audio components
    audio_encoder: Arc<Mutex<OpusEncoder>>,
    audio_decoder: Arc<Mutex<OpusDecoder>>,
    audio_packetizer: Arc<Mutex<OpusRtpPacketizer>>,

    // Shared transport and buffers
    transport: Arc<Mutex<Option<SecureUdpTransport>>>,
    udp_transport: Arc<Mutex<Option<UdpTransport>>>,
    jitter_buffer: Arc<Mutex<JitterBuffer>>,
    packet_handler: Arc<Mutex<PacketHandler>>,
    audio_packet_handler: Arc<Mutex<PacketHandler>>,

    // Video channels
    tx_encode: SyncSender<VideoFrame>,
    tx_audio_encode: SyncSender<AudioFrame>,
    tx_control: Option<Sender<ControlMessage>>,
    rx_decode: Receiver<VideoFrame>,
    rx_audio_decode: Receiver<AudioFrame>,
    rx_control: Receiver<ControlMessage>,

    send_thread: Option<JoinHandle<()>>,
    recv_thread: Option<JoinHandle<()>>,
    video_decode_thread: Option<JoinHandle<()>>,
    config: P2PConfig,
    logger: Logger,

    dtls_context: Option<DtlsContext>,
    dtls_engine: Arc<Mutex<Option<DtlsEngine>>>,
    local_fingerprint: Option<String>,
    remote_fingerprint: Option<String>,
    secure_connection_established: bool,
    control_sequence: Arc<Mutex<u16>>,

    /// File transfer session (SCTP data channels)
    file_session: Arc<Mutex<Option<FileSession>>>,
}

impl SecureP2PSession {
    pub fn new(config: &P2PConfig, logger: Logger) -> Result<Self> {
        logger.info("Creating SECURE P2P session (DTLS/SRTP enabled)");

        let (encoder, decoder) = create_codec_components(config, &logger)?;
        let (audio_encoder, audio_decoder) = create_audio_codec_components(&logger)?;

        let packetizer = H264RtpPacketizer::new(96, 1460, config.fps());
        let audio_packetizer = OpusRtpPacketizer::new(111, 1400, 48000, 20); // 20ms frames

        let (jitter_buffer, packet_handler, audio_packet_handler) = create_buffer_components();
        let (tx_encode, rx_decode, tx_audio_encode, rx_audio_decode, rx_control) =
            create_channels();
        let (udp_transport, transport) = create_transport_components(config, &logger)?;

        Ok(Self {
            encoder: Arc::new(Mutex::new(encoder)),
            decoder: Arc::new(Mutex::new(decoder)),
            packetizer: Arc::new(Mutex::new(packetizer)),
            audio_encoder: Arc::new(Mutex::new(audio_encoder)),
            audio_decoder: Arc::new(Mutex::new(audio_decoder)),
            audio_packetizer: Arc::new(Mutex::new(audio_packetizer)),
            transport,
            udp_transport,
            jitter_buffer: Arc::new(Mutex::new(jitter_buffer)),
            packet_handler: Arc::new(Mutex::new(packet_handler)),
            audio_packet_handler: Arc::new(Mutex::new(audio_packet_handler)),
            tx_encode,
            tx_audio_encode,
            tx_control: None,
            rx_decode,
            rx_audio_decode,
            rx_control,
            send_thread: None,
            recv_thread: None,
            video_decode_thread: None,
            config: config.clone(),
            logger,
            dtls_context: None,
            dtls_engine: Arc::new(Mutex::new(None)),
            local_fingerprint: None,
            remote_fingerprint: None,
            secure_connection_established: false,
            control_sequence: Arc::new(Mutex::new(0)),
            file_session: Arc::new(Mutex::new(None)),
        })
    }

    pub fn init_dtls(&mut self) -> Result<String> {
        self.logger
            .info("Initializing DTLS context for key exchange");

        let dtls = DtlsContext::new()
            .map_err(|e| NetworkError::SecurityError(format!("DTLS init failed: {}", e)))?;

        let fingerprint = dtls.get_fingerprint().to_string();
        self.logger
            .info(&format!("DTLS initialized, fingerprint: {}", fingerprint));

        self.local_fingerprint = Some(fingerprint.clone());
        self.dtls_context = Some(dtls);

        Ok(fingerprint)
    }

    pub fn set_remote_fingerprint(&mut self, fingerprint: String) -> Result<()> {
        self.logger
            .info(&format!("Setting remote DTLS fingerprint: {}", fingerprint));
        self.remote_fingerprint = Some(fingerprint);
        Ok(())
    }

    pub fn get_local_fingerprint(&self) -> Option<&str> {
        self.local_fingerprint.as_deref()
    }

    pub fn has_remote_fingerprint(&self) -> bool {
        self.remote_fingerprint.is_some()
    }

    pub fn establish_secure_connection(
        &mut self,
        remote_addr: SocketAddr,
        is_server: bool,
    ) -> Result<()> {
        let remote_fingerprint = self
            .remote_fingerprint
            .as_ref()
            .ok_or_else(|| NetworkError::SecurityError("Remote fingerprint not set".to_string()))?;

        let dtls = self
            .dtls_context
            .take()
            .ok_or_else(|| NetworkError::SecurityError("DTLS not initialized".to_string()))?;

        let (dtls, dtls_engine) = dtls_setup::establish_secure_connection(
            remote_addr,
            is_server,
            remote_fingerprint,
            dtls,
            self.udp_transport.clone(),
            self.transport.clone(),
            &self.logger,
        )?;

        self.dtls_context = Some(dtls);
        *self.dtls_engine.lock().unwrap() = Some(dtls_engine);
        self.secure_connection_established = true;

        self.logger
            .info("Initializing file transfer session (SCTP/Data Channels)");
        let mut file_session = FileSession::new(!is_server);
        match file_session.establish() {
            Ok(init_packet) => {
                if !init_packet.is_empty()
                    && let Ok(mut dtls_guard) = self.dtls_engine.lock()
                    && let Some(dtls_engine) = dtls_guard.as_mut()
                {
                    if let Err(e) = dtls_engine.send_application_data(&init_packet) {
                        self.logger
                            .error(&format!("Failed to send SCTP INIT: {}", e));
                    } else {
                        self.logger.info("✓ SCTP INIT sent successfully via DTLS");
                        // Send the encrypted DTLS packets via UDP to remote address
                        let packets = dtls_engine.take_pending_packets();
                        if let Ok(mut transport_guard) = self.transport.lock()
                            && let Some(transport) = transport_guard.as_mut()
                        {
                            let remote_addr = transport.remote_addr();
                            for packet in packets {
                                if let Err(e) = transport.socket().send_to(&packet, remote_addr) {
                                    self.logger
                                        .error(&format!("Failed to send DTLS packet: {}", e));
                                }
                            }
                        }
                    }
                }
                *self.file_session.lock().unwrap() = Some(file_session);
                self.logger.info("✓ File transfer session established");
            }
            Err(e) => {
                self.logger
                    .error(&format!("Failed to establish file session: {}", e));
            }
        }

        Ok(())
    }

    pub fn is_secure(&self) -> bool {
        self.secure_connection_established
    }

    /// Send a file to the remote peer
    ///
    /// # Arguments
    /// * `path` - Path to the file to send
    ///
    /// # Returns
    /// * `Ok(transfer_id)` - Unique ID for this transfer
    /// * `Err(msg)` - Error message if file couldn't be sent
    pub fn send_file(&self, path: &Path) -> Result<u64> {
        let file_session_guard = self
            .file_session
            .lock()
            .map_err(|e| NetworkError::TransportError(format!("Lock error: {}", e)))?;

        let file_session = file_session_guard.as_ref().ok_or_else(|| {
            NetworkError::TransportError("File session not established".to_string())
        })?;

        file_session
            .send_file(path)
            .map_err(NetworkError::TransportError)
    }

    /// Accept an incoming file transfer
    pub fn accept_file_transfer(&self, transfer_id: u64, save_path: &Path) -> Result<()> {
        let file_session_guard = self
            .file_session
            .lock()
            .map_err(|e| NetworkError::TransportError(format!("Lock error: {}", e)))?;

        let file_session = file_session_guard.as_ref().ok_or_else(|| {
            NetworkError::TransportError("File session not established".to_string())
        })?;

        file_session
            .accept_transfer(transfer_id, save_path)
            .map_err(NetworkError::TransportError)
    }

    /// Reject an incoming file transfer
    pub fn reject_file_transfer(&self, transfer_id: u64, reason: &str) -> Result<()> {
        let file_session_guard = self
            .file_session
            .lock()
            .map_err(|e| NetworkError::TransportError(format!("Lock error: {}", e)))?;

        let file_session = file_session_guard.as_ref().ok_or_else(|| {
            NetworkError::TransportError("File session not established".to_string())
        })?;

        file_session
            .reject_transfer(transfer_id, reason)
            .map_err(NetworkError::TransportError)
    }

    /// Cancel an ongoing file transfer
    pub fn cancel_file_transfer(&self, transfer_id: u64, reason: &str) -> Result<()> {
        let file_session_guard = self
            .file_session
            .lock()
            .map_err(|e| NetworkError::TransportError(format!("Lock error: {}", e)))?;

        let file_session = file_session_guard.as_ref().ok_or_else(|| {
            NetworkError::TransportError("File session not established".to_string())
        })?;

        file_session
            .cancel_transfer(transfer_id, reason)
            .map_err(NetworkError::TransportError)
    }

    /// Get file transfer progress (bytes_transferred, total_bytes)
    pub fn get_file_transfer_progress(&self, transfer_id: u64) -> Option<(u64, u64)> {
        if let Ok(file_session_guard) = self.file_session.lock()
            && let Some(file_session) = file_session_guard.as_ref() {
                return file_session.get_progress(transfer_id);
            }
        None
    }

    /// Check if file channel is ready to send/receive files
    ///
    /// Returns true when the SCTP association is established and the file channel is open
    pub fn is_file_channel_ready(&self) -> bool {
        if let Ok(file_session_guard) = self.file_session.lock()
            && let Some(file_session) = file_session_guard.as_ref() {
                return file_session.is_established();
            }
        false
    }

    /// Poll for outgoing SCTP packets and send them (batches multiple packets)
    pub fn poll_sctp_send(&self) -> Result<()> {
        const MAX_SCTP_SEND_BATCH: usize = 16; // Match recv batch size

        let mut file_session_guard = self
            .file_session
            .lock()
            .map_err(|e| NetworkError::TransportError(format!("Lock error: {}", e)))?;

        if let Some(file_session) = file_session_guard.as_mut() {
            // Try to send multiple packets per call for better throughput
            for _ in 0..MAX_SCTP_SEND_BATCH {
                if let Some(sctp_data) = file_session.poll_send() {
                    let data_len = sctp_data.len();

                    // Send through DTLS engine as application data
                    if let Ok(mut dtls_guard) = self.dtls_engine.lock()
                        && let Some(dtls_engine) = dtls_guard.as_mut() {
                            dtls_engine.send_application_data(&sctp_data).map_err(|e| {
                                NetworkError::TransportError(format!("DTLS send failed: {}", e))
                            })?;

                            // Send the encrypted DTLS packets via UDP to remote address
                            let packets = dtls_engine.take_pending_packets();
                            if let Ok(mut transport_guard) = self.transport.lock()
                                && let Some(transport) = transport_guard.as_mut() {
                                    let remote_addr = transport.remote_addr();
                                    for packet in packets {
                                        transport.socket().send_to(&packet, remote_addr).map_err(
                                            |e| {
                                                NetworkError::TransportError(format!(
                                                    "UDP send failed: {}",
                                                    e
                                                ))
                                            },
                                        )?;
                                    }
                                }

                            // Notify file session that bytes were sent (triggers adaptive chunking)
                            file_session.on_bytes_sent(data_len);
                        }
                } else {
                    // No more data to send
                    break;
                }
            }
        }
        Ok(())
    }

    /// Process incoming SCTP data
    pub fn process_sctp_data(&self, data: &[u8]) -> Result<()> {
        let mut file_session_guard = self
            .file_session
            .lock()
            .map_err(|e| NetworkError::TransportError(format!("Lock error: {}", e)))?;

        if let Some(file_session) = file_session_guard.as_mut() {
            match file_session.on_sctp_data(data) {
                Ok(responses) if !responses.is_empty() => {
                    // Send ALL response packets through transport
                    if let Ok(mut transport_guard) = self.transport.lock()
                        && let Some(transport) = transport_guard.as_mut() {
                            for response_bytes in responses {
                                transport.send_sctp(&response_bytes)?;
                            }
                        }
                }
                Ok(_) => {
                    // No response needed
                }
                Err(e) => {
                    self.logger.error(&format!("SCTP processing error: {}", e));
                }
            }
        }
        Ok(())
    }

    pub fn update_encoder_resolution(
        &mut self,
        width: u32,
        height: u32,
        bitrate: u32,
    ) -> Result<()> {
        self.logger.info(&format!(
            "Updating encoder resolution: {}x{} @ {:.2} Mbps",
            width,
            height,
            bitrate as f64 / 1_000_000.0
        ));

        let new_encoder = H264Encoder::new(
            width,
            height,
            bitrate,
            30,
            self.config.fps(),
            self.logger.clone(),
        )
        .map_err(|e| NetworkError::Config(format!("Failed to recreate encoder: {}", e)))?;

        *self.encoder.lock().unwrap_or_else(|poisoned| {
            self.logger.error("Encoder mutex poisoned, recovering");
            poisoned.into_inner()
        }) = new_encoder;

        self.config = self
            .config
            .clone()
            .with_resolution(width, height)
            .with_bitrate(bitrate);

        self.logger.info("Encoder recreated successfully");
        Ok(())
    }

    pub fn start(&mut self) -> Result<()> {
        if !self.secure_connection_established {
            return Err(NetworkError::SecurityError(
                "Cannot start session before secure connection is established".to_string(),
            ));
        }

        self.logger.info("Starting secure send/receive threads");

        let (send_handle, recv_handle, video_decode_handle, tx_control) = spawn_threads(self)?;

        self.send_thread = Some(send_handle);
        self.recv_thread = Some(recv_handle);
        self.video_decode_thread = Some(video_decode_handle);
        self.tx_control = Some(tx_control);

        self.logger.info("Secure send/receive threads started");
        Ok(())
    }

    pub fn send_frame(&self, frame: VideoFrame) -> Result<()> {
        if !self.secure_connection_established {
            return Err(NetworkError::SecurityError(
                "Cannot send frame before secure connection is established".to_string(),
            ));
        }

        self.tx_encode
            .try_send(frame)
            .map_err(|e| NetworkError::ChannelError(format!("Failed to send frame: {}", e)))
    }

    pub fn send_audio_frame(&self, frame: AudioFrame) -> Result<()> {
        if !self.secure_connection_established {
            return Err(NetworkError::SecurityError(
                "Cannot send audio frame before secure connection is established".to_string(),
            ));
        }

        self.tx_audio_encode
            .try_send(frame)
            .map_err(|e| NetworkError::ChannelError(format!("Failed to send audio frame: {}", e)))
    }

    pub fn receive_frame(&self) -> Result<Option<VideoFrame>> {
        match self.rx_decode.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(NetworkError::ChannelError(
                "Decode channel disconnected".to_string(),
            )),
        }
    }

    pub fn receive_audio_frame(&self) -> Result<Option<AudioFrame>> {
        match self.rx_audio_decode.try_recv() {
            Ok(frame) => Ok(Some(frame)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(NetworkError::ChannelError(
                "Audio decode channel disconnected".to_string(),
            )),
        }
    }

    pub fn send_control_message(&self, message: ControlMessage) -> Result<()> {
        if !self.secure_connection_established {
            return Err(NetworkError::SecurityError(
                "Cannot send control message before secure connection".to_string(),
            ));
        }

        let payload = message.to_bytes();
        let mut seq_guard = self.control_sequence.lock().unwrap();
        let seq = *seq_guard;
        *seq_guard = seq_guard.wrapping_add(1);
        drop(seq_guard);

        // Get video SSRC from packetizer to use video_ssrc + 1 for control
        let video_ssrc = self
            .packetizer
            .lock()
            .unwrap_or_else(|poisoned| {
                self.logger
                    .error("Packetizer mutex poisoned in send_control_message, recovering");
                poisoned.into_inner()
            })
            .get_ssrc();

        let control_packet = create_control_packet(seq, payload, video_ssrc);

        self.transport
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .as_mut()
            .ok_or_else(|| NetworkError::TransportError("Transport not initialized".to_string()))?
            .send_rtp(&control_packet)
            .map_err(|e| NetworkError::TransportError(format!("Failed to send control: {}", e)))
    }

    pub fn receive_control_message(&self) -> Result<Option<ControlMessage>> {
        match self.rx_control.try_recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(std::sync::mpsc::TryRecvError::Empty) => Ok(None),
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Err(NetworkError::ChannelError(
                "Control channel disconnected".to_string(),
            )),
        }
    }

    /// Demultiplex incoming UDP packets (RTP/RTCP/DTLS)
    /// MUST be called before receive_sctp() to feed packets into DTLS layer!
    pub fn unified_receive(&mut self) -> Result<()> {
        let mut transport_guard = self
            .transport
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(transport) = transport_guard.as_mut() {
            transport.unified_receive()?;
        }
        Ok(())
    }

    /// Receives and processes SCTP data from transport
    /// Should be called regularly in receive loop
    pub fn receive_and_process_sctp(&mut self) -> Result<()> {
        let packets: Vec<Vec<u8>> = {
            let mut binding = self
                .transport
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());

            let transport = binding.as_mut().ok_or_else(|| {
                NetworkError::TransportError("Transport not initialized".to_string())
            })?;

            let mut packets = Vec::new();
            for _ in 0..100 {
                match transport.receive_sctp()? {
                    Some(sctp_data) => packets.push(sctp_data),
                    None => break, // No more packets available
                }
            }
            packets
        }; // Lock released here

        for sctp_data in packets {
            self.process_sctp_data(&sctp_data)?;
        }

        Ok(())
    }

    /// Poll for file transfer events
    pub fn poll_event(&self) -> Option<super::file_transfer::FileTransferEvent> {
        let file_session_guard = match self.file_session.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                // Lock contention - SCTP thread is holding the lock
                return None;
            }
        };
        let file_session = file_session_guard.as_ref()?;
        file_session.poll_event()
    }

    pub fn get_jitter_stats(&self) -> network::JitterBufferStats {
        self.jitter_buffer
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .stats()
            .clone()
    }

    pub fn get_packet_stats(&self) -> network::PacketStats {
        let video_stats = self
            .packet_handler
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .stats()
            .clone();

        let audio_stats = self
            .audio_packet_handler
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .stats()
            .clone();

        // Aggregate stats
        let packets_received = video_stats.packets_received + audio_stats.packets_received;
        let packets_lost = video_stats.packets_lost + audio_stats.packets_lost;
        let packets_reordered = video_stats.packets_reordered + audio_stats.packets_reordered;
        let packets_duplicate = video_stats.packets_duplicate + audio_stats.packets_duplicate;

        let total_expected = packets_received + packets_lost;
        let loss_rate = if total_expected > 0 {
            packets_lost as f64 / total_expected as f64
        } else {
            0.0
        };

        network::PacketStats {
            packets_received,
            packets_lost,
            packets_reordered,
            packets_duplicate,
            loss_rate,
        }
    }

    pub fn get_rtcp_stats(&self) -> Option<network::RtcpStats> {
        self.transport
            .lock()
            .unwrap_or_else(|poisoned| {
                self.logger
                    .error("Transport mutex poisoned in get_rtcp_stats, recovering");
                poisoned.into_inner()
            })
            .as_ref()
            .map(|t| (*t.get_stats()).clone())
    }

    /// Clears jitter buffer and packet handler to flush delayed video packets
    /// Call this when camera turns off to prevent stale frames from being displayed
    pub fn clear_video_buffers(&self) {
        self.logger
            .info("Clearing video buffers (jitter buffer + packet handler)");

        self.jitter_buffer
            .lock()
            .unwrap_or_else(|poisoned| {
                self.logger
                    .error("Jitter buffer mutex poisoned in clear_video_buffers, recovering");
                poisoned.into_inner()
            })
            .clear();

        self.packet_handler
            .lock()
            .unwrap_or_else(|poisoned| {
                self.logger
                    .error("Packet handler mutex poisoned in clear_video_buffers, recovering");
                poisoned.into_inner()
            })
            .clear();

        self.logger.info("Video buffers cleared successfully");
    }

    /// Close the session and clean up resources
    pub fn close(&mut self) {
        self.logger
            .info("[SESSION_CLEANUP] Closing SecureP2PSession...");

        // Drop transport to close socket
        if let Ok(mut transport_guard) = self.transport.lock() {
            *transport_guard = None;
            self.logger
                .info("[SESSION_CLEANUP] Secure transport closed");
        }

        if let Ok(mut udp_guard) = self.udp_transport.lock() {
            *udp_guard = None;
            self.logger.info("[SESSION_CLEANUP] UDP transport closed");
        }

        // Close file session
        if let Ok(mut file_session_guard) = self.file_session.lock() {
            *file_session_guard = None;
            self.logger.info("[SESSION_CLEANUP] File session closed");
        }

        // Clear thread handles (they will be joined when dropped)
        self.send_thread = None;
        self.recv_thread = None;
        self.video_decode_thread = None;

        self.secure_connection_established = false;
        self.logger
            .info("[SESSION_CLEANUP] SecureP2PSession closed");
    }
}

fn create_codec_components(
    config: &P2PConfig,
    logger: &Logger,
) -> Result<(H264Encoder, H264Decoder)> {
    let encoder = H264Encoder::new(
        config.frame_width(),
        config.frame_height(),
        config.codec_bitrate(),
        30,
        config.fps(),
        logger.clone(),
    )
    .map_err(|e| NetworkError::Config(format!("Failed to create encoder: {}", e)))?;

    let decoder = H264Decoder::new(logger.clone())
        .map_err(|e| NetworkError::Config(format!("Failed to create decoder: {}", e)))?;

    logger.info(&format!(
        "Encoder created: {}x{} @ {:.1}fps, {:.2} Mbps",
        config.frame_width(),
        config.frame_height(),
        config.fps(),
        config.codec_bitrate() as f64 / 1_000_000.0
    ));

    Ok((encoder, decoder))
}

/// Create audio codec components (Opus encoder/decoder)
fn create_audio_codec_components(logger: &Logger) -> Result<(OpusEncoder, OpusDecoder)> {
    // Opus standard: 48kHz, stereo, 64kbps for high quality voice
    const SAMPLE_RATE: u32 = 48000;
    const CHANNELS: u32 = 2;
    const BITRATE: u32 = 64000;

    logger.info("Creating Opus audio encoder and decoder");

    let encoder = OpusEncoder::new(SAMPLE_RATE, CHANNELS, BITRATE, logger.clone())
        .map_err(|e| NetworkError::Config(format!("Failed to create audio encoder: {}", e)))?;

    let decoder = OpusDecoder::new(SAMPLE_RATE, CHANNELS, logger.clone())
        .map_err(|e| NetworkError::Config(format!("Failed to create audio decoder: {}", e)))?;

    logger.info(&format!(
        "Opus codec created: {}Hz, {} channels, {:.1} kbps",
        SAMPLE_RATE,
        CHANNELS,
        BITRATE as f64 / 1000.0
    ));

    Ok((encoder, decoder))
}

fn create_buffer_components() -> (JitterBuffer, PacketHandler, PacketHandler) {
    let jitter_config = network::JitterBufferConfig {
        clock_rate: 90000,
        min_delay_frames: 1,
        max_delay_frames: 8,
        target_jitter_ms: 10.0,
        max_capacity: 250,
        adaptation_speed: 0.15,
        ultra_low_latency: true,
    };

    let jitter_buffer = JitterBuffer::with_config(jitter_config);
    let packet_handler = PacketHandler::new();
    let audio_packet_handler = PacketHandler::new();

    (jitter_buffer, packet_handler, audio_packet_handler)
}

fn create_channels() -> ChannelsTuple {
    let (tx_encode, _rx_encode) = sync_channel::<VideoFrame>(12);
    let (_tx_decode, rx_decode) = sync_channel::<VideoFrame>(12);
    let (tx_audio_encode, _rx_audio_encode) = sync_channel::<AudioFrame>(120); // More buffer for audio
    let (_tx_audio_decode, rx_audio_decode) = sync_channel::<AudioFrame>(120);
    let (_tx_control, rx_control) = channel::<ControlMessage>();

    (
        tx_encode,
        rx_decode,
        tx_audio_encode,
        rx_audio_decode,
        rx_control,
    )
}

fn create_transport_components(config: &P2PConfig, logger: &Logger) -> Result<TransportComponents> {
    let local_addr_str = format!("0.0.0.0:{}", config.local_port());

    let udp_transport = UdpTransport::new(&local_addr_str)
        .map_err(|e| NetworkError::TransportError(format!("Failed to create UDP: {}", e)))?;

    logger.info(&format!(
        "UDP transport created on {} (DTLS handshake required for security)",
        local_addr_str
    ));

    Ok((
        Arc::new(Mutex::new(Some(udp_transport))),
        Arc::new(Mutex::new(None)),
    ))
}

fn spawn_threads(
    session: &mut SecureP2PSession,
    ) -> Result<JoinHandlesAndSender> {
    let (new_tx_encode, rx_encode) = sync_channel::<VideoFrame>(12);
    let (tx_decode, new_rx_decode) = sync_channel::<VideoFrame>(12);
    let (new_tx_audio_encode, rx_audio_encode) = sync_channel::<AudioFrame>(120);
    let (tx_audio_decode, new_rx_audio_decode) = sync_channel::<AudioFrame>(120);
    let (tx_control, new_rx_control) = channel::<ControlMessage>();

    let old_tx_encode = std::mem::replace(&mut session.tx_encode, new_tx_encode);
    let old_rx_decode = std::mem::replace(&mut session.rx_decode, new_rx_decode);
    let old_tx_audio_encode = std::mem::replace(&mut session.tx_audio_encode, new_tx_audio_encode);
    let old_rx_audio_decode = std::mem::replace(&mut session.rx_audio_decode, new_rx_audio_decode);
    let old_rx_control = std::mem::replace(&mut session.rx_control, new_rx_control);

    drop(old_tx_encode);
    drop(old_rx_decode);
    drop(old_tx_audio_encode);
    drop(old_rx_audio_decode);
    drop(old_rx_control);

    let send_params = send_thread::SendThreadParams {
        encoder: Arc::clone(&session.encoder),
        packetizer: Arc::clone(&session.packetizer),
        audio_encoder: Arc::clone(&session.audio_encoder),
        audio_packetizer: Arc::clone(&session.audio_packetizer),
        transport: Arc::clone(&session.transport),
        rx_encode,
        rx_audio_encode,
        logger: session.logger.clone(),
    };

    let send_handle = thread::Builder::new()
        .name("secure-send".to_string())
        .spawn(move || send_thread::run_send_thread(send_params))
        .map_err(|e| NetworkError::ThreadError(format!("Failed to spawn send thread: {}", e)))?;

    // Spawn Video Decode Thread
    let video_decode_params = video_decode_thread::VideoDecodeThreadParams {
        jitter_buffer: Arc::clone(&session.jitter_buffer),
        decoder: Arc::clone(&session.decoder),
        tx_decode,
        logger: session.logger.clone(),
    };

    let video_decode_handle = thread::Builder::new()
        .name("video-decode".to_string())
        .spawn(move || video_decode_thread::run_video_decode_thread(video_decode_params))
        .map_err(|e| {
            NetworkError::ThreadError(format!("Failed to spawn video decode thread: {}", e))
        })?;

    let recv_params = recv_thread::RecvThreadParams {
        audio_decoder: Arc::clone(&session.audio_decoder),
        transport: Arc::clone(&session.transport),
        jitter_buffer: Arc::clone(&session.jitter_buffer),
        packet_handler: Arc::clone(&session.packet_handler),
        audio_packet_handler: Arc::clone(&session.audio_packet_handler),
        tx_audio_decode,
        tx_control: tx_control.clone(),
        logger: session.logger.clone(),
        dtls_engine: Arc::clone(&session.dtls_engine),
        file_session: Arc::clone(&session.file_session),
    };

    let recv_handle = thread::Builder::new()
        .name("secure-recv".to_string())
        .spawn(move || recv_thread::run_recv_thread(recv_params))
        .map_err(|e| NetworkError::ThreadError(format!("Failed to spawn recv thread: {}", e)))?;

    Ok((send_handle, recv_handle, video_decode_handle, tx_control))
}

fn create_control_packet(seq: u16, payload: Vec<u8>, video_ssrc: u32) -> RtpPacket {
    // Use video_ssrc + 1 to avoid SRTP replay window conflicts
    // Control messages need separate SSRC from video packets
    let control_ssrc = video_ssrc.wrapping_add(1);

    let mut header = RtpHeader::new(network::codec::rtp::control_payload::CONTROL, control_ssrc);
    header.sequence_number = seq;
    header.timestamp = 0;
    header.marker = true;

    RtpPacket { header, payload }
}
