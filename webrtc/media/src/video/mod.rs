//! Video processing module
//!
//! Handles video capture, encoding, decoding, and frame manipulation.

pub mod camera;
pub mod codecs;
pub mod constants;
pub mod converters;
pub mod frame;
pub mod traits;
pub mod utils;

// Re-exports
pub use camera::{Camera, CameraConfig, CameraInfo};
pub use codecs::{H264Decoder, H264Encoder, VP8Decoder, VP8Encoder};
pub use converters::frame_to_rgb;
pub use frame::VideoFrame;
pub use traits::{VideoDecoder, VideoEncoder};
