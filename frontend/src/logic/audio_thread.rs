//! Audio Capture Thread
//!
//! This module handles continuous audio capture in a dedicated thread.
//! Audio frames are captured at a fixed interval (20ms Opus frames) and sent to the encoder.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use webrtc::WebRtcConnection;

// Opus frame duration: 20ms is optimal for WebRTC
const AUDIO_FRAME_DURATION: Duration = Duration::from_millis(20);

/// Runs the audio capture loop in a dedicated thread.
///
/// The thread continuously checks if the microphone is running and captures audio frames
/// when available. Frames are sent directly through WebRTC (no UI display needed).
/// Maintains accurate 20ms timing for Opus encoding.
pub fn run_audio_thread(webrtc_arc: Arc<Mutex<WebRtcConnection>>) {
    loop {
        let frame_start = Instant::now();

        // Capture and send audio frame
        let capture_result = {
            let Ok(mut conn) = webrtc_arc.lock() else {
                std::thread::sleep(Duration::from_millis(100));
                continue;
            };

            if !conn.is_microphone_running() {
                None
            } else {
                Some(conn.capture_audio_and_send())
            }
        };

        let Some(result) = capture_result else {
            // Microphone is off - sleep and retry
            std::thread::sleep(Duration::from_millis(100));
            continue;
        };

        match result {
            Ok(_) => {
                // Successfully captured and sent audio
                let elapsed = frame_start.elapsed();
                if elapsed < AUDIO_FRAME_DURATION {
                    std::thread::sleep(AUDIO_FRAME_DURATION - elapsed);
                }
            }
            Err(_) => {
                // Error capturing audio - retry after short delay
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}
