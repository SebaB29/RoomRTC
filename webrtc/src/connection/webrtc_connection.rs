//! Main WebRTC connection implementation
//!
//! Coordinates camera, audio, ICE, SDP, and media session components

use super::audio::AudioHandler;
use super::camera::CameraHandler;
use super::ice::IceHandler;
use super::sdp::SdpHandler;
use crate::audio_info::AudioInfo;
use crate::audio_manager::AudioSettings;
use crate::camera_info::CameraInfo;
use crate::camera_manager::CameraResolution;
use crate::session::{ControlMessage, P2PConfig, SecureP2PSession};
use logging::Logger;
use std::error::Error;
use std::net::SocketAddr;

/// Type alias for RGB frame data: (width, height, pixel_data)
pub type RgbFrame = (usize, usize, Vec<u8>);

/// Main WebRTC connection with DTLS/SRTP encryption
pub struct WebRtcConnection {
    ice_handler: IceHandler,
    sdp_handler: SdpHandler,
    camera_handler: CameraHandler,
    audio_handler: AudioHandler,
    media_session: SecureP2PSession,
    session_config: P2PConfig,
    logger: Logger,
    connection_started: bool,
    is_offerer: bool,
    file_channel_ready_emitted: bool,
}

impl WebRtcConnection {
    /// Creates a new secure WebRTC connection with DTLS/SRTP
    pub fn new(port: Option<u16>, logger: Logger) -> Result<Self, Box<dyn Error>> {
        logger.info("Creating secure WebRTC connection (DTLS/SRTP enabled)");

        let local_port = network::find_available_port(port.unwrap_or(5000), &logger)?;
        logger.info(&format!(
            "Using local port: {} (secure transport)",
            local_port
        ));

        let session_config = P2PConfig::builder()
            .local_port(local_port)
            .remote_port(5000)
            .build();

        let mut media_session =
            SecureP2PSession::new(&session_config.clone().with_auto_bitrate(), logger.clone())?;

        let fingerprint = media_session
            .init_dtls()
            .map_err(|e| format!("Failed to initialize DTLS: {}", e))?;

        logger.info(&format!("DTLS initialized, fingerprint: {}", fingerprint));

        Ok(Self {
            ice_handler: IceHandler::new(logger.clone()),
            sdp_handler: SdpHandler::new(logger.clone()),
            camera_handler: CameraHandler::new(logger.clone()),
            audio_handler: AudioHandler::new(logger.clone()),
            media_session,
            session_config,
            logger,
            connection_started: false,
            is_offerer: false,
            file_channel_ready_emitted: false,
        })
    }

    pub fn list_camera_ids_fast() -> Vec<i32> {
        CameraHandler::list_camera_ids_fast()
    }

    pub fn create_offer_from_new(logger: Logger) -> Result<(Self, String), Box<dyn Error>> {
        logger.info("Creating new secure WebRTC connection for OFFERER");
        let mut conn = Self::new(None, logger.clone())?;
        conn.is_offerer = true;

        let offer = conn.create_offer()?;
        logger.info("OFFER generated with DTLS fingerprint");
        Ok((conn, offer))
    }

    pub fn create_answer_from_new(
        offer_sdp: &str,
        logger: Logger,
    ) -> Result<(Self, String), Box<dyn Error>> {
        logger.info("Creating new secure WebRTC connection for ANSWERER");
        let mut conn = Self::new(None, logger.clone())?;
        conn.is_offerer = false;

        logger.info("Processing remote OFFER with DTLS fingerprint...");
        conn.set_remote_offer(offer_sdp)?;
        logger.info("Remote OFFER processed, DTLS fingerprint extracted");

        let answer = conn.create_answer()?;
        logger.info("ANSWER generated with DTLS fingerprint");
        Ok((conn, answer))
    }

