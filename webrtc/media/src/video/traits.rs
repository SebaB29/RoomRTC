//! Video codec traits for encoding and decoding
//!
//! This module defines the core traits that all video codecs must implement.
//! These traits allow for codec abstraction and future extensibility.

use super::frame::VideoFrame;
use crate::error::Result;

/// Trait for video encoders
///
/// Implementors of this trait can encode raw video frames into
/// compressed formats (H.264, VP8, VP9, AV1, etc.).
///
/// # Responsibilities
/// - Compress raw video frames into codec-specific formats
/// - Handle keyframe requests and bitrate adjustments
/// - Maintain encoder state across multiple frames
pub trait VideoEncoder {
    /// Encodes a single video frame into compressed data
    ///
    /// # Arguments
    /// * `frame` - The raw video frame to encode
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Encoded data ready for transmission
    /// * `Err` - If encoding fails
    ///
    /// # Notes
    /// The returned data may contain one or more NAL units depending
    /// on the codec. For H.264, this could include SPS/PPS/IDR frames.
    fn encode(&mut self, frame: &VideoFrame) -> Result<Vec<u8>>;

    /// Returns the codec name (e.g., "H264", "VP8", "VP9", "AV1")
    ///
    /// Used for debugging, logging, and codec negotiation in SDP.
    fn get_codec(&self) -> &str;

    /// Returns the configured bitrate in bits per second
    ///
    /// Useful for bandwidth monitoring and adaptation.
    fn get_bitrate(&self) -> u32 {
        0 // Default implementation for backward compatibility
    }

    /// Requests a keyframe (I-frame) on the next encode call
    ///
    /// Used when packet loss is detected or when a new peer joins.
    fn request_keyframe(&mut self) {
        // Default: no-op, codecs can override
    }
}

/// Trait for video decoders
///
/// Implementors of this trait can decode compressed video data back into
/// raw frames (H.264, VP8, VP9, AV1, etc.).
///
/// # Responsibilities
///
/// - Decompress codec-specific data into raw video frames
/// - Handle missing/corrupted frames gracefully
/// - Maintain decoder state across multiple packets
pub trait VideoDecoder {
    /// Decodes compressed data into a raw video frame
    ///
    /// # Arguments
    /// * `data` - Compressed codec-specific data
    ///
    /// # Returns
    /// * `Ok(VideoFrame)` - Successfully decoded frame
    /// * `Err` - If decoding fails or data is corrupted
    ///
    /// # Notes
    /// Some codecs may require multiple calls with fragmented data
    /// before a complete frame can be decoded.
    fn decode(&mut self, data: &[u8]) -> Result<VideoFrame>;

    /// Returns the codec name (e.g., "H264", "VP8", "VP9", "AV1")
    ///
    /// Used for debugging, logging, and codec negotiation in SDP.
    fn get_codec(&self) -> &str;

    /// Resets decoder state
    ///
    /// Called when switching streams or recovering from errors.
    fn reset(&mut self) {
        // Default: no-op, codecs can override
    }
}
