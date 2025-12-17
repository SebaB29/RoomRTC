//! Opus audio decoder implementation
//!
//! Provides Opus decoding using FFmpeg's libopus codec.

use crate::audio::frame::AudioFrame;
use crate::error::{MediaError, Result};
use ffmpeg::decoder::audio::Audio as FfmpegAudioDecoder;
use ffmpeg_next as ffmpeg;
use logging::Logger;

/// Opus audio decoder for WebRTC
///
/// Decodes Opus compressed audio to PCM format for playback.
pub struct OpusDecoder {
    decoder: FfmpegAudioDecoder,
    logger: Logger,
    frame_count: u64,
    sample_rate: u32,
    channels: u32,
}

impl OpusDecoder {
    /// Creates a new Opus decoder
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Expected sample rate in Hz
    /// * `channels` - Expected number of channels
    /// * `logger` - Logger instance
    ///
    /// # Returns
    ///
    /// * `Ok(OpusDecoder)` - Successfully initialized decoder
    /// * `Err` - If FFmpeg initialization fails
    pub fn new(sample_rate: u32, channels: u32, logger: Logger) -> Result<Self> {
        logger.info(&format!(
            "Initializing Opus decoder: sample_rate={}, channels={}",
            sample_rate, channels
        ));

        ffmpeg::init().map_err(|e| MediaError::Codec(format!("FFmpeg init error: {}", e)))?;

        let codec = ffmpeg::decoder::find(ffmpeg::codec::Id::OPUS)
            .ok_or_else(|| MediaError::Codec("Opus codec not found".to_string()))?;

        let ctx = ffmpeg::codec::context::Context::new_with_codec(codec);

        let decoder = ctx
            .decoder()
            .audio()
            .map_err(|e| MediaError::Codec(format!("Error creating/opening decoder: {}", e)))?;

        logger.info(&format!(
            "Opus decoder initialized successfully - Format: {:?}, Rate: {}, Channels: {}",
            decoder.format(),
            decoder.rate(),
            decoder.channels()
        ));

        Ok(OpusDecoder {
            decoder,
            logger,
            frame_count: 0,
            sample_rate,
            channels,
        })
    }

    /// Decodes an Opus packet to PCM audio
    ///
    /// # Arguments
    ///
    /// * `data` - Encoded Opus packet data
    ///
    /// # Returns
    ///
    /// * `Ok(Option<AudioFrame>)` - Decoded audio frame, or None if more data needed
    /// * `Err` - If decoding fails
    pub fn decode(&mut self, data: &[u8]) -> Result<Option<AudioFrame>> {
        if data.is_empty() {
            return Ok(None);
        }

        // Create packet from data
        let packet = ffmpeg::Packet::copy(data);

        // Send packet to decoder
        self.decoder
            .send_packet(&packet)
            .map_err(|e| MediaError::Codec(format!("Error sending packet to decoder: {}", e)))?;

        // Receive decoded frame
        let mut decoded_frame = ffmpeg::frame::Audio::empty();

        match self.decoder.receive_frame(&mut decoded_frame) {
            Ok(_) => {
                self.frame_count += 1;

                if self.frame_count.is_multiple_of(100) {
                    self.logger.debug(&format!(
                        "Decoded audio frame #{}, samples: {}",
                        self.frame_count,
                        decoded_frame.samples()
                    ));
                }

                // Extract PCM samples
                let samples = self.extract_samples(&decoded_frame)?;

                Ok(Some(AudioFrame::new(
                    samples,
                    self.channels,
                    self.sample_rate,
                )))
            }
            Err(_e) => {
                // Need more data or other non-fatal error
                // Return None to indicate no frame available yet
                Ok(None)
            }
        }
    }