    pub fn create_offer(&mut self) -> Result<String, Box<dyn Error>> {
        self.logger
            .info("Creating SDP offer with DTLS fingerprint and ICE candidates");

        let port = self.session_config.local_port();
        self.logger
            .info(&format!("Gathering ICE candidates on port {}...", port));
        self.ice_handler.gather_candidates(port)?;

        self.logger.info(&format!(
            "Total ICE candidates gathered: {}",
            self.ice_handler.ice_agent.local_candidates.len()
        ));

        for (i, candidate) in self
            .ice_handler
            .ice_agent
            .local_candidates
            .iter()
            .enumerate()
        {
            self.logger.info(&format!(
                "  Candidate {}: {}:{} (type: {:?})",
                i + 1,
                candidate.address,
                candidate.port,
                candidate.candidate_type
            ));
        }

        let fingerprint = self
            .media_session
            .get_local_fingerprint()
            .ok_or("DTLS not initialized")?;

        let sdp = self.sdp_handler.create_offer(&self.ice_handler.ice_agent)?;
        let sdp = SdpHandler::add_fingerprint_to_sdp(sdp, fingerprint, true);

        self.logger.info("SDP offer created with DTLS fingerprint");
        Ok(sdp)
    }

    pub fn create_answer(&mut self) -> Result<String, Box<dyn Error>> {
        self.logger
            .info("Creating SDP answer with DTLS fingerprint and ICE candidates");

        let port = self.session_config.local_port();
        self.logger
            .info(&format!("Gathering ICE candidates on port {}...", port));
        self.ice_handler.gather_candidates(port)?;

        self.logger.info(&format!(
            "Total ICE candidates gathered: {}",
            self.ice_handler.ice_agent.local_candidates.len()
        ));

        for (i, candidate) in self
            .ice_handler
            .ice_agent
            .local_candidates
            .iter()
            .enumerate()
        {
            self.logger.info(&format!(
                "  Candidate {}: {}:{} (type: {:?})",
                i + 1,
                candidate.address,
                candidate.port,
                candidate.candidate_type
            ));
        }

        let fingerprint = self
            .media_session
            .get_local_fingerprint()
            .ok_or("DTLS not initialized")?;

        let sdp = self
            .sdp_handler
            .create_answer(&self.ice_handler.ice_agent)?;
        let sdp = SdpHandler::add_fingerprint_to_sdp(sdp, fingerprint, false);

        self.logger.info("SDP answer created with DTLS fingerprint");
        Ok(sdp)
    }

    pub fn set_remote_sdp(&mut self, sdp: &str) -> Result<(), Box<dyn Error>> {
        use sdp::SdpType;

        let sdp_type = if sdp.contains("a=sendrecv") || sdp.contains("m=application") {
            if sdp.lines().any(|line| line.starts_with("a=group:")) {
                SdpType::Offer
            } else {
                SdpType::Answer
            }
        } else {
            SdpType::Offer
        };

        self.set_remote_description_internal(sdp_type, sdp)
    }

    pub fn set_remote_offer(&mut self, sdp: &str) -> Result<(), Box<dyn Error>> {
        self.set_remote_description_internal(sdp::SdpType::Offer, sdp)
    }

    pub fn set_remote_answer(&mut self, sdp: &str) -> Result<(), Box<dyn Error>> {
        self.set_remote_description_internal(sdp::SdpType::Answer, sdp)
    }

    fn set_remote_description_internal(
        &mut self,
        sdp_type: sdp::SdpType,
        sdp: &str,
    ) -> Result<(), Box<dyn Error>> {
        self.logger.info(&format!(
            "Setting remote SDP {:?} and extracting DTLS fingerprint",
            sdp_type
        ));

        self.process_remote_candidates(sdp)?;
        self.extract_and_set_fingerprint(sdp)?;
        self.extract_and_apply_remote_endpoint(sdp)?;

        Ok(())
    }

