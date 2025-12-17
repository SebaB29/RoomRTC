//! Audio frame representation.
//!
//! Provides the core `AudioFrame` type for representing raw audio samples
//! with metadata throughout the media processing pipeline.

/// Audio sample data type (16-bit PCM)
pub type AudioSample = i16;

/// Audio frame containing captured samples
#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// Raw audio samples (interleaved for multi-channel)
    pub samples: Vec<AudioSample>,
    /// Number of channels
    pub channels: u32,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Timestamp in milliseconds
    pub timestamp_ms: u64,
}

impl AudioFrame {
    /// Creates a new audio frame
    pub fn new(samples: Vec<AudioSample>, channels: u32, sample_rate: u32) -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            samples,
            channels,
            sample_rate,
            timestamp_ms,
        }
    }

    /// Returns the number of frames (samples per channel)
    pub fn frame_count(&self) -> usize {
        self.samples.len() / self.channels as usize
    }

    /// Returns the duration of this audio frame in milliseconds
    pub fn duration_ms(&self) -> f64 {
        (self.frame_count() as f64 / self.sample_rate as f64) * 1000.0
    }

    /// Returns the size in bytes
    pub fn size_bytes(&self) -> usize {
        self.samples.len() * std::mem::size_of::<AudioSample>()
    }
}
