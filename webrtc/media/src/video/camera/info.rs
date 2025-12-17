//! Camera device information.
//!
//! Stores metadata about detected camera devices including
//! capabilities and supported configurations.

/// Information about an available camera device
#[derive(Debug, Clone)]
pub struct CameraInfo {
    /// Device identifier
    pub device_id: i32,
    /// Device name or path
    pub name: String,
    /// Maximum supported width in pixels
    pub max_width: u32,
    /// Maximum supported height in pixels
    pub max_height: u32,
    /// Commonly supported frame rates
    pub supported_fps: Vec<f64>,
}

impl CameraInfo {
    /// Common frame rates supported by most cameras
    const COMMON_FPS: [f64; 4] = [15.0, 24.0, 30.0, 60.0];

    /// Creates camera info with detected capabilities
    ///
    /// # Arguments
    /// * `device_id` - Camera device identifier
    /// * `name` - Device name or path
    /// * `max_width` - Maximum supported width
    /// * `max_height` - Maximum supported height
    pub fn new(device_id: i32, name: String, max_width: u32, max_height: u32) -> Self {
        Self {
            device_id,
            name,
            max_width,
            max_height,
            supported_fps: Self::COMMON_FPS.to_vec(),
        }
    }

    /// Returns a string representation of the resolution
    pub fn resolution_string(&self) -> String {
        format!("{}x{}", self.max_width, self.max_height)
    }

    /// Checks if the camera supports a specific frame rate
    pub fn supports_fps(&self, fps: f64) -> bool {
        self.supported_fps.iter().any(|&f| (f - fps).abs() < 0.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_camera_info_creation() {
        let info = CameraInfo::new(0, "Test Camera".to_string(), 1920, 1080);

        assert_eq!(info.device_id, 0);
        assert_eq!(info.name, "Test Camera");
        assert_eq!(info.max_width, 1920);
        assert_eq!(info.max_height, 1080);
        assert!(!info.supported_fps.is_empty());
    }

    #[test]
    fn test_resolution_string() {
        let info = CameraInfo::new(0, "Test".to_string(), 1920, 1080);
        assert_eq!(info.resolution_string(), "1920x1080");
    }

    #[test]
    fn test_supports_fps_exact_match() {
        let info = CameraInfo::new(0, "Test".to_string(), 640, 480);

        assert!(info.supports_fps(30.0));
        assert!(info.supports_fps(60.0));
        assert!(info.supports_fps(15.0));
        assert!(info.supports_fps(24.0));
    }

    #[test]
    fn test_supports_fps_not_supported() {
        let info = CameraInfo::new(0, "Test".to_string(), 640, 480);

        assert!(!info.supports_fps(120.0));
        assert!(!info.supports_fps(45.0));
        assert!(!info.supports_fps(1.0));
    }

    #[test]
    fn test_supports_fps_tolerance() {
        let info = CameraInfo::new(0, "Test".to_string(), 640, 480);

        // Within tolerance (< 0.1)
        assert!(info.supports_fps(30.05));
        assert!(info.supports_fps(59.95));
    }

    #[test]
    fn test_camera_info_clone() {
        let info1 = CameraInfo::new(1, "Camera 1".to_string(), 1280, 720);
        let info2 = info1.clone();

        assert_eq!(info1.device_id, info2.device_id);
        assert_eq!(info1.name, info2.name);
        assert_eq!(info1.max_width, info2.max_width);
        assert_eq!(info1.max_height, info2.max_height);
    }
}
