//! Audio configuration types.
//!
//! Provides configuration options for audio capture including
//! sample rate, channels, and device selection.

use crate::error::{MediaError, Result};

/// Audio capture configuration
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Audio device ID (None = default device)
    pub device_id: Option<i32>,
    /// Sample rate in Hz (e.g., 48000 for 48kHz)
    pub sample_rate: u32,
    /// Number of audio channels (1 = mono, 2 = stereo)
    pub channels: u32,
    /// Buffer size in frames
    pub buffer_size: u32,
}

impl AudioConfig {
    /// Minimum valid sample rate
    const MIN_SAMPLE_RATE: u32 = 8000;
    /// Maximum valid sample rate
    const MAX_SAMPLE_RATE: u32 = 192000;
    /// Minimum valid channel count
    const MIN_CHANNELS: u32 = 1;
    /// Maximum valid channel count
    const MAX_CHANNELS: u32 = 8;
    /// Minimum valid buffer size
    const MIN_BUFFER_SIZE: u32 = 64;
    /// Maximum valid buffer size
    const MAX_BUFFER_SIZE: u32 = 8192;

    /// Creates a new audio configuration with validation
    ///
    /// # Arguments
    /// * `device_id` - Audio device identifier (None for default)
    /// * `sample_rate` - Sample rate in Hz (clamped to 8000-192000)
    /// * `channels` - Number of channels (clamped to 1-8)
    ///
    /// # Returns
    /// * `Ok(AudioConfig)` - Successfully created configuration
    /// * `Err(MediaError::Config)` - If parameters are invalid
    pub fn new(device_id: Option<i32>, sample_rate: u32, channels: u32) -> Result<Self> {
        let sample_rate = sample_rate.clamp(Self::MIN_SAMPLE_RATE, Self::MAX_SAMPLE_RATE);
        let channels = channels.clamp(Self::MIN_CHANNELS, Self::MAX_CHANNELS);

        // Default buffer size: 20ms of audio at given sample rate
        let buffer_size = (sample_rate / 50).clamp(Self::MIN_BUFFER_SIZE, Self::MAX_BUFFER_SIZE);

        Ok(Self {
            device_id,
            sample_rate,
            channels,
            buffer_size,
        })
    }

    /// Sets custom buffer size with validation
    ///
    /// # Arguments
    /// * `buffer_size` - Buffer size in frames (64-8192)
    ///
    /// # Returns
    /// * `Ok(AudioConfig)` - Successfully set buffer size
    /// * `Err(MediaError::Config)` - If buffer size is invalid
    pub fn with_buffer_size(mut self, buffer_size: u32) -> Result<Self> {
        if !(Self::MIN_BUFFER_SIZE..=Self::MAX_BUFFER_SIZE).contains(&buffer_size) {
            return Err(MediaError::Config(format!(
                "Buffer size must be between {} and {}, got {}",
                Self::MIN_BUFFER_SIZE,
                Self::MAX_BUFFER_SIZE,
                buffer_size
            )));
        }

        self.buffer_size = buffer_size;
        Ok(self)
    }

    /// Returns the buffer duration in milliseconds
    pub fn buffer_duration_ms(&self) -> f64 {
        (self.buffer_size as f64 / self.sample_rate as f64) * 1000.0
    }

    /// Returns the number of bytes per sample
    pub fn bytes_per_sample(&self) -> usize {
        2 // 16-bit audio = 2 bytes per sample
    }

    /// Returns the total bytes per frame (all channels)
    pub fn bytes_per_frame(&self) -> usize {
        self.bytes_per_sample() * self.channels as usize
    }

    /// Returns the buffer size in bytes
    pub fn buffer_size_bytes(&self) -> usize {
        self.buffer_size as usize * self.bytes_per_frame()
    }
}

/// Default audio configuration (default device, 48kHz, stereo, 960 frames)
impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            device_id: None,
            sample_rate: 48000,
            channels: 2,
            buffer_size: 960, // 20ms at 48kHz
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AudioConfig::default();
        assert!(config.device_id.is_none());
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
        assert_eq!(config.buffer_size, 960);
    }

    #[test]
    fn test_config_with_device() {
        let config = AudioConfig::new(Some(1), 48000, 2).unwrap();
        assert_eq!(config.device_id, Some(1));
        assert_eq!(config.sample_rate, 48000);
        assert_eq!(config.channels, 2);
    }

    #[test]
    fn test_sample_rate_clamping() {
        // Too low
        let config = AudioConfig::new(None, 4000, 2).unwrap();
        assert_eq!(config.sample_rate, 8000);

        // Too high
        let config = AudioConfig::new(None, 200000, 2).unwrap();
        assert_eq!(config.sample_rate, 192000);

        // Normal
        let config = AudioConfig::new(None, 44100, 2).unwrap();
        assert_eq!(config.sample_rate, 44100);
    }

    #[test]
    fn test_channels_clamping() {
        // Too low
        let config = AudioConfig::new(None, 48000, 0).unwrap();
        assert_eq!(config.channels, 1);

        // Too high
        let config = AudioConfig::new(None, 48000, 16).unwrap();
        assert_eq!(config.channels, 8);

        // Normal
        let config = AudioConfig::new(None, 48000, 2).unwrap();
        assert_eq!(config.channels, 2);
    }

    #[test]
    fn test_buffer_size_calculation() {
        let config = AudioConfig::new(None, 48000, 2).unwrap();
        // 48000 / 50 = 960 (20ms)
        assert_eq!(config.buffer_size, 960);

        let config = AudioConfig::new(None, 16000, 1).unwrap();
        // 16000 / 50 = 320 (20ms)
        assert_eq!(config.buffer_size, 320);
    }

    #[test]
    fn test_with_buffer_size() {
        let config = AudioConfig::new(None, 48000, 2)
            .unwrap()
            .with_buffer_size(1024)
            .unwrap();
        assert_eq!(config.buffer_size, 1024);
    }

    #[test]
    fn test_buffer_size_validation() {
        let config = AudioConfig::new(None, 48000, 2).unwrap();

        // Too small
        let result = config.clone().with_buffer_size(32);
        assert!(result.is_err());

        // Too large
        let result = config.clone().with_buffer_size(10000);
        assert!(result.is_err());

        // Valid
        let result = config.with_buffer_size(512);
        assert!(result.is_ok());
    }

    #[test]
    fn test_buffer_duration_ms() {
        let config = AudioConfig::new(None, 48000, 2).unwrap();
        assert!((config.buffer_duration_ms() - 20.0).abs() < 0.1);

        let config = AudioConfig::new(None, 16000, 1).unwrap();
        assert!((config.buffer_duration_ms() - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_bytes_calculations() {
        let config = AudioConfig::new(None, 48000, 2).unwrap();

        assert_eq!(config.bytes_per_sample(), 2); // 16-bit
        assert_eq!(config.bytes_per_frame(), 4); // 2 channels * 2 bytes
        assert_eq!(config.buffer_size_bytes(), 960 * 4); // 960 frames * 4 bytes

        let config = AudioConfig::new(None, 48000, 1).unwrap();
        assert_eq!(config.bytes_per_frame(), 2); // 1 channel * 2 bytes
    }
}
