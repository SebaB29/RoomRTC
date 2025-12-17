//! Opus audio codec module
//!
//! Provides Opus encoding and decoding for audio streaming.
//! Opus is the standard audio codec for WebRTC.

mod decoder;
mod encoder;

pub use decoder::OpusDecoder;
pub use encoder::OpusEncoder;
