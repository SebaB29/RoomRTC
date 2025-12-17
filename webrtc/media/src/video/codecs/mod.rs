//! Video codecs module
//!
//! Video encoding and decoding implementations.

pub mod h264;
pub mod vp8;

pub use h264::{H264Decoder, H264Encoder};
pub use vp8::{VP8Decoder, VP8Encoder};
