//! Audio management functionality for WebRTC connection

use crate::audio_info::AudioInfo;
use crate::audio_manager::{AudioManager, AudioSettings};
use logging::Logger;
use std::error::Error;

/// Handles all audio-related operations
pub(super) struct AudioHandler {
    audio_manager: AudioManager,
    logger: Logger,
    is_muted: bool,
}

impl AudioHandler {
    pub fn new(logger: Logger) -> Self {
        Self {
            audio_manager: AudioManager::new(logger.clone()),
            logger,
            is_muted: false,
        }
    }

    pub fn discover_devices(&mut self) -> Result<Vec<AudioInfo>, Box<dyn Error>> {
        self.audio_manager.discover_devices()
    }

    pub fn is_audio_running(&self) -> bool {
        self.audio_manager.is_capturing()
    }

    pub fn is_muted(&self) -> bool {
        self.is_muted
    }

    pub fn start_audio(
        &mut self,
        device_id: Option<i32>,
        sample_rate: u32,
        channels: u32,
    ) -> Result<(AudioSettings, bool), Box<dyn Error>> {
        self.logger.info(&format!(
            "START_AUDIO called: device={:?}, sample_rate={}, channels={}",
            device_id, sample_rate, channels
        ));

        let settings = self
            .audio_manager
            .start_capture(device_id, sample_rate, channels)?;
        self.logger.info("Audio capture started");
        Ok((settings, true))
    }

    pub fn start_audio_auto(
        &mut self,
        sample_rate: u32,
        channels: u32,
    ) -> Result<AudioSettings, Box<dyn Error>> {
        let settings = self
            .audio_manager
            .start_capture_auto(sample_rate, channels)?;
        self.logger.info("Audio capture started automatically");
        Ok(settings)
    }

    pub fn stop_audio(&mut self) {
        self.audio_manager.stop_capture();
        self.logger.info("Audio capture stopped");
    }

    pub fn mute(&mut self) {
        if !self.is_muted {
            self.is_muted = true;
            self.logger.info("Audio muted");
        }
    }

    pub fn unmute(&mut self) {
        if self.is_muted {
            self.is_muted = false;
            self.audio_manager.clear_capture_buffer();
            self.logger.info("Audio unmuted");
        }
    }

    pub fn toggle_mute(&mut self) -> bool {
        if self.is_muted {
            self.unmute();
        } else {
            self.mute();
        }
        self.is_muted
    }

    pub fn capture_frame(&mut self) -> Result<media::AudioFrame, Box<dyn Error>> {
        if self.is_muted {
            // Return silent frame when muted
            // We need settings to know size/rate.
            // If capture is running, we can query it?
            if let Some(settings) = self.audio_manager.get_settings() {
                // buffer_size is 0 in new implementation currently, need to calculate manually based on 20ms
                let sample_count =
                    (settings.sample_rate as usize * settings.channels as usize * 20) / 1000;
                let samples = vec![0i16; sample_count];
                Ok(media::AudioFrame::new(
                    samples,
                    settings.channels,
                    settings.sample_rate,
                ))
            } else {
                Err("Audio capture not started, cannot generate silent frame".into())
            }
        } else {
            self.audio_manager.capture_frame()
        }
    }

    /// Starts audio playback on the output device
    pub fn start_playback(
        &mut self,
        sample_rate: u32,
        channels: u32,
    ) -> Result<(), Box<dyn Error>> {
        self.audio_manager.start_playback(sample_rate, channels)
    }



    /// Plays an audio frame on the output device
    pub fn play_frame(&mut self, frame: &media::AudioFrame) -> Result<(), Box<dyn Error>> {
        self.audio_manager.play_frame(frame)
    }

}
