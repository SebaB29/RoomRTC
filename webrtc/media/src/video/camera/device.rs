//! Camera device management.
//!
//! Core camera capture functionality including initialization,
//! configuration, and frame capture operations.

use crate::common::constants::logging::CAMERA_LOG_INTERVAL;
use crate::error::{MediaError, Result};
use crate::video::frame::VideoFrame;
use logging::Logger;
use opencv::prelude::*;
use opencv::videoio::{CAP_ANY, VideoCapture};

use super::config::CameraConfig;
use super::detection::CameraDetection;

/// Video capture device
///
/// Manages camera initialization, configuration, frame capture, and cleanup.
/// Uses OpenCV VideoCapture for cross-platform camera access.
pub struct Camera {
    capture: VideoCapture,
    config: CameraConfig,
    logger: Logger,
    frame_count: u64,
    actual_width: u32,
    actual_height: u32,
    actual_fps: f64,
}

/// Camera configuration result after initialization
struct CameraSettings {
    width: u32,
    height: u32,
    fps: f64,
}

impl Camera {
    /// Creates a new camera with specified configuration
    ///
    /// Initializes the camera device and applies resolution/FPS settings.
    /// Logs warnings if requested settings cannot be applied exactly.
    ///
    /// # Arguments
    /// * `config` - Camera configuration (device ID, resolution, FPS)
    /// * `logger` - Logger instance for monitoring
    ///
    /// # Returns
    /// * `Ok(Camera)` - Successfully initialized camera
    /// * `Err(MediaError)` - If camera cannot be opened or configured
    pub fn new(config: CameraConfig, logger: Logger) -> Result<Self> {
        logger.info(&format!(
            "Initializing camera ID {} @ {} fps",
            config.device_id, config.fps
        ));

        let mut capture = VideoCapture::new(config.device_id, CAP_ANY)
            .map_err(|e| MediaError::Camera(format!("Failed to open camera: {}", e)))?;

        if !Self::is_opened(&capture)? {
            return Err(MediaError::Camera("Camera is not available".to_string()));
        }

        let settings = Self::configure(&mut capture, &config, &logger)?;

        logger.info("Camera initialized successfully");

        Ok(Camera {
            capture,
            config,
            logger,
            frame_count: 0,
            actual_width: settings.width,
            actual_height: settings.height,
            actual_fps: settings.fps,
        })
    }

    /// Creates a camera with auto-detected maximum resolution
    ///
    /// Automatically selects the first available camera and uses its maximum resolution.
    /// Prioritizes cameras with higher resolution and common aspect ratios.
    ///
    /// # Arguments
    /// * `fps` - Desired frames per second
    /// * `logger` - Logger instance
    ///
    /// # Returns
    /// * `Ok(Camera)` - Successfully initialized camera
    /// * `Err(MediaError)` - If no cameras found or initialization fails
    pub fn new_auto(fps: f64, logger: Logger) -> Result<Self> {
        logger.info("Auto-detecting available cameras...");
        let devices = CameraDetection::list_devices(&logger)?;

        // Select best camera: prioritize by resolution (area)
        let device = devices
            .iter()
            .max_by_key(|d| d.max_width * d.max_height)
            .ok_or_else(|| {
                MediaError::Camera(
                    "No camera devices found. Please connect a camera and try again.".to_string(),
                )
            })?;

        logger.info(&format!(
            "Auto-selected: {} (ID: {}, Resolution: {}x{}, FPS: {:.1})",
            device.name, device.device_id, device.max_width, device.max_height, fps
        ));

        let config = CameraConfig {
            device_id: device.device_id,
            width: Some(device.max_width),
            height: Some(device.max_height),
            fps,
        };

        Self::new(config, logger)
    }

    /// Captures a single frame from the camera
    ///
    /// Reads one frame from the video capture device and converts it to VideoFrame.
    /// Tracks frame count for debugging and monitoring.
    ///
    /// # Returns
    /// * `Ok(VideoFrame)` - Successfully captured frame
    /// * `Err(MediaError)` - If capture fails or frame is empty
    pub fn capture_frame(&mut self) -> Result<VideoFrame> {
        let mut mat = Mat::default();

        let success = self
            .capture
            .read(&mut mat)
            .map_err(|e| MediaError::Camera(format!("Failed to read frame: {}", e)))?;

        if !success || mat.empty() {
            return Err(MediaError::Camera("Empty or invalid frame".to_string()));
        }

        // Additional validation: check dimensions
        if mat.cols() == 0 || mat.rows() == 0 {
            return Err(MediaError::Camera("Invalid frame dimensions".to_string()));
        }

        self.frame_count += 1;

        // Log progress periodically
        if self.frame_count.is_multiple_of(CAMERA_LOG_INTERVAL) {
            self.logger
                .debug(&format!("Frames captured: {}", self.frame_count));
        }

        Ok(VideoFrame::new(mat))
    }

