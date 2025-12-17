//! Camera management functionality for WebRTC connection

use crate::camera_info::CameraInfo;
use crate::camera_manager::{CameraManager, CameraResolution};
use logging::Logger;
use media::VideoFrame;
use media::video::camera::CameraDetection;
use std::error::Error;

/// Handles all camera-related operations
pub(super) struct CameraHandler {
    camera_manager: CameraManager,
    logger: Logger,
}

impl CameraHandler {
    pub fn new(logger: Logger) -> Self {
        Self {
            camera_manager: CameraManager::new(logger.clone()),
            logger,
        }
    }

    pub fn list_camera_ids_fast() -> Vec<i32> {
        CameraDetection::list_device_ids_fast()
    }

    pub fn discover_cameras(&mut self) -> Result<Vec<CameraInfo>, Box<dyn Error>> {
        self.camera_manager.discover_cameras()
    }

    pub fn is_camera_running(&self) -> bool {
        self.camera_manager.is_running()
    }

    pub fn start_camera(
        &mut self,
        camera_index: i32,
        fps: f64,
    ) -> Result<(CameraResolution, bool), Box<dyn Error>> {
        self.logger.info(&format!(
            "START_CAMERA called: device={}, fps={}",
            camera_index, fps
        ));

        let resolution = self.camera_manager.start_camera(camera_index, fps)?;
        self.logger.info("Camera started");
        Ok((resolution, true))
    }

    pub fn start_camera_auto(&mut self, fps: f64) -> Result<CameraResolution, Box<dyn Error>> {
        let resolution = self.camera_manager.start_camera_auto(fps)?;
        self.logger.info("Camera started automatically");
        Ok(resolution)
    }

    pub fn stop_camera(&mut self) {
        self.camera_manager.stop_camera();
        self.logger.info("Camera stopped");
    }

    pub fn capture_frame(&mut self) -> Result<VideoFrame, Box<dyn Error>> {
        self.camera_manager.capture_frame()
    }
}
