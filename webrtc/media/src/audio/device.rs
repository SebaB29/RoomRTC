//! Audio device management.
//!
//! Core audio capture functionality including initialization,
//! configuration, and audio sample capture operations.

use crate::common::constants::logging::AUDIO_LOG_INTERVAL;
use crate::error::{MediaError, Result};
use logging::Logger;

use super::capture::AudioCapture;
use super::config::AudioConfig;
use super::detection::AudioDetection;
use super::frame::AudioFrame;
use super::playback::AudioPlayback;

/// Audio capture and playback device
///
/// Manages audio device initialization, configuration, sample capture, and playback.
/// Uses platform-specific APIs for cross-platform audio input/output.
pub struct Audio {
    config: AudioConfig,
    logger: Logger,
    frame_count: u64,
    is_capturing: bool,
    is_playing: bool,
    playback_frame_count: u64,
    playback: Option<AudioPlayback>,
    capture: Option<AudioCapture>,
}

impl Audio {
    /// Creates a new audio device with specified configuration
    ///
    /// Initializes the audio input device and prepares for capture.
    ///
    /// # Arguments
    /// * `config` - Audio configuration (device ID, sample rate, channels)
    /// * `logger` - Logger instance for monitoring
    ///
    /// # Returns
    /// * `Ok(Audio)` - Successfully initialized audio device
    /// * `Err(MediaError)` - If audio device cannot be opened or configured
    pub fn new(config: AudioConfig, logger: Logger) -> Result<Self> {
        logger.info(&format!(
            "Initializing audio device {:?} @ {} Hz, {} channel(s)",
            config.device_id, config.sample_rate, config.channels
        ));

        // Verify device availability
        if let Some(device_id) = config.device_id
            && !AudioDetection::is_available(device_id, &logger)
        {
            return Err(MediaError::Audio(format!(
                "Audio device {} is not available",
                device_id
            )));
        }

        logger.info("Audio device initialized successfully");

        Ok(Audio {
            config,
            logger,
            frame_count: 0,
            is_capturing: false,
            is_playing: false,
            playback_frame_count: 0,
            playback: None,
            capture: None,
        })
    }

    /// Creates an audio device with auto-detected default device
    ///
    /// # Arguments
    /// * `sample_rate` - Desired sample rate in Hz
    /// * `channels` - Number of channels (1=mono, 2=stereo)
    /// * `logger` - Logger instance
    ///
    /// # Returns
    /// * `Ok(Audio)` - Successfully initialized audio device
    /// * `Err(MediaError)` - If no audio devices found or initialization fails
    pub fn new_auto(sample_rate: u32, channels: u32, logger: Logger) -> Result<Self> {
        logger.info("Auto-detecting available audio devices...");
        let device = AudioDetection::get_default_device(&logger)?;

        logger.info(&format!(
            "Auto-selected: {} (ID: {}, Max Rate: {})",
            device.name, device.device_id, device.max_sample_rate
        ));

        let config = AudioConfig::new(Some(device.device_id), sample_rate, channels)?;

        Self::new(config, logger)
    }

    /// Starts audio capture
    ///
    /// Begins capturing audio from the device using cpal.
    ///
    /// # Returns
    /// * `Ok(())` - Successfully started capture
    /// * `Err(MediaError)` - If capture cannot be started
    pub fn start_capture(&mut self) -> Result<()> {
        if self.is_capturing {
            return Err(MediaError::Audio("Audio capture already started".into()));
        }

        self.logger.info("Starting audio capture with cpal");
        self.capture = Some(AudioCapture::new(
            self.config.device_id,
            self.config.sample_rate,
            self.config.channels,
            &self.logger,
        )?);
        self.is_capturing = true;
        Ok(())
    }

    /// Stops audio capture
    pub fn stop_capture(&mut self) {
        if self.is_capturing {
            self.logger.info("Stopping audio capture");
            self.capture = None; // Dropping AudioCapture stops the stream
            self.is_capturing = false;
        }
    }

    /// Captures an audio frame
    ///
    /// Reads one buffer of audio samples from the device.
    ///
    /// # Returns
    /// * `Ok(AudioFrame)` - Successfully captured audio frame
    /// * `Err(MediaError)` - If capture fails or device is not capturing
    pub fn capture_frame(&mut self) -> Result<AudioFrame> {
        if !self.is_capturing {
            return Err(MediaError::Audio("Audio capture not started".into()));
        }

        let sample_count = self.config.buffer_size as usize * self.config.channels as usize;

        let samples = if let Some(ref capture) = self.capture {
            capture.read_samples(sample_count)
        } else {
            vec![0i16; sample_count]
        };

        self.frame_count += 1;

        // Log progress periodically
        if self.frame_count.is_multiple_of(AUDIO_LOG_INTERVAL) {
            self.logger
                .debug(&format!("Audio frames captured: {}", self.frame_count));
        }

        Ok(AudioFrame::new(
            samples,
            self.config.channels,
            self.config.sample_rate,
        ))
    }

