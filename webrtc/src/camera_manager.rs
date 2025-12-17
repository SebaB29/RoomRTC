//! Camera management module
//!
//! Handles camera initialization, discovery, and lifecycle management

use crate::camera_info::CameraInfo;
use logging::Logger;
use media::video::camera::CameraDetection;
use media::{Camera, CameraConfig};
use std::error::Error;

/// Manages camera devices and their lifecycle
pub struct CameraManager {
    /// Currently active camera
    active_camera: Option<Camera>,
    /// Cached list of available cameras
    available_cameras: Vec<CameraInfo>,
    /// Logger instance
    logger: Logger,
}

impl CameraManager {
    /// Creates a new CameraManager
    pub fn new(logger: Logger) -> Self {
        Self {
            active_camera: None,
            available_cameras: Vec::new(),
            logger,
        }
    }

    /// Discovers all available cameras on the system
    pub fn discover_cameras(&mut self) -> Result<Vec<CameraInfo>, Box<dyn Error>> {
        self.logger.info("Discovering available cameras...");

        let internal_cameras = CameraDetection::list_devices(&self.logger)?;

        self.available_cameras = internal_cameras
            .into_iter()
            .map(CameraInfo::from_internal)
            .collect();

        self.logger.info(&format!(
            "Discovered {} camera(s)",
            self.available_cameras.len()
        ));

        Ok(self.available_cameras.clone())
    }

    /// Starts a camera with the given configuration
    pub fn start_camera(
        &mut self,
        device_id: i32,
        fps: f64,
    ) -> Result<CameraResolution, Box<dyn Error>> {
        if self.active_camera.is_some() {
            self.logger.warn("Camera already started");
            return Err("Camera already running".into());
        }

        self.logger
            .info(&format!("Starting camera {} at {:.1} fps", device_id, fps));

        let config = CameraConfig::new(device_id, fps)?;
        let camera = Camera::new(config, self.logger.clone())?;

        let (width, height) = camera.actual_resolution();
        let actual_fps = camera.actual_fps();

        self.logger.info(&format!(
            "Camera {} started: {}x{} @ {:.1} fps",
            device_id, width, height, actual_fps
        ));

        let resolution = CameraResolution {
            width,
            height,
            fps: actual_fps,
        };

        self.active_camera = Some(camera);

        Ok(resolution)
    }

    /// Starts camera with auto-detection
    pub fn start_camera_auto(&mut self, fps: f64) -> Result<CameraResolution, Box<dyn Error>> {
        if self.active_camera.is_some() {
            self.logger.warn("Camera already started");
            return Err("Camera already running".into());
        }

        self.logger.info("Starting camera with auto-detection");
        let camera = Camera::new_auto(fps, self.logger.clone())?;

        let (width, height) = camera.actual_resolution();
        let actual_fps = camera.actual_fps();

        self.logger.info(&format!(
            "Camera detected: {}x{} @ {:.1} fps",
            width, height, actual_fps
        ));

        let resolution = CameraResolution {
            width,
            height,
            fps: actual_fps,
        };

        self.active_camera = Some(camera);

        Ok(resolution)
    }

    /// Stops the current camera
    pub fn stop_camera(&mut self) {
        if let Some(camera) = self.active_camera.take() {
            self.logger.info("Stopping camera and releasing hardware");
            drop(camera);
        }
    }

    /// Checks if camera is currently running
    pub fn is_running(&self) -> bool {
        self.active_camera.is_some()
    }

    /// Captures a frame from the active camera
    pub fn capture_frame(&mut self) -> Result<media::VideoFrame, Box<dyn Error>> {
        let camera = self.active_camera.as_mut().ok_or("Camera not started")?;
        Ok(camera.capture_frame()?)
    }
}

/// Camera resolution information
#[derive(Debug, Clone, Copy)]
pub struct CameraResolution {
    pub width: u32,
    pub height: u32,
    pub fps: f64,
}