    /// Returns the total number of frames captured
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Returns the camera configuration
    pub fn config(&self) -> &CameraConfig {
        &self.config
    }

    /// Returns the actual resolution being used
    pub fn actual_resolution(&self) -> (u32, u32) {
        (self.actual_width, self.actual_height)
    }

    /// Returns the actual FPS being used
    pub fn actual_fps(&self) -> f64 {
        self.actual_fps
    }

    /// Configures camera capture parameters
    ///
    /// Sets resolution, framerate, and verifies actual values.
    /// Returns CameraSettings with actual configured values.
    fn configure(
        capture: &mut VideoCapture,
        config: &CameraConfig,
        logger: &Logger,
    ) -> Result<CameraSettings> {
        Self::apply_camera_settings(capture, config)?;
        let settings = Self::read_actual_settings(capture)?;
        Self::log_configuration(&settings, config, logger);
        Ok(settings)
    }

    /// Applies requested settings to camera
    fn apply_camera_settings(capture: &mut VideoCapture, config: &CameraConfig) -> Result<()> {
        use opencv::videoio::{CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH};

        if let Some((width, height)) = config.resolution() {
            let _ = capture.set(CAP_PROP_FRAME_WIDTH, f64::from(width));
            let _ = capture.set(CAP_PROP_FRAME_HEIGHT, f64::from(height));
        }
        let _ = capture.set(CAP_PROP_FPS, config.fps);
        Ok(())
    }

    /// Reads actual camera settings after configuration
    fn read_actual_settings(capture: &VideoCapture) -> Result<CameraSettings> {
        use opencv::videoio::{CAP_PROP_FPS, CAP_PROP_FRAME_HEIGHT, CAP_PROP_FRAME_WIDTH};

        let width = Self::get_property(capture, CAP_PROP_FRAME_WIDTH)? as u32;
        let height = Self::get_property(capture, CAP_PROP_FRAME_HEIGHT)? as u32;
        let fps = Self::get_property(capture, CAP_PROP_FPS)?;

        Ok(CameraSettings { width, height, fps })
    }

    /// Logs configuration results and warnings for mismatches
    fn log_configuration(settings: &CameraSettings, config: &CameraConfig, logger: &Logger) {
        logger.info(&format!(
            "Camera configured: {}x{} @ {:.1} FPS",
            settings.width, settings.height, settings.fps
        ));

        // Warn on resolution mismatch
        if let Some((req_w, req_h)) = config.resolution()
            && (settings.width != req_w || settings.height != req_h)
        {
            logger.warn(&format!(
                "Resolution mismatch (got: {}x{}, requested: {}x{})",
                settings.width, settings.height, req_w, req_h
            ));
        }

        // Warn on FPS mismatch
        if (settings.fps - config.fps).abs() > 1.0 {
            logger.warn(&format!(
                "FPS mismatch (got: {:.1}, requested: {:.1})",
                settings.fps, config.fps
            ));
        }
    }

    /// Safely gets a camera property
    fn get_property(capture: &VideoCapture, prop: i32) -> Result<f64> {
        capture
            .get(prop)
            .map_err(|e| MediaError::Camera(format!("Error getting property: {}", e)))
    }

    /// Verifies if camera is opened and ready
    fn is_opened(capture: &VideoCapture) -> Result<bool> {
        capture
            .is_opened()
            .map_err(|e| MediaError::Camera(format!("Error verifying camera status: {}", e)))
    }
}

impl Drop for Camera {
    /// Releases camera resources when dropped
    ///
    /// Logs final statistics and ensures proper cleanup of OpenCV resources.
    fn drop(&mut self) {
        self.logger.info(&format!(
            "Closing camera. Total frames captured: {}",
            self.frame_count
        ));

        if let Err(e) = self.capture.release() {
            self.logger.error(&format!("Error releasing camera: {}", e));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;
    use tempfile::tempdir;

    fn create_test_logger() -> Logger {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_camera.log");
        Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    #[test]
    fn test_camera_invalid_id() {
        let logger = create_test_logger();
        let config = CameraConfig::new(999, 30.0).unwrap();
        let result = Camera::new(config, logger);
        assert!(result.is_err());
    }
}
