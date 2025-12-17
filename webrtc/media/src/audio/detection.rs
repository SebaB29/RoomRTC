//! Audio device detection and enumeration.
//!
//! Platform-specific utilities for discovering and probing available
//! audio input devices on the system.

use crate::error::Result;
use logging::Logger;

use super::info::AudioInfo;

/// Audio device detection and enumeration
pub struct AudioDetection;

impl AudioDetection {
    /// Lists all available audio input devices
    ///
    /// Scans for audio input devices on the system.
    /// On Linux: Uses ALSA/PulseAudio to enumerate devices
    /// On other platforms: Uses platform-specific APIs
    ///
    /// # Arguments
    /// * `logger` - Logger for debug information
    ///
    /// # Returns
    /// * `Ok(Vec<AudioInfo>)` - List of detected audio input devices
    /// * `Err` - If detection fails critically
    pub fn list_devices(logger: &Logger) -> Result<Vec<AudioInfo>> {
        logger.info("Scanning for available audio input devices...");

        #[cfg(target_os = "linux")]
        let devices = Self::list_devices_linux(logger);

        #[cfg(target_os = "windows")]
        let devices = Self::list_devices_windows(logger);

        #[cfg(target_os = "macos")]
        let devices = Self::list_devices_macos(logger);

        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        let devices = {
            logger.warn("Audio device detection not supported on this platform");
            vec![AudioInfo::default_for_device(
                0,
                "Default Audio Device".to_string(),
            )]
        };

        if devices.is_empty() {
            logger.warn("No audio input devices detected");
        } else {
            logger.info(&format!("Found {} audio input device(s)", devices.len()));
        }

        Ok(devices)
    }

    /// Linux-specific device enumeration using ALSA
    #[cfg(target_os = "linux")]
    fn list_devices_linux(logger: &Logger) -> Vec<AudioInfo> {
        use std::process::Command;

        let mut devices = Vec::new();

        // Try to use 'arecord -l' to list recording devices
        if let Ok(output) = Command::new("arecord").arg("-l").output()
            && output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                devices.extend(Self::parse_arecord_output(&stdout, logger));
            }

        // Always include default device
        if devices.is_empty() {
            logger.info("Using default audio device");
            devices.push(AudioInfo::default_for_device(
                0,
                "Default Audio Input".to_string(),
            ));
        }

        devices
    }

    /// Parses arecord -l output to extract device information
    #[cfg(target_os = "linux")]
    fn parse_arecord_output(output: &str, logger: &Logger) -> Vec<AudioInfo> {
        let mut devices = Vec::new();
        let mut device_id = 0;

        for line in output.lines() {
            if line.contains("card") && line.contains("device") {
                // Extract device name from line like: "card 0: PCH [HDA Intel PCH], device 0: ..."
                let parts: Vec<&str> = line.split(':').collect();
                if parts.len() >= 2 {
                    let section = parts[1].trim();
                    // Extract text between brackets
                    if let Some(start) = section.find('[')
                        && let Some(end) = section.find(']') {
                            let device_name = section[start + 1..end].trim().to_string();
                            logger.debug(&format!("Found audio device: {}", device_name));
                            devices.push(AudioInfo::default_for_device(device_id, device_name));
                            device_id += 1;
                        }
                }
            }
        }

        devices
    }

    /// Windows-specific device enumeration
    #[cfg(target_os = "windows")]
    fn list_devices_windows(_logger: &Logger) -> Vec<AudioInfo> {
        // For now, return default device
        // In a full implementation, would use Windows Audio APIs (WASAPI)
        vec![AudioInfo::default_for_device(
            0,
            "Default Audio Input".to_string(),
        )]
    }

    /// macOS-specific device enumeration
    #[cfg(target_os = "macos")]
    fn list_devices_macos(_logger: &Logger) -> Vec<AudioInfo> {
        // For now, return default device
        // In a full implementation, would use Core Audio APIs
        vec![AudioInfo::default_for_device(
            0,
            "Default Audio Input".to_string(),
        )]
    }

    /// Checks if a specific audio device is available
    ///
    /// # Arguments
    /// * `device_id` - Device identifier to check
    /// * `logger` - Logger for debug information
    ///
    /// # Returns
    /// * `true` if device exists and is accessible
    /// * `false` otherwise
    pub fn is_available(device_id: i32, logger: &Logger) -> bool {
        logger.debug(&format!(
            "Checking availability of audio device {}",
            device_id
        ));

        match Self::list_devices(logger) {
            Ok(devices) => devices.iter().any(|d| d.device_id == device_id),
            Err(_) => false,
        }
    }

    /// Gets information about the default audio input device
    ///
    /// # Arguments
    /// * `logger` - Logger for debug information
    ///
    /// # Returns
    /// * `Ok(AudioInfo)` - Default device information
    /// * `Err` - If no devices are available
    pub fn get_default_device(logger: &Logger) -> Result<AudioInfo> {
        let devices = Self::list_devices(logger)?;
        devices
            .into_iter()
            .next()
            .ok_or_else(|| crate::error::MediaError::Audio("No audio input devices found".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;
    use tempfile::tempdir;

    fn create_test_logger() -> Logger {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_audio_detection.log");
        Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    #[test]
    fn test_list_devices() {
        let logger = create_test_logger();
        let result = AudioDetection::list_devices(&logger);
        assert!(result.is_ok());

        let devices = result.unwrap();
        // Should at least have default device
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_get_default_device() {
        let logger = create_test_logger();
        let result = AudioDetection::get_default_device(&logger);
        assert!(result.is_ok());

        let device = result.unwrap();
        assert_eq!(device.device_id, 0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn test_parse_arecord_output() {
        let logger = create_test_logger();
        let sample_output = "card 0: PCH [HDA Intel PCH], device 0: ALC3246 Analog [ALC3246 Analog]\n\
                             card 1: USB [USB Audio], device 0: USB Audio [USB Audio]";

        let devices = AudioDetection::parse_arecord_output(sample_output, &logger);
        assert_eq!(devices.len(), 2);
        assert_eq!(devices[0].name, "HDA Intel PCH");
        assert_eq!(devices[1].name, "USB Audio");
    }
}
