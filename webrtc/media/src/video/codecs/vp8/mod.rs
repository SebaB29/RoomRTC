//! VP8 codec implementation
//!
//! This module provides VP8 video encoding and decoding functionality
//! using FFmpeg's libvpx.
//!

pub mod decoder;
pub mod encoder;

pub use decoder::VP8Decoder;
pub use encoder::VP8Encoder;
