//! Audio device information.
//!
//! Stores metadata about detected audio devices including
//! capabilities and supported configurations.

/// Information about an available audio device
#[derive(Debug, Clone)]
pub struct AudioInfo {
    /// Device identifier
    pub device_id: i32,
    /// Device name or path
    pub name: String,
    /// Maximum supported sample rate in Hz
    pub max_sample_rate: u32,
    /// Supported channel counts (mono=1, stereo=2, etc)
    pub supported_channels: Vec<u32>,
}

impl AudioInfo {
    /// Common sample rates supported by most audio devices
    const COMMON_SAMPLE_RATES: [u32; 5] = [8000, 16000, 22050, 44100, 48000];

    /// Creates audio info with detected capabilities
    ///
    /// # Arguments
    /// * `device_id` - Audio device identifier
    /// * `name` - Device name or path
    /// * `max_sample_rate` - Maximum supported sample rate in Hz
    /// * `supported_channels` - List of supported channel counts
    pub fn new(
        device_id: i32,
        name: String,
        max_sample_rate: u32,
        supported_channels: Vec<u32>,
    ) -> Self {
        Self {
            device_id,
            name,
            max_sample_rate,
            supported_channels,
        }
    }

    /// Creates default audio info for a device
    ///
    /// Assumes common defaults: 48kHz, stereo
    pub fn default_for_device(device_id: i32, name: String) -> Self {
        Self {
            device_id,
            name,
            max_sample_rate: 48000,
            supported_channels: vec![1, 2], // mono and stereo
        }
    }

    /// Returns a string representation of the sample rate
    pub fn sample_rate_string(&self) -> String {
        format!("{} Hz", self.max_sample_rate)
    }

    /// Checks if the audio device supports a specific sample rate
    pub fn supports_sample_rate(&self, sample_rate: u32) -> bool {
        sample_rate <= self.max_sample_rate && Self::COMMON_SAMPLE_RATES.contains(&sample_rate)
    }

    /// Checks if the audio device supports a specific channel count
    pub fn supports_channels(&self, channels: u32) -> bool {
        self.supported_channels.contains(&channels)
    }

    /// Returns the recommended sample rate (highest common supported)
    pub fn recommended_sample_rate(&self) -> u32 {
        *Self::COMMON_SAMPLE_RATES
            .iter()
            .rev()
            .find(|&&rate| rate <= self.max_sample_rate)
            .unwrap_or(&48000)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_info_creation() {
        let info = AudioInfo::new(0, "Test Microphone".to_string(), 48000, vec![1, 2]);

        assert_eq!(info.device_id, 0);
        assert_eq!(info.name, "Test Microphone");
        assert_eq!(info.max_sample_rate, 48000);
        assert_eq!(info.supported_channels, vec![1, 2]);
    }

    #[test]
    fn test_default_for_device() {
        let info = AudioInfo::default_for_device(1, "Default Audio".to_string());

        assert_eq!(info.device_id, 1);
        assert_eq!(info.name, "Default Audio");
        assert_eq!(info.max_sample_rate, 48000);
        assert!(info.supports_channels(1));
        assert!(info.supports_channels(2));
    }

    #[test]
    fn test_sample_rate_string() {
        let info = AudioInfo::default_for_device(0, "Test".to_string());
        assert_eq!(info.sample_rate_string(), "48000 Hz");
    }

    #[test]
    fn test_supports_sample_rate() {
        let info = AudioInfo::default_for_device(0, "Test".to_string());

        assert!(info.supports_sample_rate(8000));
        assert!(info.supports_sample_rate(16000));
        assert!(info.supports_sample_rate(44100));
        assert!(info.supports_sample_rate(48000));
        assert!(!info.supports_sample_rate(96000)); // exceeds max
        assert!(!info.supports_sample_rate(32000)); // not in common list
    }

    #[test]
    fn test_supports_channels() {
        let info = AudioInfo::new(0, "Test".to_string(), 48000, vec![1, 2]);

        assert!(info.supports_channels(1));
        assert!(info.supports_channels(2));
        assert!(!info.supports_channels(4));
        assert!(!info.supports_channels(8));
    }

    #[test]
    fn test_recommended_sample_rate() {
        let info1 = AudioInfo::default_for_device(0, "Test".to_string());
        assert_eq!(info1.recommended_sample_rate(), 48000);

        let info2 = AudioInfo::new(1, "Low Quality".to_string(), 22050, vec![1]);
        assert_eq!(info2.recommended_sample_rate(), 22050);

        let info3 = AudioInfo::new(2, "Very Low".to_string(), 7999, vec![1]);
        assert_eq!(info3.recommended_sample_rate(), 48000); // fallback
    }

    #[test]
    fn test_recommended_channels() {
        let info1 = AudioInfo::new(0, "Stereo".to_string(), 48000, vec![1, 2]);
        assert_eq!(info1.recommended_channels(), 2);

        let info2 = AudioInfo::new(1, "Mono".to_string(), 48000, vec![1]);
        assert_eq!(info2.recommended_channels(), 1);
    }

    #[test]
    fn test_audio_info_clone() {
        let info1 = AudioInfo::default_for_device(1, "Device 1".to_string());
        let info2 = info1.clone();

        assert_eq!(info1.device_id, info2.device_id);
        assert_eq!(info1.name, info2.name);
        assert_eq!(info1.max_sample_rate, info2.max_sample_rate);
    }
}
