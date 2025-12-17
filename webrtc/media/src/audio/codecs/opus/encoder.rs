//! Opus audio encoder implementation
//!
//! Provides Opus encoding using FFmpeg's libopus codec,
//! optimized for low-latency voice communication.

use crate::audio::frame::AudioFrame;
use crate::error::{MediaError, Result};
use ffmpeg_next as ffmpeg;
use logging::Logger;

/// Opus audio encoder for WebRTC
///
/// Encodes PCM audio frames to Opus compressed format.
/// Optimized for voice with low latency.
pub struct OpusEncoder {
    encoder: ffmpeg::encoder::Audio,
    logger: Logger,
    frame_count: u64,
    pts: i64,
    sample_rate: u32,
    channels: u32,
}

impl OpusEncoder {
    /// Creates a new Opus encoder
    ///
    /// # Arguments
    ///
    /// * `sample_rate` - Sample rate in Hz (48000 recommended for Opus)
    /// * `channels` - Number of channels (1=mono, 2=stereo)
    /// * `bitrate` - Target bitrate in bits per second (32000-128000 typical)
    /// * `logger` - Logger instance
    ///
    /// # Returns
    ///
    /// * `Ok(OpusEncoder)` - Successfully initialized encoder
    /// * `Err` - If FFmpeg initialization or codec setup fails
    pub fn new(sample_rate: u32, channels: u32, bitrate: u32, logger: Logger) -> Result<Self> {
        logger.info(&format!(
            "Initializing Opus encoder: sample_rate={}, channels={}, bitrate={}",
            sample_rate, channels, bitrate
        ));

        ffmpeg::init().map_err(|e| MediaError::Codec(format!("FFmpeg init error: {}", e)))?;

        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::OPUS)
            .ok_or_else(|| MediaError::Codec("Opus codec not found".to_string()))?
            .audio()
            .map_err(|e| MediaError::Codec(format!("Not an audio codec: {}", e)))?;

        // Create context
        let ctx = ffmpeg::codec::context::Context::new_with_codec(*codec);
        let mut encoder = ctx
            .encoder()
            .audio()
            .map_err(|e| MediaError::Codec(format!("Error creating encoder context: {}", e)))?;

        // Configure encoder parameters
        encoder.set_rate(sample_rate as i32);
        encoder.set_format(ffmpeg::format::Sample::I16(
            ffmpeg::format::sample::Type::Packed,
        ));

        // Set channel layout based on channel count
        let channel_layout = match channels {
            1 => ffmpeg::ChannelLayout::MONO,
            2 => ffmpeg::ChannelLayout::STEREO,
            _ => {
                return Err(MediaError::Codec(format!(
                    "Unsupported channel count: {}",
                    channels
                )));
            }
        };
        encoder.set_channel_layout(channel_layout);

        encoder.set_bit_rate(bitrate as usize);
        encoder.set_time_base((1, sample_rate as i32));

        // Low-latency settings via options dictionary
        // These parameters are crucial for Opus configuration
        let mut opts = ffmpeg::Dictionary::new();
        opts.set("application", "voip"); // Optimize for voice
        opts.set("frame_duration", "20"); // 20ms frames (low latency)
        opts.set("packet_loss", "15"); // Enable packet loss resilience (0-100%)
        opts.set("complexity", "8"); // Max quality (0-10)
        opts.set("vbr", "on"); // Variable bitrate for better quality

        let encoder = encoder
            .open_with(opts)
            .map_err(|e| MediaError::Codec(format!("Error opening encoder: {}", e)))?;

        logger.info(&format!(
            "Opus encoder opened: {}Hz, {} ch, {} bps",
            sample_rate, channels, bitrate
        ));

        logger.info("Opus encoder initialized successfully");