    fn process_remote_candidates(&mut self, sdp: &str) -> Result<(), Box<dyn Error>> {
        let candidate_attrs: Vec<String> = sdp
            .lines()
            .filter_map(|line| {
                if line.starts_with("a=candidate:") {
                    Some(line.to_string())
                } else if line.starts_with("candidate:") {
                    Some(format!("a={}", line))
                } else {
                    None
                }
            })
            .collect();

        self.logger.info(&format!(
            "Found {} remote ICE candidate lines in SDP",
            candidate_attrs.len()
        ));

        if !candidate_attrs.is_empty() {
            for (i, attr) in candidate_attrs.iter().enumerate() {
                self.logger
                    .info(&format!("  Candidate {}: {}", i + 1, attr));
            }

            self.ice_handler
                .ice_agent
                .add_remote_candidates_from_sdp(&candidate_attrs)?;

            self.logger.info(&format!(
                "Successfully added {} remote ICE candidates",
                self.ice_handler.ice_agent.remote_candidates.len()
            ));
        } else {
            self.logger.warn("No ICE candidates found in remote SDP");
        }

        Ok(())
    }

    fn extract_and_set_fingerprint(&mut self, sdp: &str) -> Result<(), Box<dyn Error>> {
        if let Some(fingerprint) = SdpHandler::extract_fingerprint_from_sdp(sdp) {
            self.logger.info(&format!(
                "Extracted remote DTLS fingerprint: {}",
                fingerprint
            ));
            self.media_session
                .set_remote_fingerprint(fingerprint)
                .map_err(|e| format!("Failed to set remote fingerprint: {}", e))?;
        } else {
            return Err("Remote SDP does not contain DTLS fingerprint".into());
        }
        Ok(())
    }

    fn extract_and_apply_remote_endpoint(&mut self, sdp: &str) -> Result<(), Box<dyn Error>> {
        if let Some((remote_ip, remote_port)) =
            self.ice_handler.extract_remote_endpoint_from_sdp(sdp)
        {
            self.logger.info(&format!(
                "Extracted remote endpoint from SDP: {}:{}",
                remote_ip, remote_port
            ));
            self.apply_remote_endpoint(&remote_ip, remote_port)?;
        } else {
            self.logger
                .warn("Could not extract remote endpoint from SDP - will use default");
        }
        Ok(())
    }

    pub fn set_stun_servers(&mut self, servers: Vec<String>) {
        self.ice_handler.set_stun_servers(servers);
    }

    pub fn set_turn_servers(&mut self, servers: Vec<String>) {
        self.ice_handler.set_turn_servers(servers);
    }

    pub fn add_ice_candidate(
        &mut self,
        candidate: &str,
        sdp_mid: &str,
        sdp_mline_index: u16,
    ) -> Result<(), Box<dyn Error>> {
        self.ice_handler
            .add_ice_candidate(candidate, sdp_mid, sdp_mline_index)
    }

    pub fn establish_connection(&mut self) -> Result<(), Box<dyn Error>> {
        if self.connection_started {
            return Ok(());
        }

        self.logger.info("Validating DTLS preconditions...");
        self.validate_dtls_preconditions()
            .map_err(|e| -> Box<dyn Error> { e.into() })?;

        self.logger.info(&format!(
            "Establishing secure connection with DTLS handshake ({} remote ICE candidates)",
            self.ice_handler.ice_agent.remote_candidates.len()
        ));

        let remote_addr = self.determine_remote_address()?;
        let is_server = !self.is_offerer;

        self.logger.info(&format!(
            "Remote: {} | Local role: {} | Offerer: {}",
            remote_addr,
            if is_server {
                "server (answerer)"
            } else {
                "client (offerer)"
            },
            self.is_offerer
        ));

        self.media_session
            .establish_secure_connection(remote_addr, is_server)
            .map_err(|e| format!("Failed to establish secure connection: {}", e))?;

        self.logger
            .info("DTLS handshake completed, SRTP keys established");

        self.start_media_threads()?;
        self.verify_security()?;

        // Auto-start audio playback so users can hear remote audio immediately
        if let Err(e) = self.audio_handler.start_playback(48000, 2) {
            self.logger.warn(&format!(
                "Failed to auto-start audio playback: {}. Audio reception may not work.",
                e
            ));
        } else {
            self.logger
                .info("Audio playback auto-started for receiving remote audio");
        }

        self.connection_started = true;
        self.log_connection_status();

        Ok(())
    }

