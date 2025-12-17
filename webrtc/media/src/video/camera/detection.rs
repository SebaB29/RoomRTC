//! Camera device detection and enumeration.
//!
//! Platform-specific utilities for discovering and probing available
//! camera devices on the system.

use crate::error::Result;
use logging::Logger;
use opencv::prelude::*;
use opencv::videoio::{CAP_ANY, VideoCapture};
use std::time::{Duration, Instant};

use super::info::CameraInfo;

/// Camera device detection and enumeration
pub struct CameraDetection;

impl CameraDetection {
    /// Lists all available camera devices with timeout protection
    ///
    /// Scans for video devices on the system and tests each one for usability.
    /// On Linux: Checks /dev/video* devices
    /// On other platforms: Tests common device IDs (0-3)
    ///
    /// # Arguments
    /// * `logger` - Logger for debug information
    ///
    /// # Returns
    /// * `Ok(Vec<CameraInfo>)` - List of detected working cameras
    /// * `Err` - If detection fails critically
    pub fn list_devices(logger: &Logger) -> Result<Vec<CameraInfo>> {
        logger.info("Scanning for available cameras...");

        let device_ids = Self::enumerate_device_ids();
        logger.info(&format!(
            "Found {} potential video device(s) to check",
            device_ids.len()
        ));

        let mut cameras = Vec::new();

        for device_id in device_ids {
            logger.debug(&format!("Checking device {}...", device_id));

            if let Some(info) = Self::probe_device(device_id, logger) {
                logger.info(&format!(
                    "Device {}: {} - max {}x{}",
                    device_id, info.name, info.max_width, info.max_height
                ));
                cameras.push(info);
            } else {
                logger.debug(&format!("Device {} is not usable", device_id));
            }
        }

        if cameras.is_empty() {
            logger.warn("No cameras detected");
        } else {
            logger.info(&format!("Found {} working camera(s)", cameras.len()));
        }

        Ok(cameras)
    }

    /// Probes a single device to determine if it's usable
    ///
    /// Uses a short timeout to avoid hanging on non-responsive devices.
    /// Returns device information if usable, None otherwise.
    fn probe_device(device_id: i32, logger: &Logger) -> Option<CameraInfo> {
        const PROBE_TIMEOUT: Duration = Duration::from_millis(100);
        let start = Instant::now();

        Self::check_device_exists_linux(device_id)?;
        let mut capture = Self::open_capture_with_timeout(device_id, start, PROBE_TIMEOUT)?;
        let (width, height) = Self::get_device_resolution(&capture, device_id, logger);
        let name = Self::get_device_name(device_id);
        let _ = capture.release();

        Some(CameraInfo::new(device_id, name, width, height))
    }

    /// Checks if device exists on Linux systems
    #[cfg(target_os = "linux")]
    fn check_device_exists_linux(device_id: i32) -> Option<()> {
        use std::path::Path;
        if Path::new(&format!("/dev/video{}", device_id)).exists() {
            Some(())
        } else {
            None
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn check_device_exists_linux(_device_id: i32) -> Option<()> {
        Some(())
    }

    /// Opens capture device with timeout protection
    fn open_capture_with_timeout(
        device_id: i32,
        start: Instant,
        timeout: Duration,
    ) -> Option<VideoCapture> {
        let mut capture = VideoCapture::new(device_id, CAP_ANY).ok()?;

        if start.elapsed() > timeout {
            let _ = capture.release();
            return None;
        }

        if !capture.is_opened().unwrap_or(false) {
            let _ = capture.release();
            return None;
        }

        Some(capture)
    }

    /// Gets device resolution with validation
    fn get_device_resolution(
        capture: &VideoCapture,
        device_id: i32,
        logger: &Logger,
    ) -> (u32, u32) {
        let width = Self::get_property(capture, opencv::videoio::CAP_PROP_FRAME_WIDTH)
            .unwrap_or(640.0) as u32;
        let height = Self::get_property(capture, opencv::videoio::CAP_PROP_FRAME_HEIGHT)
            .unwrap_or(480.0) as u32;

        Self::validate_resolution(width, height, device_id, logger)
    }

    /// Validates and sanitizes resolution values
    fn validate_resolution(width: u32, height: u32, device_id: i32, logger: &Logger) -> (u32, u32) {
        const MAX_WIDTH: u32 = 7680; // 8K width
        const MAX_HEIGHT: u32 = 4320; // 8K height
        const DEFAULT_WIDTH: u32 = 640;
        const DEFAULT_HEIGHT: u32 = 480;

        if width > 0 && height > 0 && width <= MAX_WIDTH && height <= MAX_HEIGHT {
            (width, height)
        } else {
            logger.debug(&format!(
                "Device {} reported invalid resolution {}x{}, using {}x{}",
                device_id, width, height, DEFAULT_WIDTH, DEFAULT_HEIGHT
            ));
            (DEFAULT_WIDTH, DEFAULT_HEIGHT)
        }
    }

    /// Checks if a specific camera device is available
    ///
    /// Fast check without fully initializing the camera.
    ///
    /// # Arguments
    /// * `device_id` - Device identifier to check
    ///
    /// # Returns
    /// * `true` if device exists and can be opened
    pub fn is_device_available(device_id: i32) -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::path::Path;
            // On Linux, just check if device file exists
            Path::new(&format!("/dev/video{}", device_id)).exists()
        }

        #[cfg(target_os = "macos")]
        {
            // On macOS, try quick open/close test
            if let Ok(mut capture) = VideoCapture::new(device_id, CAP_ANY) {
                let is_open = capture.is_opened().unwrap_or(false);
                let _ = capture.release();
                return is_open;
            }
            false
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            // Other platforms: try to open and check
            if let Ok(mut capture) = VideoCapture::new(device_id, CAP_ANY) {
                let is_open = capture.is_opened().unwrap_or(false);
                let _ = capture.release();
                return is_open;
            }
            false
        }
    }