        Ok(OpusEncoder {
            encoder,
            logger,
            frame_count: 0,
            pts: 0,
            sample_rate,
            channels,
        })
    }

    /// Encodes an audio frame to Opus format
    ///
    /// # Arguments
    ///
    /// * `frame` - Audio frame with PCM samples
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<u8>)` - Encoded Opus packet
    /// * `Err` - If encoding fails
    pub fn encode(&mut self, frame: &AudioFrame) -> Result<Vec<u8>> {
        // Validate frame matches encoder configuration
        if frame.channels != self.channels {
            return Err(MediaError::Codec(format!(
                "Channel mismatch: encoder expects {}, got {}",
                self.channels, frame.channels
            )));
        }

        if frame.sample_rate != self.sample_rate {
            self.logger.warn(&format!(
                "Sample rate mismatch: encoder expects {}, got {} - audio quality may degrade",
                self.sample_rate, frame.sample_rate
            ));
        }

        // Create FFmpeg audio frame with proper parameters
        let frame_count = frame.samples.len() / frame.channels as usize;
        let mut audio_frame = ffmpeg::frame::Audio::new(
            ffmpeg::format::Sample::I16(ffmpeg::format::sample::Type::Packed),
            frame_count,
            ffmpeg::ChannelLayout::default(frame.channels as i32),
        );

        audio_frame.set_pts(Some(self.pts));

        // Copy samples to FFmpeg frame (ensure proper alignment)
        // SAFETY: FFmpeg frame data buffer is guaranteed to be large enough
        // for the specified frame_count and sample format (I16 packed)
        Self::copy_samples_to_frame(&mut audio_frame, &frame.samples)?;

        // Update PTS for next frame
        self.pts += frame_count as i64;

        // Send frame to encoder
        self.encoder
            .send_frame(&audio_frame)
            .map_err(|e| MediaError::Codec(format!("Error sending audio frame: {}", e)))?;

        // Try to receive encoded packet (may need multiple frames)
        let mut encoded_packet = ffmpeg::Packet::empty();

        match self.encoder.receive_packet(&mut encoded_packet) {
            Ok(_) => {
                self.frame_count += 1;

                if self.frame_count.is_multiple_of(100) {
                    self.logger.debug(&format!(
                        "Encoded audio frame #{}, size: {} bytes, pts: {}",
                        self.frame_count,
                        encoded_packet.size(),
                        self.pts
                    ));
                }

                Ok(encoded_packet.data().unwrap_or(&[]).to_vec())
            }
            Err(_) => {
                // EAGAIN means encoder needs more input
                // This is normal for Opus which buffers internally
                if self.frame_count == 0 {
                    self.logger.debug("Encoder buffering initial frames");
                }
                Ok(Vec::new())
            }
        }
    }

    /// Flush any remaining encoded data
    pub fn flush(&mut self) -> Result<Vec<Vec<u8>>> {
        self.encoder
            .send_eof()
            .map_err(|e| MediaError::Codec(format!("Error flushing encoder: {}", e)))?;

        let mut packets = Vec::new();
        let mut packet = ffmpeg::Packet::empty();

        while self.encoder.receive_packet(&mut packet).is_ok() {
            if let Some(data) = packet.data() {
                packets.push(data.to_vec());
            }
        }

        Ok(packets)
    }

    /// Copies i16 PCM samples to FFmpeg audio frame
    fn copy_samples_to_frame(
        audio_frame: &mut ffmpeg::frame::Audio,
        samples: &[i16],
    ) -> Result<()> {
        let data = audio_frame.data_mut(0);
        let required_bytes = std::mem::size_of_val(samples);

        if required_bytes > data.len() {
            return Err(MediaError::Codec(format!(
                "Sample buffer too large: {} bytes, frame capacity: {} bytes",
                required_bytes,
                data.len()
            )));
        }

        // Copy samples using safe iteration
        for (i, &sample) in samples.iter().enumerate() {
            let bytes = sample.to_ne_bytes();
            let offset = i * 2;
            data[offset] = bytes[0];
            data[offset + 1] = bytes[1];
        }
        Ok(())
    }
}

impl Drop for OpusEncoder {
    fn drop(&mut self) {
        let _ = self.flush();
        self.logger.info(&format!(
            "Opus encoder stopped. Total frames encoded: {}",
            self.frame_count
        ));
    }
}