    /// Returns the total number of frames captured
    pub fn frame_count(&self) -> u64 {
        self.frame_count
    }

    /// Returns the audio configuration
    pub fn config(&self) -> &AudioConfig {
        &self.config
    }

    /// Returns whether audio is currently being captured
    pub fn is_capturing(&self) -> bool {
        self.is_capturing
    }

    /// Starts audio playback
    ///
    /// Begins playing audio to the output device using cpal.
    ///
    /// # Returns
    /// * `Ok(())` - Successfully started playback
    /// * `Err(MediaError)` - If playback cannot be started
    pub fn start_playback(&mut self) -> Result<()> {
        if self.is_playing {
            return Err(MediaError::Audio("Audio playback already started".into()));
        }

        self.logger.info("Starting audio playback with cpal");
        self.playback = Some(AudioPlayback::new(
            self.config.sample_rate,
            self.config.channels,
            &self.logger,
        )?);
        self.is_playing = true;
        Ok(())
    }

    /// Stops audio playback
    pub fn stop_playback(&mut self) {
        if self.is_playing {
            self.logger.info("Stopping audio playback");
            self.playback = None; // Dropping AudioPlayback stops the stream
            self.is_playing = false;
        }
    }

    /// Plays an audio frame
    ///
    /// Sends audio samples to the output device for playback.
    ///
    /// # Arguments
    /// * `frame` - Audio frame to play
    ///
    /// # Returns
    /// * `Ok(())` - Successfully queued frame for playback
    /// * `Err(MediaError)` - If playback fails or device is not in playback mode
    pub fn play_frame(&mut self, frame: &AudioFrame) -> Result<()> {
        if !self.is_playing {
            return Err(MediaError::Audio("Audio playback not started".into()));
        }

        if let Some(ref playback) = self.playback {
            playback.play_samples(&frame.samples);
        }

        self.playback_frame_count += 1;

        if self.playback_frame_count.is_multiple_of(AUDIO_LOG_INTERVAL) {
            self.logger.info(&format!(
                "Playing audio frame #{} ({} samples)",
                self.playback_frame_count,
                frame.samples.len()
            ));
        }

        Ok(())
    }

    /// Returns whether audio is currently being played
    pub fn is_playing(&self) -> bool {
        self.is_playing
    }

    /// Returns the total number of frames played
    pub fn playback_frame_count(&self) -> u64 {
        self.playback_frame_count
    }

    /// Clears the audio capture buffer
    pub fn clear_capture_buffer(&self) {
        if let Some(ref capture) = self.capture {
            capture.clear_buffer();
        }
    }
}

impl Drop for Audio {
    fn drop(&mut self) {
        self.stop_capture();
        self.stop_playback();
        self.logger.info("Audio device released");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;
    use tempfile::tempdir;

    fn create_test_logger() -> Logger {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_audio.log");
        Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    fn create_test_audio() -> Audio {
        let logger = create_test_logger();
        let config = AudioConfig::default();
        Audio::new(config, logger).unwrap()
    }

    #[test]
    fn test_audio_creation() {
        let audio = create_test_audio();
        assert_eq!(audio.frame_count(), 0);
        assert!(!audio.is_capturing());
    }

    #[test]
    fn test_audio_auto() {
        let logger = create_test_logger();
        let result = Audio::new_auto(48000, 2, logger);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_stop_capture() {
        let mut audio = create_test_audio();

        assert!(audio.start_capture().is_ok());
        assert!(audio.is_capturing());

        // Starting again should fail
        assert!(audio.start_capture().is_err());

        audio.stop_capture();
        assert!(!audio.is_capturing());
    }

    #[test]
    fn test_capture_frame() {
        let mut audio = create_test_audio();

        // Should fail before starting capture
        assert!(audio.capture_frame().is_err());

        audio.start_capture().unwrap();

        // Should succeed after starting
        let frame = audio.capture_frame().unwrap();
        assert_eq!(frame.channels, 2);
        assert_eq!(frame.sample_rate, 48000);
        assert!(!frame.samples.is_empty());

        assert_eq!(audio.frame_count(), 1);
    }

    #[test]
    fn test_audio_frame() {
        let samples = vec![0i16; 960 * 2]; // 20ms stereo at 48kHz
        let frame = AudioFrame::new(samples, 2, 48000);

        assert_eq!(frame.channels, 2);
        assert_eq!(frame.sample_rate, 48000);
        assert_eq!(frame.frame_count(), 960);
        assert!((frame.duration_ms() - 20.0).abs() < 0.1);
    }

    #[test]
    fn test_audio_frame_mono() {
        let samples = vec![0i16; 320]; // 20ms mono at 16kHz
        let frame = AudioFrame::new(samples, 1, 16000);

        assert_eq!(frame.channels, 1);
        assert_eq!(frame.frame_count(), 320);
        assert!((frame.duration_ms() - 20.0).abs() < 0.1);
    }
}
