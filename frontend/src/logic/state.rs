//! Logic Thread State
//!
//! Maintains WebRTC connection state independent of the UI thread.

use logging::Logger;
use std::sync::{Arc, Mutex};
use webrtc::WebRtcConnection;

/// State maintained by the logic thread.
/// This holds the WebRTC connection and is independent of egui.
pub struct LogicState {
    pub webrtc: Option<Arc<Mutex<WebRtcConnection>>>,
    pub camera_thread_handle: Option<std::thread::JoinHandle<()>>,
    pub audio_thread_handle: Option<std::thread::JoinHandle<()>>,
    pub receive_thread_handle: Option<std::thread::JoinHandle<()>>,
    /// Temporary storage for connection being set up (before StartConnection command)
    pub pending_connection: Option<WebRtcConnection>,
    /// Logger for file transfer operations
    pub logger: Option<Logger>,
}

impl LogicState {
    pub fn new() -> Self {
        // Try to create a logger, but don't fail if we can't
        let logger = logging::Logger::with_component(
            "frontend.log".into(),
            logging::LogLevel::Info,
            "Logic".to_string(),
            false,
        )
        .ok();

        Self {
            webrtc: None,
            camera_thread_handle: None,
            audio_thread_handle: None,
            receive_thread_handle: None,
            pending_connection: None,
            logger,
        }
    }

    /// Stop all threads and clean up resources
    pub fn cleanup(&mut self) {
        // WebRtcConnection's Drop impl will send disconnect message automatically
        if let Some(webrtc_arc) = self.webrtc.take()
            && let Ok(mut webrtc) = webrtc_arc.lock()
        {
            if let Some(l) = self.logger
                .as_ref() { l.info("[LOGIC_CLEANUP] Closing WebRTC connection...") }
            webrtc.close();
            if let Some(l) = self.logger
                .as_ref() { l.info("[LOGIC_CLEANUP] WebRTC connection closed") }
        }

        self.camera_thread_handle = None;
        self.audio_thread_handle = None;
        self.receive_thread_handle = None;

        if let Some(l) = self.logger
            .as_ref() { l.info("[LOGIC_CLEANUP] Cleanup complete") }
    }
}

impl Default for LogicState {
    fn default() -> Self {
        Self::new()
    }
}
