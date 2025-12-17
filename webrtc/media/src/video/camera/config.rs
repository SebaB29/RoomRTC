//! Camera configuration types.
//!
//! Provides configuration options for camera capture including
//! resolution, framerate, and device selection.

use crate::error::{MediaError, Result};

/// Camera capture configuration
#[derive(Debug, Clone)]
pub struct CameraConfig {
    /// Camera device ID (0 for default camera)
    pub device_id: i32,
    /// Frame width in pixels (None = use maximum available)
    pub width: Option<u32>,
    /// Frame height in pixels (None = use maximum available)
    pub height: Option<u32>,
    /// Target frames per second
    pub fps: f64,
}

impl CameraConfig {
    /// Minimum valid FPS value
    const MIN_FPS: f64 = 1.0;
    /// Maximum valid FPS value
    const MAX_FPS: f64 = 240.0;
    /// Minimum valid resolution dimension
    const MIN_DIMENSION: u32 = 1;
    /// Maximum valid resolution dimension (8K)
    const MAX_DIMENSION: u32 = 7680;

    /// Creates a new camera configuration with validation
    ///
    /// # Arguments
    /// * `device_id` - Camera device identifier
    /// * `fps` - Target frames per second (clamped to 1.0-240.0)
    ///
    /// # Returns
    /// * `Ok(CameraConfig)` - Successfully created configuration
    /// * `Err(MediaError::Config)` - If fps is not finite (NaN or infinite)
    pub fn new(device_id: i32, fps: f64) -> Result<Self> {
        if !fps.is_finite() {
            return Err(MediaError::Config(
                "FPS must be a finite number (not NaN or infinite)".to_string(),
            ));
        }

        let fps = fps.clamp(Self::MIN_FPS, Self::MAX_FPS);

        Ok(Self {
            device_id,
            width: None,
            height: None,
            fps,
        })
    }

    /// Sets specific resolution with validation
    ///
    /// # Arguments
    /// * `width` - Frame width in pixels (1-7680)
    /// * `height` - Frame height in pixels (1-4320)
    ///
    /// # Returns
    /// * `Ok(CameraConfig)` - Successfully set resolution
    /// * `Err(MediaError::Config)` - If resolution is invalid (0 or exceeds maximum)
    pub fn with_resolution(mut self, width: u32, height: u32) -> Result<Self> {
        if !(Self::MIN_DIMENSION..=Self::MAX_DIMENSION).contains(&width) {
            return Err(MediaError::Config(format!(
                "Width must be between {} and {}, got {}",
                Self::MIN_DIMENSION,
                Self::MAX_DIMENSION,
                width
            )));
        }

        if !(Self::MIN_DIMENSION..=Self::MAX_DIMENSION).contains(&height) {
            return Err(MediaError::Config(format!(
                "Height must be between {} and {}, got {}",
                Self::MIN_DIMENSION,
                Self::MAX_DIMENSION,
                height
            )));
        }

        self.width = Some(width);
        self.height = Some(height);
        Ok(self)
    }

    /// Returns the resolution as a tuple if configured
    pub fn resolution(&self) -> Option<(u32, u32)> {
        match (self.width, self.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        }
    }
}

/// Default camera configuration (device 0, 30 FPS, max resolution)
impl Default for CameraConfig {
    fn default() -> Self {
        Self {
            device_id: 0,
            width: None,
            height: None,
            fps: 30.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = CameraConfig::default();
        assert_eq!(config.device_id, 0);
        assert_eq!(config.fps, 30.0);
        assert!(config.width.is_none());
        assert!(config.height.is_none());
    }

    #[test]
    fn test_config_with_resolution() {
        let config = CameraConfig::new(0, 60.0)
            .unwrap()
            .with_resolution(1920, 1080)
            .unwrap();
        assert_eq!(config.device_id, 0);
        assert_eq!(config.fps, 60.0);
        assert_eq!(config.width, Some(1920));
        assert_eq!(config.height, Some(1080));
        assert_eq!(config.resolution(), Some((1920, 1080)));
    }

    #[test]
    fn test_fps_clamping() {
        // Too low
        let config = CameraConfig::new(0, 0.5).unwrap();
        assert_eq!(config.fps, 1.0);

        // Too high
        let config = CameraConfig::new(0, 300.0).unwrap();
        assert_eq!(config.fps, 240.0);

        // Normal
        let config = CameraConfig::new(0, 60.0).unwrap();
        assert_eq!(config.fps, 60.0);
    }

    #[test]
    fn test_fps_nan() {
        let result = CameraConfig::new(0, f64::NAN);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MediaError::Config(_)));
    }

    #[test]
    fn test_fps_infinity() {
        let result = CameraConfig::new(0, f64::INFINITY);
        assert!(result.is_err());

        let result = CameraConfig::new(0, f64::NEG_INFINITY);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_width() {
        let result = CameraConfig::new(0, 30.0).unwrap().with_resolution(0, 480);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MediaError::Config(_)));
    }

    #[test]
    fn test_invalid_width_too_large() {
        let result = CameraConfig::new(0, 30.0)
            .unwrap()
            .with_resolution(10000, 480);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_height() {
        let result = CameraConfig::new(0, 30.0).unwrap().with_resolution(640, 0);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), MediaError::Config(_)));
    }

    #[test]
    fn test_invalid_height_too_large() {
        let result = CameraConfig::new(0, 30.0)
            .unwrap()
            .with_resolution(640, 10000);
        assert!(result.is_err());
    }

    #[test]
    fn test_resolution_helpers() {
        let config_with = CameraConfig::new(0, 30.0)
            .unwrap()
            .with_resolution(1920, 1080)
            .unwrap();
        assert_eq!(config_with.resolution(), Some((1920, 1080)));

        let config_without = CameraConfig::new(0, 30.0).unwrap();
        assert_eq!(config_without.resolution(), None);
    }

    #[test]
    fn test_valid_edge_cases() {
        // Minimum valid values
        let config = CameraConfig::new(0, 1.0)
            .unwrap()
            .with_resolution(1, 1)
            .unwrap();
        assert_eq!(config.fps, 1.0);
        assert_eq!(config.resolution(), Some((1, 1)));

        // Maximum valid values
        let config = CameraConfig::new(0, 240.0)
            .unwrap()
            .with_resolution(7680, 7680)
            .unwrap();
        assert_eq!(config.fps, 240.0);
        assert_eq!(config.resolution(), Some((7680, 7680)));
    }
}
