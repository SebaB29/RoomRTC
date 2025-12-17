//! Audio management module
//!
//! Handles audio device initialization, discovery, and lifecycle management

use crate::audio_info::AudioInfo;
use logging::Logger;
use media::audio::{AudioCapture, AudioPlayback};
use media::{AudioDetection, AudioInfo as MediaAudioInfo};
use std::error::Error;

/// Manages audio devices and their lifecycle
pub struct AudioManager {
    /// Active audio capture (microphone)
    capture: Option<AudioCapture>,
    /// Active audio playback (speakers)
    playback: Option<AudioPlayback>,
    /// Cached list of available audio devices
    available_devices: Vec<AudioInfo>,
    /// Logger instance
    logger: Logger,
}

impl AudioManager {
    /// Creates a new AudioManager
    pub fn new(logger: Logger) -> Self {
        Self {
            capture: None,
            playback: None,
            available_devices: Vec::new(),
            logger,
        }
    }

    /// Discovers all available audio input devices on the system
    pub fn discover_devices(&mut self) -> Result<Vec<AudioInfo>, Box<dyn Error>> {
        self.logger.info("Discovering available audio devices...");

        let internal_devices: Vec<MediaAudioInfo> = AudioDetection::list_devices(&self.logger)?;

        self.available_devices = internal_devices
            .into_iter()
            .map(AudioInfo::from_internal)
            .collect();

        self.logger.info(&format!(
            "Discovered {} audio device(s)",
            self.available_devices.len()
        ));

        Ok(self.available_devices.clone())
    }

    /// Starts audio capture (microphone)
    pub fn start_capture(
        &mut self,
        device_id: Option<i32>,
        sample_rate: u32,
        channels: u32,
    ) -> Result<AudioSettings, Box<dyn Error>> {
        if self.capture.is_some() {
            self.logger.warn("Audio capture already started");
            return Err("Audio capture already running".into());
        }

        self.logger.info(&format!(
            "Starting audio capture {:?} at {} Hz, {} channel(s)",
            device_id, sample_rate, channels
        ));

        let capture = AudioCapture::new(device_id, sample_rate, channels, &self.logger)?;

        // Log the actual settings
        let settings = AudioSettings {
            sample_rate: capture.sample_rate(),
            channels: capture.channels(),
            buffer_size: 0, // Not exposed by AudioCapture directly currently
        };

        self.logger.info(&format!(
            "Audio capture started: {} Hz, {} channel(s)",
            settings.sample_rate, settings.channels
        ));

        self.capture = Some(capture);

        Ok(settings)
    }

    /// Starts audio capture with auto-detection
    pub fn start_capture_auto(
        &mut self,
        sample_rate: u32,
        channels: u32,
    ) -> Result<AudioSettings, Box<dyn Error>> {
        if self.capture.is_some() {
            self.logger.warn("Audio capture already started");
            return Err("Audio capture already running".into());
        }

        self.logger
            .info("Starting audio capture with auto-detection");

        // Use default device logic
        let device = AudioDetection::get_default_device(&self.logger)?;
        self.logger.info(&format!(
            "Auto-selected input device: {} (ID: {})",
            device.name, device.device_id
        ));

        self.start_capture(Some(device.device_id), sample_rate, channels)
    }

    /// Stops audio capture
    pub fn stop_capture(&mut self) {
        if self.capture.is_some() {
            self.logger.info("Stopping audio capture");
            self.capture = None;
        }
    }

    /// Checks if audio capture is running
    pub fn is_capturing(&self) -> bool {
        self.capture.is_some()
    }

    /// Captures an audio frame
    pub fn capture_frame(&mut self) -> Result<media::AudioFrame, Box<dyn Error>> {
        let capture = self.capture.as_mut().ok_or("Audio capture not started")?;

        // Assuming ~20ms buffer target for WebRTC
        let samples_needed =
            (capture.sample_rate() as usize * capture.channels() as usize * 20) / 1000;
        let samples = capture.read_samples(samples_needed);

        Ok(media::AudioFrame::new(
            samples,
            capture.channels(),
            capture.sample_rate(),
        ))
    }

    /// Starts audio playback
    pub fn start_playback(
        &mut self,
        sample_rate: u32,
        channels: u32,
    ) -> Result<(), Box<dyn Error>> {
        if self.playback.is_some() {
            self.logger.warn("Audio playback already started");
            return Ok(()); // Idempotent
        }

        self.logger.info(&format!(
            "Starting audio playback at {} Hz, {} channel(s)",
            sample_rate, channels
        ));

        let playback = AudioPlayback::new(sample_rate, channels, &self.logger)?;
        self.playback = Some(playback);

        self.logger.info("Audio playback started");
        Ok(())
    }

    /// Stops audio playback
    pub fn stop_playback(&mut self) {
        if self.playback.is_some() {
            self.logger.info("Stopping audio playback");
            self.playback = None;
        }
    }

    /// Plays an audio frame
    pub fn play_frame(&mut self, frame: &media::AudioFrame) -> Result<(), Box<dyn Error>> {
        match self.playback.as_ref() {
            Some(playback) => {
                playback.play_samples(&frame.samples);
                Ok(())
            }
            None => {
                // If playback isn't explicitly started, try to start it with frame settings?
                // Or just error. Ideally it should be started.
                // For now, let's error to enforce explicit lifecycle.
                Err("Audio playback not started".into())
            }
        }
    }

    /// Returns whether playback is active
    pub fn is_playing(&self) -> bool {
        self.playback.is_some()
    }

    pub fn get_settings(&self) -> Option<AudioSettings> {
        self.capture.as_ref().map(|c| AudioSettings {
            sample_rate: c.sample_rate(),
            channels: c.channels(),
            buffer_size: 0,
        })
    }

    /// Clears the audio capture buffer
    pub fn clear_capture_buffer(&self) {
        if let Some(ref capture) = self.capture {
            capture.clear_buffer();
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AudioSettings {
    pub sample_rate: u32,
    pub channels: u32,
    pub buffer_size: u32,
}
