//! Camera capture module
//!
//! Provides camera device detection, configuration, and video capture.

pub mod config;
pub mod detection;
pub mod device;
pub mod info;
pub mod pool;

pub use config::CameraConfig;
pub use detection::CameraDetection;
pub use device::Camera;
pub use info::CameraInfo;
pub use pool::{CameraJob, CameraThreadPool, EncodedFrame};