    /// Checks if any camera is available on the system
    ///
    /// Fast check that stops at the first available device.
    pub fn is_any_device_available() -> bool {
        #[cfg(target_os = "linux")]
        {
            use std::fs;
            // On Linux, just check if any /dev/video* exists
            if let Ok(entries) = fs::read_dir("/dev") {
                return entries
                    .flatten()
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .any(|name| name.starts_with("video"));
            }
            false
        }

        #[cfg(not(target_os = "linux"))]
        {
            // On other platforms, check common device IDs
            let device_ids = Self::enumerate_device_ids();
            device_ids.iter().any(|&id| Self::is_device_available(id))
        }
    }

    /// Enumerates potential device IDs for Linux platforms
    ///
    /// Linux: Scans /dev/video* files (even IDs only to avoid duplicates)
    ///
    /// Note: On Linux, cameras often appear as both even (video0, video2) and odd
    /// (video1, video3) device nodes, where even nodes are typically the actual
    /// camera and odd nodes are metadata. We scan even IDs only for efficiency.
    #[cfg(target_os = "linux")]
    fn enumerate_device_ids() -> Vec<i32> {
        use std::fs;

        let mut device_ids = Vec::with_capacity(4);

        // Scan /dev for video devices efficiently
        if let Ok(entries) = fs::read_dir("/dev") {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    // Match video* devices and extract ID directly
                    if let Some(id_str) = name.strip_prefix("video")
                        && let Ok(id) = id_str.parse::<i32>() {
                            // Only use even IDs (primary devices) to avoid duplicates
                            if id % 2 == 0 && id < 20 {
                                device_ids.push(id);
                            }
                        }
                }
            }
        }

        // Sort for predictable order
        device_ids.sort_unstable();

        // Fallback to device 0 if no devices found
        if device_ids.is_empty() {
            device_ids.push(0);
        }

        device_ids
    }

    /// Enumerates potential device IDs for non-Linux platforms
    ///
    /// Returns a list of common camera device IDs.
    #[cfg(not(target_os = "linux"))]
    fn enumerate_device_ids() -> Vec<i32> {
        vec![0, 1, 2, 3]
    }

    /// Gets a readable device name based on platform
    fn get_device_name(device_id: i32) -> String {
        #[cfg(target_os = "linux")]
        {
            // Try to read friendly name from v4l2
            if let Ok(name) = Self::get_v4l2_device_name(device_id) {
                return name;
            }
            format!("Linux Camera {}", device_id)
        }

        #[cfg(target_os = "macos")]
        {
            // macOS typically uses device ID directly
            format!("macOS Camera {}", device_id)
        }

        #[cfg(not(any(target_os = "linux", target_os = "macos")))]
        {
            format!("Camera {}", device_id)
        }
    }

    /// Attempts to get friendly device name from v4l2 on Linux
    #[cfg(target_os = "linux")]
    fn get_v4l2_device_name(device_id: i32) -> std::io::Result<String> {
        use std::fs;
        let path = format!("/sys/class/video4linux/video{}/name", device_id);
        let name = fs::read_to_string(path)?;
        Ok(name.trim().to_string())
    }

    /// Safely gets a camera property
    fn get_property(capture: &VideoCapture, prop: i32) -> Option<f64> {
        capture.get(prop).ok()
    }

    /// Lists camera device IDs without testing connections (fast)
    ///
    /// Only checks for device file existence on Linux (/dev/video*) or
    /// returns common IDs (0-3) on other platforms.
    /// Does NOT open or test any camera connections.
    ///
    /// # Returns
    /// * List of potentially available camera device IDs
    pub fn list_device_ids_fast() -> Vec<i32> {
        Self::enumerate_device_ids()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;
    use tempfile::tempdir;

    fn create_test_logger() -> Logger {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_detection.log");
        Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    #[test]
    fn test_is_device_available() {
        // Test that function doesn't panic
        let _ = CameraDetection::is_device_available(0);
        let _ = CameraDetection::is_device_available(999);
    }

    #[test]
    fn test_list_device_ids_fast() {
        let ids = CameraDetection::list_device_ids_fast();
        // Should return at least one ID
        assert!(!ids.is_empty());
        // IDs should be non-negative
        assert!(ids.iter().all(|&id| id >= 0));
    }

    #[test]
    fn test_enumerate_device_ids() {
        let ids = CameraDetection::enumerate_device_ids();
        // Should return at least a fallback device
        assert!(!ids.is_empty());
        // IDs should be sorted and unique
        for i in 1..ids.len() {
            assert!(ids[i] >= ids[i - 1]);
        }
    }

    #[test]
    fn test_list_devices() {
        let logger = create_test_logger();
        let result = CameraDetection::list_devices(&logger);
        // Should not panic, may or may not find devices
        assert!(result.is_ok());
    }
}
