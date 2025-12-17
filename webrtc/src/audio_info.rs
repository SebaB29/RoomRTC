//! Public audio information types
//!
//! Simple wrapper around internal audio detection to expose
//! only what external users need.

/// Information about an available audio device
///
/// This is a simplified public representation of audio device capabilities.
/// External users can query available audio devices and select one by device_id.
#[derive(Debug, Clone, PartialEq)]
pub struct AudioInfo {
    /// Device identifier (used to start the audio device)
    pub device_id: i32,
    /// Human-readable device name
    pub name: String,
    /// Maximum supported sample rate in Hz
    pub max_sample_rate: u32,
    /// Supported channel counts (1=mono, 2=stereo, etc)
    pub supported_channels: Vec<u32>,
}

impl AudioInfo {
    /// Creates a new AudioInfo from internal media type
    pub(crate) fn from_internal(info: media::AudioInfo) -> Self {
        Self {
            device_id: info.device_id,
            name: info.name,
            max_sample_rate: info.max_sample_rate,
            supported_channels: info.supported_channels,
        }
    }

    /// Returns a string representation of the sample rate
    pub fn sample_rate_string(&self) -> String {
        format!("{} Hz", self.max_sample_rate)
    }

    /// Checks if the audio device supports a specific sample rate
    pub fn supports_sample_rate(&self, sample_rate: u32) -> bool {
        sample_rate <= self.max_sample_rate
    }

    /// Checks if the audio device supports a specific channel count
    pub fn supports_channels(&self, channels: u32) -> bool {
        self.supported_channels.contains(&channels)
    }

    /// Returns the recommended sample rate (highest supported)
    pub fn recommended_sample_rate(&self) -> u32 {
        self.max_sample_rate
    }

    /// Returns the recommended channel count (stereo if supported, else mono)
    pub fn recommended_channels(&self) -> u32 {
        if self.supported_channels.contains(&2) {
            2 // stereo
        } else {
            1 // mono
        }
    }
}
