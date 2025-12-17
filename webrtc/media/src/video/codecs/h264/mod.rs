//! H.264 (AVC) codec implementation
//!
//! This module provides H.264 video encoding and decoding functionality
//! using FFmpeg's libx264.

pub mod decoder;
pub mod encoder;

pub use decoder::H264Decoder;
pub use encoder::H264Encoder;