    fn determine_remote_address(&self) -> Result<SocketAddr, Box<dyn Error>> {
        let (remote_ip, remote_port) = self.ice_handler.get_remote_address().unwrap_or_else(|| {
            self.logger
                .warn("No remote ICE candidates, using fallback address");
            ("127.0.0.1".to_string(), self.session_config.remote_port())
        });

        let remote_addr: SocketAddr = format!("{}:{}", remote_ip, remote_port).parse()?;
        self.logger
            .info(&format!("Using remote address: {}", remote_addr));
        Ok(remote_addr)
    }

    fn start_media_threads(&mut self) -> Result<(), Box<dyn Error>> {
        self.logger.info("Starting media threads...");
        self.media_session
            .start()
            .map_err(|e| format!("Failed to start media session: {}", e))?;
        self.logger.info("Media threads started successfully");
        Ok(())
    }

    fn verify_security(&self) -> Result<(), Box<dyn Error>> {
        if !self.media_session.is_secure() {
            return Err("CRITICAL: Media session reports NOT SECURE after setup!".into());
        }
        Ok(())
    }

    fn log_connection_status(&self) {
        self.logger.info("CONNECTION IS NOW ENCRYPTED AND RUNNING");
        self.logger
            .info(&format!("connection_started={}", self.connection_started));
        self.logger
            .info(&format!("is_secure={}", self.media_session.is_secure()));
    }

    pub fn is_connected(&self) -> bool {
        self.connection_started && self.media_session.is_secure()
    }

    pub fn local_port(&self) -> u16 {
        self.session_config.local_port()
    }

    pub fn remote_port(&self) -> u16 {
        self.session_config.remote_port()
    }

    pub fn discover_cameras(&mut self) -> Result<Vec<CameraInfo>, Box<dyn Error>> {
        self.camera_handler.discover_cameras()
    }

    pub fn is_camera_running(&self) -> bool {
        self.camera_handler.is_camera_running()
    }

    pub fn is_microphone_running(&self) -> bool {
        self.audio_handler.is_audio_running()
    }

    pub fn start_camera(&mut self, camera_index: i32, fps: f64) -> Result<(), Box<dyn Error>> {
        let (resolution, should_send_message) =
            self.camera_handler.start_camera(camera_index, fps)?;
        self.apply_camera_resolution(resolution)?;

        if self.connection_started && should_send_message {
            self.send_camera_on_message()?;
        }

        Ok(())
    }

    pub fn start_camera_auto(&mut self, fps: f64) -> Result<(), Box<dyn Error>> {
        let resolution = self.camera_handler.start_camera_auto(fps)?;
        self.apply_camera_resolution(resolution)?;

        if self.connection_started {
            self.send_camera_on_message()?;
        }

        Ok(())
    }

    fn send_camera_on_message(&self) -> Result<(), Box<dyn Error>> {
        self.logger.info("Sending CameraOn control message to peer");
        self.media_session
            .send_control_message(ControlMessage::CameraOn)
            .map_err(|e| format!("Failed to send CameraOn message: {}", e).into())
    }

    pub fn stop_camera(&mut self) {
        self.camera_handler.stop_camera();

        if self.connection_started {
            self.logger
                .info("Sending CameraOff control message to peer");
            if let Err(e) = self
                .media_session
                .send_control_message(ControlMessage::CameraOff)
            {
                self.logger
                    .error(&format!("Failed to send CameraOff message: {}", e));
            }
        }
    }

    pub fn capture_frame(&mut self) -> Result<media::VideoFrame, Box<dyn Error>> {
        self.camera_handler.capture_frame()
    }

    // ===== Audio Methods =====

    pub fn discover_audio_devices(&mut self) -> Result<Vec<AudioInfo>, Box<dyn Error>> {
        self.audio_handler.discover_devices()
    }

    pub fn is_audio_running(&self) -> bool {
        self.audio_handler.is_audio_running()
    }

    pub fn is_audio_muted(&self) -> bool {
        self.audio_handler.is_muted()
    }

