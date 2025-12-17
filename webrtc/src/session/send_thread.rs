//! Send thread functionality for secure P2P session

use logging::Logger;
use media::{AudioFrame, H264Encoder, OpusEncoder, VideoFrame};
use network::{H264RtpPacketizer, OpusRtpPacketizer, RtpPacketizer, SecureUdpTransport};
use std::sync::mpsc::{Receiver, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Type alias for encoded frame with optional SPS/PPS to reduce type complexity
type EncodedFrame = (Vec<Vec<u8>>, Option<Vec<u8>>, Option<Vec<u8>>);

/// Type alias for NAL packets (SPS/PPS, cached, frame) to reduce type complexity
type NalPackets = (Vec<Vec<u8>>, Vec<Vec<u8>>, Vec<Vec<u8>>);

/// Parameters for send thread
pub(super) struct SendThreadParams {
    pub encoder: Arc<Mutex<H264Encoder>>,
    pub packetizer: Arc<Mutex<H264RtpPacketizer>>,
    pub audio_encoder: Arc<Mutex<OpusEncoder>>,
    pub audio_packetizer: Arc<Mutex<OpusRtpPacketizer>>,
    pub transport: Arc<Mutex<Option<SecureUdpTransport>>>,
    pub rx_encode: Receiver<VideoFrame>,
    pub rx_audio_encode: Receiver<AudioFrame>,
    pub logger: Logger,
}

struct SendThreadState {
    frame_count: u32,
    audio_frame_count: u32,
    sps_pps_sent: bool,
    packet_count: u64,
    audio_packet_count: u64,
}

pub(super) fn run_send_thread(params: SendThreadParams) {
    params
        .logger
        .info("Secure SEND thread started (video + audio)");
    let mut state = SendThreadState {
        frame_count: 0,
        audio_frame_count: 0,
        sps_pps_sent: false,
        packet_count: 0,
        audio_packet_count: 0,
    };

    loop {
        let mut had_activity = false;

        // Check for video frames
        match params.rx_encode.try_recv() {
            Ok(frame) => {
                had_activity = true;
                state.frame_count += 1;
                log_frame_received(
                    &params.logger,
                    state.frame_count,
                    &frame,
                    state.packet_count,
                );

                if let Err(e) = process_frame(&params, &mut state, frame) {
                    params
                        .logger
                        .error(&format!("Frame processing error: {}", e));
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                params
                    .logger
                    .info("Secure SEND thread: video channel disconnected");
            }
        }

        // Check for audio frames
        match params.rx_audio_encode.try_recv() {
            Ok(audio_frame) => {
                had_activity = true;
                state.audio_frame_count += 1;

                if state.audio_frame_count.is_multiple_of(100) {
                    params.logger.debug(&format!(
                        "🎤 SEND: Audio frame {} ({} samples)",
                        state.audio_frame_count,
                        audio_frame.samples.len()
                    ));
                }

                if let Err(e) = process_audio_frame(&params, &mut state, audio_frame) {
                    params
                        .logger
                        .error(&format!("Audio processing error: {}", e));
                }
            }
            Err(TryRecvError::Empty) => {}
            Err(TryRecvError::Disconnected) => {
                params
                    .logger
                    .info("Secure SEND thread: audio channel disconnected");
            }
        }

        if !had_activity {
            thread::sleep(Duration::from_millis(1));
        }
    }
}

fn log_frame_received(logger: &Logger, frame_count: u32, frame: &VideoFrame, packet_count: u64) {
    logger.info(&format!(
        "🎥 SEND: Received frame {} from channel ({}x{})",
        frame_count,
        frame.width(),
        frame.height()
    ));

    if frame_count.is_multiple_of(100) {
        logger.info(&format!(
            "Sent {} frames, {} packets",
            frame_count, packet_count
        ));
    }
}

fn process_frame(
    params: &SendThreadParams,
    state: &mut SendThreadState,
    frame: VideoFrame,
) -> Result<(), String> {
    let (encoded_packets, cached_sps, cached_pps) =
        encode_frame(&params.encoder, &frame, &params.logger, state.sps_pps_sent)?;

    let (cached_packets, sps_pps_packets, frame_packets) = prepare_nal_packets(
        encoded_packets,
        cached_sps,
        cached_pps,
        state.sps_pps_sent,
        &params.logger,
    )?;

    if !state.sps_pps_sent && (!sps_pps_packets.is_empty() || !cached_packets.is_empty()) {
        params.logger.info(&format!(
            "Sending {} SPS/PPS parameter sets",
            sps_pps_packets.len() + cached_packets.len()
        ));
        state.sps_pps_sent = true;
    }

    let nal_count = send_all_nals(
        &cached_packets,
        &sps_pps_packets,
        &frame_packets,
        params,
        &mut state.packet_count,
    )?;

    params.logger.info(&format!(
        "Frame {} complete: {} NALs, {} total packets sent",
        state.frame_count, nal_count, state.packet_count
    ));

    Ok(())
}

fn encode_frame(
    encoder: &Arc<Mutex<H264Encoder>>,
    frame: &VideoFrame,
    logger: &Logger,
    sps_pps_sent: bool,
) -> Result<EncodedFrame, String> {
    let mut encoder_guard = encoder.lock().unwrap_or_else(|poisoned| {
        logger.error("Encoder mutex poisoned in send thread, recovering");
        poisoned.into_inner()
    });

    let packets = encoder_guard
        .encode(frame)
        .map_err(|e| format!("Encoding failed: {}", e))?;

    let sps = if !sps_pps_sent {
        encoder_guard.get_sps().cloned()
    } else {
        None
    };

    let pps = if !sps_pps_sent {
        encoder_guard.get_pps().cloned()
    } else {
        None
    };

    Ok((packets, sps, pps))
}

fn prepare_nal_packets(
    encoded_packets: Vec<Vec<u8>>,
    cached_sps: Option<Vec<u8>>,
    cached_pps: Option<Vec<u8>>,
    sps_pps_sent: bool,
    logger: &Logger,
) -> Result<NalPackets, String> {
    let (sps_pps_packets, frame_packets): (Vec<_>, Vec<_>) = encoded_packets
        .into_iter()
        .filter(|data| !data.is_empty())
        .partition(|data| matches!(get_nal_type(data), 7 | 8));

    let mut cached_packets = Vec::new();
    if sps_pps_packets.is_empty() && !sps_pps_sent {
        if let Some(sps) = cached_sps {
            logger.info("Using cached SPS from encoder");
            cached_packets.push(sps);
        }
        if let Some(pps) = cached_pps {
            logger.info("Using cached PPS from encoder");
            cached_packets.push(pps);
        }
    }

    Ok((cached_packets, sps_pps_packets, frame_packets))
}

fn send_all_nals(
    cached_packets: &[Vec<u8>],
    sps_pps_packets: &[Vec<u8>],
    frame_packets: &[Vec<u8>],
    params: &SendThreadParams,
    packet_count: &mut u64,
) -> Result<usize, String> {
    let mut nal_count = 0;

    for h264_data in cached_packets
        .iter()
        .chain(sps_pps_packets)
        .chain(frame_packets)
    {
        nal_count += 1;
        send_nal(h264_data, nal_count, params, packet_count)?;
    }

    Ok(nal_count)
}

fn send_nal(
    h264_data: &[u8],
    nal_count: usize,
    params: &SendThreadParams,
    packet_count: &mut u64,
) -> Result<(), String> {
    let nal_type = get_nal_type(h264_data);
    let rtp_packets = params
        .packetizer
        .lock()
        .unwrap_or_else(|poisoned| {
            params
                .logger
                .error("Packetizer mutex poisoned in send thread, recovering");
            poisoned.into_inner()
        })
        .packetize(h264_data);

    params.logger.debug(&format!(
        "NAL #{}: type={}, size={} bytes → {} RTP packets",
        nal_count,
        nal_type,
        h264_data.len(),
        rtp_packets.len()
    ));

    for packet in rtp_packets {
        *packet_count += 1;
        send_rtp_packet(&packet, *packet_count, &params.transport, &params.logger)?;
    }

    Ok(())
}

fn send_rtp_packet(
    packet: &network::codec::rtp::RtpPacket,
    packet_count: u64,
    transport: &Arc<Mutex<Option<SecureUdpTransport>>>,
    logger: &Logger,
) -> Result<(), String> {
    let mut transport_guard = transport.lock().unwrap_or_else(|poisoned| {
        logger.error("Transport mutex poisoned in send thread, recovering");
        poisoned.into_inner()
    });

    if let Some(transport) = transport_guard.as_mut() {
        if let Err(e) = transport.send_rtp(packet) {
            logger.error(&format!(
                "SEND ERROR: Failed to send RTP packet #{}: {}",
                packet_count, e
            ));
        } else if packet_count.is_multiple_of(100) {
            logger.debug(&format!(
                "Sent RTP packet #{}, seq={}",
                packet_count, packet.header.sequence_number
            ));
        }
    } else {
        return Err("TRANSPORT ERROR: Not initialized - DTLS handshake required".to_string());
    }

    Ok(())
}

pub(super) fn get_nal_type(data: &[u8]) -> u8 {
    data.windows(4)
        .position(|w| w == [0x00, 0x00, 0x00, 0x01])
        .and_then(|i| data.get(i + 4).map(|&b| b & 0x1F))
        .or_else(|| {
            data.windows(3)
                .position(|w| w == [0x00, 0x00, 0x01])
                .and_then(|i| data.get(i + 3).map(|&b| b & 0x1F))
        })
        .unwrap_or(0)
}

/// Process audio frame: encode and send
fn process_audio_frame(
    params: &SendThreadParams,
    state: &mut SendThreadState,
    audio_frame: AudioFrame,
) -> Result<(), String> {
    // Encode audio frame
    let encoded_audio = {
        let mut encoder = params
            .audio_encoder
            .lock()
            .map_err(|e| format!("Encoder lock error: {}", e))?;

        encoder
            .encode(&audio_frame)
            .map_err(|e| format!("Audio encoding failed: {}", e))?
    };

    if encoded_audio.is_empty() {
        // Encoder needs more data, skip this frame
        return Ok(());
    }

    // Packetize encoded audio
    let rtp_packets = {
        let mut packetizer = params
            .audio_packetizer
            .lock()
            .map_err(|e| format!("Packetizer lock error: {}", e))?;

        packetizer.packetize(&encoded_audio)
    };

    // Send packets
    let mut transport = params
        .transport
        .lock()
        .map_err(|e| format!("Transport lock error: {}", e))?;

    if let Some(ref mut secure_transport) = *transport {
        for packet in rtp_packets {
            secure_transport
                .send_rtp(&packet)
                .map_err(|e| format!("Failed to send audio RTP: {}", e))?;
            state.audio_packet_count += 1;
        }
    }

    Ok(())
}
