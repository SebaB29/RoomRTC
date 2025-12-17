//! Video decoding thread
//!
//! This module handles the dedicated thread for popping frames from the jitter buffer,
//! depacketizing, decoding H264, and sending frames to the application.
//! This decouples the heavy decoding workload from the network reception thread.

use logging::Logger;
use media::{H264Decoder, VideoFrame};
use network::{H264RtpDepacketizer, JitterBuffer, RtpDepacketizer};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use super::send_thread::get_nal_type;

/// Parameters for the video decode thread
pub struct VideoDecodeThreadParams {
    pub jitter_buffer: Arc<Mutex<JitterBuffer>>,
    pub decoder: Arc<Mutex<H264Decoder>>,
    pub tx_decode: SyncSender<VideoFrame>,
    pub logger: Logger,
}

pub fn run_video_decode_thread(params: VideoDecodeThreadParams) {
    params.logger.info("Video Decode thread started");

    let mut depacketizer = H264RtpDepacketizer::new();
    let mut frames_decoded: u64 = 0;

    loop {
        // Try to pop a packet from the jitter buffer
        let packet = {
            let mut jitter = params.jitter_buffer.lock().unwrap_or_else(|poisoned| {
                params
                    .logger
                    .error("Jitter buffer mutex poisoned, recovering");
                poisoned.into_inner()
            });
            jitter.pop()
        };

        if let Some(packet) = packet {
            // Process packet
            if let Some(nal_data) = depacketizer.process_packet(&packet) {
                let nal_type = get_nal_type(&nal_data);

                // Decode
                let decoded_result = {
                    let mut decoder = params.decoder.lock().unwrap_or_else(|poisoned| {
                        params.logger.error("Decoder mutex poisoned, recovering");
                        poisoned.into_inner()
                    });
                    // Log decode attempt here if needed for deep debugging, but keep clean for now
                    decoder.decode(&nal_data)
                };

                match decoded_result {
                    Ok(Some(frame)) => {
                        frames_decoded += 1;
                        if frames_decoded.is_multiple_of(30) {
                            params.logger.info(&format!(
                                "DECODED frame #{}: {}x{}",
                                frames_decoded,
                                frame.width(),
                                frame.height()
                            ));
                        }

                        if params.tx_decode.try_send(frame).is_err() {
                            params
                                .logger
                                .warn("Failed to send decoded frame (channel full)");
                        }
                    }
                    Ok(None) => {
                        // Decoder needs more data
                    }
                    Err(e) => {
                        params
                            .logger
                            .error(&format!("Decode error (NAL type {}): {}", nal_type, e));
                    }
                }
            }
        } else {
            // No packets in jitter buffer, sleep briefly to avoid busy loop
            // The jitter buffer 'pop' is non-blocking effectively if we hold the lock,
            // but the jitter buffer logic usually returns None if not ready.
            // We sleep a bit to give recv_thread time to push.
            thread::sleep(Duration::from_millis(1));
        }
    }
}
