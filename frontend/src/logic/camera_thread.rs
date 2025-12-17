//! Camera Capture Thread
//!
//! This module handles continuous camera frame capture in a dedicated thread.
//! Frames are captured at a fixed interval and sent to both the encoder and UI.

use crate::events::LogicEvent;
use crate::logic::utils::rgb_to_color_image;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use webrtc::WebRtcConnection;

// Target frame rate control
const TARGET_FPS: f64 = 30.0;
const FRAME_DURATION: Duration = Duration::from_millis((1000.0 / TARGET_FPS) as u64); // 33ms

/// Runs the camera capture loop in a dedicated thread.
///
/// The thread continuously checks if the camera is running and captures frames
/// when available. Frames are sent through the event channel for UI display.
/// Maintains accurate 30 FPS by compensating for capture time.
pub fn run_camera_thread(webrtc_arc: Arc<Mutex<WebRtcConnection>>, evt_tx: Sender<LogicEvent>) {
    loop {
        let frame_start = Instant::now();

        // Capture, send, and get RGB for preview in one operation
        let rgb_result = {
            let Ok(mut conn) = webrtc_arc.lock() else {
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            };

            if !conn.is_camera_running() {
                None
            } else {
                Some(conn.capture_and_send())
            }
        };

        let Some(result) = rgb_result else {
            // Camera is off - sleep and retry
            std::thread::sleep(std::time::Duration::from_millis(100));
            continue;
        };

        match result {
            Ok((width, height, rgb_pixels)) => {
                let color_image = rgb_to_color_image(width, height, rgb_pixels);
                let _ = evt_tx.send(LogicEvent::LocalFrame(color_image));

                let elapsed = frame_start.elapsed();
                if elapsed < FRAME_DURATION {
                    std::thread::sleep(FRAME_DURATION - elapsed);
                }
            }
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(500));
            }
        }
    }
}
