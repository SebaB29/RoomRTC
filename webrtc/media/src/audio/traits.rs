//! Audio codec traits for encoding and decoding
//!
//! This module defines the core traits that all audio codecs must implement.
//! These traits allow for codec abstraction and future extensibility.

use super::frame::AudioFrame;
use crate::error::Result;

/// Trait for audio encoders
///
/// Implementors of this trait can encode raw audio frames into
/// compressed formats (Opus, AAC, etc.).
///
/// # Responsibilities
/// - Compress raw audio frames into codec-specific formats
/// - Handle variable bitrate and complexity settings
/// - Maintain encoder state across multiple frames
pub trait AudioEncoder {
    /// Encodes a single audio frame into compressed data
    ///
    /// # Arguments
    /// * `frame` - The raw audio frame to encode
    ///
    /// # Returns
    /// * `Ok(Vec<u8>)` - Encoded data ready for transmission
    /// * `Err` - If encoding fails
    fn encode(&mut self, frame: &AudioFrame) -> Result<Vec<u8>>;

    /// Returns the codec name (e.g., "Opus", "AAC")
    ///
    /// Used for debugging, logging, and codec negotiation in SDP.
    fn get_codec(&self) -> &str;

    /// Returns the configured bitrate in bits per second
    ///
    /// Useful for bandwidth monitoring and adaptation.
    fn get_bitrate(&self) -> u32 {
        0 // Default implementation for backward compatibility
    }
}

/// Trait for audio decoders
///
/// Implementors of this trait can decode compressed audio data back into
/// raw frames (Opus, AAC, etc.).
///
/// # Responsibilities
/// - Decompress codec-specific data into raw audio frames
/// - Handle missing/corrupted packets gracefully
/// - Maintain decoder state across multiple packets
pub trait AudioDecoder {
    /// Decodes compressed data into a raw audio frame
    ///
    /// # Arguments
    /// * `data` - Compressed codec-specific data
    ///
    /// # Returns
    /// * `Ok(AudioFrame)` - Successfully decoded frame
    /// * `Err` - If decoding fails or data is corrupted
    fn decode(&mut self, data: &[u8]) -> Result<AudioFrame>;

    /// Returns the codec name (e.g., "Opus", "AAC")
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
