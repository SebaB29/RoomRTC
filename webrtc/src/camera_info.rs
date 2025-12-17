//! Public camera information types
//!
//! Simple wrapper around internal camera detection to expose
//! only what external users need.

/// Information about an available camera device
///
/// This is a simplified public representation of camera capabilities.
/// External users can query available cameras and select one by device_id.
#[derive(Debug, Clone, PartialEq)]
pub struct CameraInfo {
    /// Device identifier (used to start the camera)
    pub device_id: i32,
    /// Human-readable device name
    pub name: String,
    /// Maximum supported width in pixels
    pub max_width: u32,
    /// Maximum supported height in pixels
    pub max_height: u32,
    /// Commonly supported frame rates
    pub supported_fps: Vec<f64>,
}

impl CameraInfo {
    /// Creates a new CameraInfo from internal media type
    pub(crate) fn from_internal(info: media::CameraInfo) -> Self {
        Self {
            device_id: info.device_id,
            name: info.name,
            max_width: info.max_width,
            max_height: info.max_height,
            supported_fps: info.supported_fps,
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

    /// Returns the recommended FPS (highest supported)
    pub fn recommended_fps(&self) -> f64 {
        *self
            .supported_fps
            .iter()
            .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .unwrap_or(&30.0)
    }
}