    /// Extracts PCM samples from FFmpeg audio frame
    ///
    /// Converts various FFmpeg audio formats to i16 PCM samples.
    /// Handles both planar (separate channels) and packed (interleaved) formats.
    fn extract_samples(&self, frame: &ffmpeg::frame::Audio) -> Result<Vec<i16>> {
        let sample_count = frame.samples();
        let channels = frame.channels() as usize;
        let total_samples = sample_count * channels;
        let mut samples = Vec::with_capacity(total_samples);

        match frame.format() {
            ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Planar) => {
                Self::extract_f32_planar(frame, sample_count, channels, &mut samples);
            }
            ffmpeg::format::Sample::F32(ffmpeg::format::sample::Type::Packed) => {
                Self::extract_f32_packed(frame, total_samples, &mut samples);
            }
            ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed) => {
                Self::extract_i16_packed(frame, total_samples, &mut samples);
            }
            ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Planar) => {
                Self::extract_i16_planar(frame, sample_count, channels, &mut samples);
            }
            other => {
                return Err(MediaError::Codec(format!(
                    "Unsupported audio format from decoder: {:?}",
                    other
                )));
            }
        }

        Ok(samples)
    }

    /// Converts f32 to i16 with proper clamping and rounding
    #[inline]
    fn convert_f32_to_i16(sample_f32: f32) -> i16 {
        let clamped = sample_f32.clamp(-1.0, 1.0);
        let scaled = clamped * 32767.0;
        scaled.round() as i16
    }

    /// Extracts F32 planar format (each channel in separate plane)
    fn extract_f32_planar(
        frame: &ffmpeg::frame::Audio,
        sample_count: usize,
        channels: usize,
        samples: &mut Vec<i16>,
    ) {
        for i in 0..sample_count {
            for ch in 0..channels {
                let plane_data = frame.data(ch);
                let f32_slice = unsafe {
                    std::slice::from_raw_parts(plane_data.as_ptr() as *const f32, sample_count)
                };
                if let Some(&sample_f32) = f32_slice.get(i) {
                    samples.push(Self::convert_f32_to_i16(sample_f32));
                } else {
                    samples.push(0);
                }
            }
        }
    }

    /// Extracts F32 packed format (all channels interleaved)
    fn extract_f32_packed(
        frame: &ffmpeg::frame::Audio,
        total_samples: usize,
        samples: &mut Vec<i16>,
    ) {
        let data = frame.data(0);
        let f32_slice = unsafe {
            std::slice::from_raw_parts(
                data.as_ptr() as *const f32,
                total_samples.min(data.len() / std::mem::size_of::<f32>()),
            )
        };
        for &sample_f32 in f32_slice.iter().take(total_samples) {
            samples.push(Self::convert_f32_to_i16(sample_f32));
        }
    }

    /// Extracts I16 packed format (direct copy)
    fn extract_i16_packed(
        frame: &ffmpeg::frame::Audio,
        total_samples: usize,
        samples: &mut Vec<i16>,
    ) {
        let data = frame.data(0);
        let max_samples = data.len() / std::mem::size_of::<i16>();
        let safe_count = total_samples.min(max_samples);

        let i16_slice =
            unsafe { std::slice::from_raw_parts(data.as_ptr() as *const i16, safe_count) };
        samples.extend_from_slice(i16_slice);
    }

    /// Extracts I16 planar format (interleave channels)
    fn extract_i16_planar(
        frame: &ffmpeg::frame::Audio,
        sample_count: usize,
        channels: usize,
        samples: &mut Vec<i16>,
    ) {
        for i in 0..sample_count {
            for ch in 0..channels {
                let plane_data = frame.data(ch);
                let i16_slice = unsafe {
                    std::slice::from_raw_parts(plane_data.as_ptr() as *const i16, sample_count)
                };
                if let Some(&sample) = i16_slice.get(i) {
                    samples.push(sample);
                } else {
                    samples.push(0);
                }
            }
        }
    }
}

impl Drop for OpusDecoder {
    fn drop(&mut self) {
        self.logger.info(&format!(
            "Opus decoder stopped. Total frames decoded: {}",
            self.frame_count
        ));
    }
}