    pub fn start_audio(
        &mut self,
        device_id: Option<i32>,
        sample_rate: u32,
        channels: u32,
    ) -> Result<AudioSettings, Box<dyn Error>> {
        let (settings, should_send_message) =
            self.audio_handler
                .start_audio(device_id, sample_rate, channels)?;

        // Start playback when starting audio
        if let Err(e) = self
            .audio_handler
            .start_playback(settings.sample_rate, settings.channels)
        {
            self.logger
                .warn(&format!("Failed to start audio playback: {}", e));
        }

        if self.connection_started && should_send_message {
            self.send_audio_on_message()?;
        }

        Ok(settings)
    }

    pub fn start_audio_auto(
        &mut self,
        sample_rate: u32,
        channels: u32,
    ) -> Result<AudioSettings, Box<dyn Error>> {
        let settings = self.audio_handler.start_audio_auto(sample_rate, channels)?;

        // Start playback when starting audio
        if let Err(e) = self
            .audio_handler
            .start_playback(settings.sample_rate, settings.channels)
        {
            self.logger
                .warn(&format!("Failed to start audio playback: {}", e));
        }

        if self.connection_started {
            self.send_audio_on_message()?;
        }

        Ok(settings)
    }

    fn send_audio_on_message(&self) -> Result<(), Box<dyn Error>> {
        self.logger.info("Sending AudioOn control message to peer");
        self.media_session
            .send_control_message(ControlMessage::AudioOn)
            .map_err(|e| format!("Failed to send AudioOn message: {}", e).into())
    }

    pub fn stop_audio(&mut self) {
        // Do NOT stop playback here. Playback should continue even if mic is off.
        // self.audio_handler.stop_playback();
        self.audio_handler.stop_audio();

        if self.connection_started {
            self.logger.info("Sending AudioOff control message to peer");
            if let Err(e) = self
                .media_session
                .send_control_message(ControlMessage::AudioOff)
            {
                self.logger
                    .error(&format!("Failed to send AudioOff message: {}", e));
            }
        }
    }

    pub fn mute_audio(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_handler.mute();

        if self.connection_started {
            self.logger
                .info("Sending AudioMuted control message to peer");
            self.media_session
                .send_control_message(ControlMessage::AudioMuted)
                .map_err(|e| format!("Failed to send AudioMuted message: {}", e))?;
        }

        Ok(())
    }

    pub fn unmute_audio(&mut self) -> Result<(), Box<dyn Error>> {
        self.audio_handler.unmute();

        if self.connection_started {
            self.logger
                .info("Sending AudioUnmuted control message to peer");
            self.media_session
                .send_control_message(ControlMessage::AudioUnmuted)
                .map_err(|e| format!("Failed to send AudioUnmuted message: {}", e))?;
        }

        Ok(())
    }

    pub fn toggle_mute(&mut self) -> Result<bool, Box<dyn Error>> {
        let is_now_muted = self.audio_handler.toggle_mute();

        if self.connection_started {
            let message = if is_now_muted {
                ControlMessage::AudioMuted
            } else {
                ControlMessage::AudioUnmuted
            };

            self.media_session
                .send_control_message(message)
                .map_err(|e| format!("Failed to send mute state message: {}", e))?;
        }

        Ok(is_now_muted)
    }

    pub fn capture_audio_frame(&mut self) -> Result<media::AudioFrame, Box<dyn Error>> {
        self.audio_handler.capture_frame()
    }

    // ===== Frame transmission =====

    pub fn send_frame(&self, frame: media::VideoFrame) -> Result<(), Box<dyn Error>> {
        static FRAME_COUNT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let count = FRAME_COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        if count.is_multiple_of(30) {
            self.logger.info(&format!(
                "BACKEND: Sending frame #{} ({}x{})",
                count,
                frame.width(),
                frame.height()
            ));
        }

        self.media_session
            .send_frame(frame)
            .map_err(|e| format!("Failed to send frame: {}", e).into())
    }

    pub fn capture_and_send(&mut self) -> Result<RgbFrame, Box<dyn Error>> {
        let frame = self.camera_handler.capture_frame()?;
        let (width, height, rgb_data) = media::frame_to_rgb(&frame)?;
        let rgb_frame = (width, height, rgb_data);

        self.send_frame(frame)?;
        Ok(rgb_frame)
    }

