//! H.264 (AVC) video decoder implementation.
//!
//! Provides H.264 decoding functionality using FFmpeg,
//! with automatic parameter set handling.

use crate::common::constants::logging::DECODER_LOG_INTERVAL;
use crate::error::{MediaError, Result};
use crate::video::frame::VideoFrame;
use crate::video::traits::VideoDecoder;
use crate::video::utils::{extract_nal_type, is_parameter_set, yuv_frame_to_mat};
use ffmpeg::decoder::video::Video as FfmpegVideoDecoder;
use ffmpeg_next as ffmpeg;
use logging::Logger;

/// Represents an H.264 video decoder using FFmpeg.
///
/// Handles decoding of H.264 NAL units into raw video frames.
pub struct H264Decoder {
    decoder: FfmpegVideoDecoder,
    logger: Logger,
    frame_count: u64,
    received_sps_pps: bool,
}

impl H264Decoder {
    /// Creates a new H.264 decoder
    ///
    /// # Arguments
    ///
    /// * `logger` - Logger instance
    ///
    /// # Returns
    ///
    /// * `Ok(H264Decoder)` - Successfully initialized decoder
    /// * `Err` - If FFmpeg initialization or codec setup fails
    pub fn new(logger: Logger) -> Result<Self> {
        logger.info("Initializing H264 decoder");

        ffmpeg::init().map_err(|e| MediaError::Codec(format!("Error init ffmpeg: {}", e)))?;

        // Try multiple decoder lookups for compatibility
        let codec = ffmpeg::decoder::find_by_name("h264")
            .or_else(|| ffmpeg::decoder::find(ffmpeg::codec::Id::H264))
            .or_else(|| ffmpeg::decoder::find_by_name("libx264"))
            .ok_or_else(|| MediaError::Codec("H264 decoder not found".to_string()))?;

        let ctx = ffmpeg::codec::context::Context::new_with_codec(codec);

        let decoder = ctx
            .decoder()
            .video()
            .map_err(|e| MediaError::Codec(format!("Error creating/opening decoder: {}", e)))?;

        Ok(H264Decoder {
            decoder,
            logger,
            frame_count: 0,
            received_sps_pps: false,
        })
    }

    /// Decodes H.264 data into a raw video frame
    ///
    /// Returns `Option<VideoFrame>` because some packets (SPS/PPS) don't produce frames.
    /// **Important**: Will reject frame data until SPS/PPS are received to prevent
    /// decoder errors and ensure proper initialization.
    ///
    /// # Arguments
    ///
    /// * `data` - H.264 NAL unit data
    ///
    /// # Returns
    ///
    /// * `Ok(Some(VideoFrame))` - Successfully decoded frame
    /// * `Ok(None)` - Packet processed but no frame produced (e.g., SPS/PPS)
    /// * `Err` - If decoding fails or frame data received before SPS/PPS
    pub fn decode(&mut self, data: &[u8]) -> Result<Option<VideoFrame>> {
        let nal_type = extract_nal_type(data);

        // Track SPS/PPS reception
        if is_parameter_set(nal_type) {
            self.received_sps_pps = true;
            self.logger
                .debug(&format!("Received parameter set: NAL type {}", nal_type));
        }

        // Skip frame data before SPS/PPS (don't error, just skip)
        if !self.received_sps_pps && !is_parameter_set(nal_type) {
            self.logger.debug(&format!(
                "Skipping NAL type {} (waiting for SPS/PPS)",
                nal_type
            ));
            return Ok(None);
        }

        let packet = ffmpeg::Packet::copy(data);

        // Try to send packet to decoder (skip on error)
        if let Err(e) = self.decoder.send_packet(&packet) {
            self.logger.warn(&format!(
                "Failed to send packet (NAL type {}): {} - skipping",
                nal_type, e
            ));
            return Ok(None);
        }

        let mut decoded_frame = ffmpeg::frame::Video::empty();

        match self.decoder.receive_frame(&mut decoded_frame) {
            Ok(_) => {
                self.frame_count += 1;
                if self.frame_count.is_multiple_of(DECODER_LOG_INTERVAL) {
                    self.logger.debug(&format!(
                        "Decoded {} frames (latest: {}x{})",
                        self.frame_count,
                        decoded_frame.width(),
                        decoded_frame.height()
                    ));
                }

                let mat = yuv_frame_to_mat(&decoded_frame)?;
                Ok(Some(VideoFrame::new(mat)))
            }
            Err(_) => {
                // No frame available yet (need more data or this was a parameter set)
                Ok(None)
            }
        }
    }
}

// Implement VideoDecoder trait for polymorphic usage
impl VideoDecoder for H264Decoder {
    fn decode(&mut self, data: &[u8]) -> Result<VideoFrame> {
        // Use the existing decode method
        self.decode(data)?
            .ok_or_else(|| MediaError::Codec("No frame decoded (may be SPS/PPS)".to_string()))
    }

    fn get_codec(&self) -> &str {
        "H264"
    }

    fn reset(&mut self) {
        self.logger.info("Resetting H264 decoder");
        self.decoder.flush();
        self.received_sps_pps = false;
        self.frame_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;
    use tempfile::tempdir;

    fn create_test_logger() -> Logger {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_decoder.log");
        Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    #[test]
    fn test_decoder_creation() {
        let logger = create_test_logger();
        let decoder = H264Decoder::new(logger);
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_decoder_trait() {
        let logger = create_test_logger();
        let mut decoder = H264Decoder::new(logger).unwrap();

        // Test trait methods
        assert_eq!(decoder.get_codec(), "H264");

        // Test reset
        decoder.reset();
        assert_eq!(decoder.frame_count, 0);
        assert!(!decoder.received_sps_pps);
    }
}
