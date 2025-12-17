//! Media Processing Module
//!
//! Handles video and audio capture, encoding/decoding, and format conversion.
//! Provides a clean separation between video and audio subsystems.

pub mod audio;
pub mod common;
pub mod error;
pub mod video;

// Re-export commonly used types
pub use error::MediaError;

// Video exports
pub use video::{
    Camera, CameraConfig, CameraInfo, H264Decoder, H264Encoder, VP8Decoder, VP8Encoder,
    VideoDecoder, VideoEncoder, VideoFrame,
};

// Audio exports
pub use audio::{
    Audio, AudioConfig, AudioDecoder, AudioDetection, AudioEncoder, AudioFrame, AudioInfo,
    AudioSample, OpusDecoder, OpusEncoder,
};

// Convenience re-exports for backward compatibility
pub use video::converters::frame_to_rgb;