    /// Captures an audio frame and sends it through the WebRTC session
    pub fn capture_audio_and_send(&mut self) -> Result<(), Box<dyn Error>> {
        let audio_frame = self.audio_handler.capture_frame()?;
        self.media_session
            .send_audio_frame(audio_frame)
            .map_err(|e| format!("Failed to send audio frame: {}", e).into())
    }

    pub fn receive_frame(&self) -> Result<Option<RgbFrame>, Box<dyn Error>> {
        match self.media_session.receive_frame() {
            Ok(Some(frame)) => {
                let (width, height, rgb_data) = media::frame_to_rgb(&frame)?;
                Ok(Some((width, height, rgb_data)))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(format!("Failed to receive frame: {}", e).into()),
        }
    }

    /// Receives all available decoded audio frames and plays them on output device
    pub fn receive_audio(&mut self) -> Result<(), Box<dyn Error>> {
        loop {
            match self.media_session.receive_audio_frame() {
                Ok(Some(audio_frame)) => {
                    // Play audio frame through output device
                    self.audio_handler.play_frame(&audio_frame)?;
                    // Continue to drain channel
                }
                Ok(None) => return Ok(()), // No frame available
                Err(e) => return Err(format!("Failed to receive audio: {}", e).into()),
            }
        }
    }

    pub fn receive_control_message(&self) -> Result<Option<ControlMessage>, Box<dyn Error>> {
        self.media_session
            .receive_control_message()
            .map_err(|e| format!("Failed to receive control message: {}", e).into())
    }

    pub fn send_participant_name(&self, name: &str) -> Result<(), Box<dyn Error>> {
        self.logger
            .info(&format!("SENDING PARTICIPANT NAME: '{}'", name));
        self.logger.info(&format!(
            "connection_started={}, is_secure={}",
            self.connection_started,
            self.media_session.is_secure()
        ));

        self.media_session
            .send_control_message(ControlMessage::ParticipantName(name.to_string()))
            .map_err(|e| {
                let err_msg = format!("Failed to send participant name: {}", e);
                self.logger.error(&err_msg);
                Box::<dyn Error>::from(err_msg)
            })?;

        self.logger
            .info("Participant name control message sent successfully");
        Ok(())
    }

    pub fn send_disconnect_message(&self, is_owner: bool) -> Result<(), Box<dyn Error>> {
        self.logger
            .info(&format!("Sending disconnect (owner: {})", is_owner));

        let message = if is_owner {
            ControlMessage::OwnerDisconnected
        } else {
            ControlMessage::ParticipantDisconnected
        };

        self.media_session
            .send_control_message(message)
            .map_err(|e| format!("Failed to send disconnect message: {}", e).into())
    }

    pub fn get_jitter_stats(&self) -> network::JitterBufferStats {
        self.media_session.get_jitter_stats()
    }

    pub fn get_packet_stats(&self) -> network::PacketStats {
        self.media_session.get_packet_stats()
    }

    pub fn get_rtcp_stats(&self) -> Option<network::RtcpStats> {
        self.media_session.get_rtcp_stats()
    }

    pub fn clear_video_buffers(&self) {
        self.media_session.clear_video_buffers();
    }

    /// Initiates sending a file via SCTP DataChannel
    /// Returns transfer_id for tracking the transfer
    pub fn send_file(&self, path: &std::path::Path) -> Result<u64, Box<dyn Error>> {
        if !self.is_connected() {
            return Err("Cannot send file: Connection not established. Please wait for DTLS handshake to complete.".into());
        }

        self.media_session.send_file(path).map_err(|e| e.into())
    }

    /// Accepts an incoming file transfer via SCTP DataChannel
    pub fn accept_file_transfer(
        &self,
        transfer_id: u64,
        save_path: &std::path::Path,
    ) -> Result<(), Box<dyn Error>> {
        if !self.is_connected() {
            return Err("Cannot accept file: Connection not established".into());
        }

        self.media_session
            .accept_file_transfer(transfer_id, save_path)
            .map_err(|e| e.into())
    }

    /// Rejects an incoming file transfer via SCTP DataChannel
    pub fn reject_file_transfer(
        &self,
        transfer_id: u64,
        reason: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !self.is_connected() {
            return Err("Cannot reject file: Connection not established".into());
        }

        self.media_session
            .reject_file_transfer(transfer_id, reason)
            .map_err(|e| e.into())
    }

    /// Cancels an ongoing file transfer
    pub fn cancel_file_transfer(
        &self,
        transfer_id: u64,
        reason: &str,
    ) -> Result<(), Box<dyn Error>> {
        if !self.is_connected() {
            return Err("Cannot cancel file: Connection not established".into());
        }
        self.media_session
            .cancel_file_transfer(transfer_id, reason)
            .map_err(|e| e.into())
    }

    /// Gets the progress of a file transfer (bytes_transferred, total_bytes)
    pub fn get_file_transfer_progress(&self, transfer_id: u64) -> Option<(u64, u64)> {
        self.media_session.get_file_transfer_progress(transfer_id)
    }

    /// Polls for outgoing SCTP packets and sends them
    /// Should be called regularly in the send thread
    pub fn poll_sctp(&self) -> Result<(), Box<dyn Error>> {
        self.media_session.poll_sctp_send().map_err(|e| e.into())
    }

    /// Demultiplex incoming UDP packets - MUST be called before receive_sctp!
    /// This feeds packets into the DTLS layer so it can decrypt SCTP data
    pub fn unified_receive(&mut self) -> Result<(), Box<dyn Error>> {
        self.media_session.unified_receive().map_err(|e| e.into())
    }

    /// Receives and processes incoming SCTP packets
    /// Should be called regularly in the receive thread
    pub fn receive_sctp(&mut self) -> Result<(), Box<dyn Error>> {
        self.media_session
            .receive_and_process_sctp()
            .map_err(|e| e.into())
    }

    /// Processes incoming SCTP data and sends responses
    /// Should be called when SCTP packets are received
    pub fn process_sctp(&self, data: &[u8]) -> Result<(), Box<dyn Error>> {
        self.media_session
            .process_sctp_data(data)
            .map_err(|e| e.into())
    }

    /// Polls for file transfer events
    /// Returns the next file transfer event if available
    pub fn poll_file_event(&self) -> Option<crate::session::file_transfer::FileTransferEvent> {
        self.media_session.poll_event()
    }

    fn apply_camera_resolution(
        &mut self,
        resolution: CameraResolution,
    ) -> Result<(), Box<dyn Error>> {
        self.logger.info(&format!(
            "Applying resolution: {}x{} @ {:.1}fps",
            resolution.width, resolution.height, resolution.fps
        ));

        let bitrate = self.calculate_bitrate(resolution.width, resolution.height);
        self.update_session_config(resolution, bitrate);
        self.update_encoder(resolution, bitrate)?;

        self.logger.info(&format!(
            "Encoder recreated: {:.2} Mbps",
            bitrate as f64 / 1_000_000.0
        ));
        Ok(())
    }

    fn calculate_bitrate(&self, width: u32, height: u32) -> u32 {
        let pixels = (width * height) as f64;
        let base_pixels = 1280.0 * 720.0;
        ((pixels / base_pixels) * 2_000_000.0) as u32
    }

    fn update_session_config(&mut self, resolution: CameraResolution, bitrate: u32) {
        self.session_config = self
            .session_config
            .clone()
            .with_resolution(resolution.width, resolution.height)
            .with_fps(resolution.fps)
            .with_bitrate(bitrate);
    }

    fn update_encoder(
        &mut self,
        resolution: CameraResolution,
        bitrate: u32,
    ) -> Result<(), Box<dyn Error>> {
        self.media_session
            .update_encoder_resolution(resolution.width, resolution.height, bitrate)
            .map_err(|e| format!("Failed to update encoder resolution: {}", e).into())
    }

    fn apply_remote_endpoint(
        &mut self,
        remote_ip: &str,
        remote_port: u16,
    ) -> Result<(), Box<dyn Error>> {
        let _remote_addr_str = format!("{}:{}", remote_ip, remote_port);
        self.session_config = self.session_config.clone().with_remote_port(remote_port);
        Ok(())
    }

    /// Validates all preconditions for DTLS handshake (debugging aid)
    pub fn validate_dtls_preconditions(&self) -> Result<(), String> {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        // 1. Verify DTLS initialized
        if self.media_session.get_local_fingerprint().is_none() {
            errors.push("DTLS not initialized");
        } else {
            self.logger.info("DTLS initialized");
        }

        // 2. Verify local ICE candidates
        if self.ice_handler.ice_agent.local_candidates.is_empty() {
            errors.push("No local ICE candidates");
        } else {
            self.logger.info(&format!(
                "Local ICE candidates: {}",
                self.ice_handler.ice_agent.local_candidates.len()
            ));
        }

        let has_remote_candidates = !self.ice_handler.ice_agent.remote_candidates.is_empty();
        let has_remote_endpoint = self.ice_handler.get_remote_address().is_some();

        if !has_remote_candidates && !has_remote_endpoint {
            errors.push("No remote ICE candidates AND no remote endpoint (CRITICAL)");
        } else if !has_remote_candidates {
            warnings.push("No remote ICE candidates, but have remote endpoint from SDP");
            self.logger
                .warn("No remote ICE candidates in SDP, using c=/m= lines for connectivity");
        } else {
            self.logger.info(&format!(
                "Remote ICE candidates: {}",
                self.ice_handler.ice_agent.remote_candidates.len()
            ));
        }

        if !self.media_session.has_remote_fingerprint() {
            errors.push("No remote DTLS fingerprint");
        } else {
            self.logger.info("Remote DTLS fingerprint set");
        }

        if !self.is_offerer {
            self.logger.info("Role: ANSWERER (DTLS server)");
        } else {
            self.logger.info("Role: OFFERER (DTLS client)");
        }

        if let Some((ip, port)) = self.ice_handler.get_remote_address() {
            self.logger
                .info(&format!("Remote endpoint: {}:{}", ip, port));
            if ip == "127.0.0.1" || ip == "0.0.0.0" {
                warnings.push("Remote address is localhost/unspecified (may fail across networks)");
            }
        } else {
            errors.push("Cannot determine remote address");
        }

        // Log warnings (non-blocking)
        for warning in &warnings {
            self.logger.warn(warning);
        }

        if errors.is_empty() {
            self.logger.info("All DTLS preconditions satisfied");
            Ok(())
        } else {
            let error_msg = format!("DTLS preconditions failed:\n{}", errors.join("\n"));
            self.logger.error(&error_msg);
            Err(error_msg)
        }
    }

    /// Check if file channel is ready and emit event if not yet emitted
    ///
    /// This method should be called periodically (e.g., from poll_sctp in receive_thread)
    /// to detect when the DataChannel completes its OPEN/ACK handshake.
    ///
    /// Returns true if the FileChannelReady event should be emitted now (first time ready).
    pub fn check_and_emit_file_channel_ready(&mut self) -> bool {
        if self.file_channel_ready_emitted {
            return false;
        }

        let is_ready = self.media_session.is_file_channel_ready();

        if is_ready {
            self.file_channel_ready_emitted = true;
            self.logger
                .info("[FILE] File channel ready - can now send/receive files");
            return true;
        }

        false
    }
}

impl Drop for WebRtcConnection {
    fn drop(&mut self) {
        self.logger.info("Closing secure WebRTC connection");
        self.close();
    }
}

impl WebRtcConnection {
    /// Properly close the WebRTC connection and release all resources
    pub fn close(&mut self) {
        self.logger.info("[CLEANUP] Stopping camera...");
        self.stop_camera();
        self.stop_audio();

        self.logger.info("[CLEANUP] Closing media session...");
        self.media_session.close();

        self.logger.info("[CLEANUP] WebRTC connection closed");
        self.connection_started = false;
        self.file_channel_ready_emitted = false;
    }
}
