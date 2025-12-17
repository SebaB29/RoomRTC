//! VP8 video decoder implementation.
//!
//! Provides VP8 decoding functionality using FFmpeg's libvpx,
//! suitable for WebRTC applications.

use crate::common::constants::logging::DECODER_LOG_INTERVAL;
use crate::error::{MediaError, Result};
use crate::video::frame::VideoFrame;
use crate::video::traits::VideoDecoder;
use crate::video::utils::yuv_frame_to_mat;
use ffmpeg::decoder::video::Video as FfmpegVideoDecoder;
use ffmpeg_next as ffmpeg;
use logging::Logger;

/// Represents a VP8 video decoder using FFmpeg.
///
/// Handles decoding of VP8 NAL units into raw video frames.
pub struct VP8Decoder {
    decoder: FfmpegVideoDecoder,
    logger: Logger,
    frame_count: u64,
}

impl VP8Decoder {
    /// Creates a new VP8 decoder
    ///
    /// # Arguments
    ///
    /// * `logger` - Logger instance
    ///
    /// # Returns
    ///
    /// * `Ok(VP8Decoder)` - Successfully initialized decoder
    /// * `Err` - If FFmpeg initialization or codec setup fails
    pub fn new(logger: Logger) -> Result<Self> {
        logger.info("Initializing VP8 decoder");

        ffmpeg::init().map_err(|e| MediaError::Codec(format!("Error init ffmpeg: {}", e)))?;

        // Try multiple decoder lookups for compatibility
        let codec = ffmpeg::decoder::find_by_name("vp8")
            .or_else(|| ffmpeg::decoder::find(ffmpeg::codec::Id::VP8))
            .or_else(|| ffmpeg::decoder::find_by_name("libvpx"))
            .ok_or_else(|| MediaError::Codec("VP8 decoder not found".to_string()))?;

        let ctx = ffmpeg::codec::context::Context::new_with_codec(codec);

        let decoder = ctx
            .decoder()
            .video()
            .map_err(|e| MediaError::Codec(format!("Error creating/opening decoder: {}", e)))?;

        Ok(VP8Decoder {
            decoder,
            logger,
            frame_count: 0,
        })
    }

    /// Decodes VP8 data into a raw video frame
    ///
    /// Returns `Option<VideoFrame>` because some packets may not produce frames immediately.
    ///
    /// # Arguments
    ///
    /// * `data` - VP8 encoded data
    ///
    /// # Returns
    ///
    /// * `Ok(Some(VideoFrame))` - Successfully decoded frame
    /// * `Ok(None)` - Packet processed but no frame produced yet
    /// * `Err` - If decoding fails
    pub fn decode(&mut self, data: &[u8]) -> Result<Option<VideoFrame>> {
        let packet = ffmpeg::Packet::copy(data);

        // Try to send packet to decoder
        if let Err(e) = self.decoder.send_packet(&packet) {
            self.logger.warn(&format!("Decoder error, flushing: {}", e));
            self.decoder.flush();

            // Retry after flush
            self.decoder
                .send_packet(&packet)
                .map_err(|e| MediaError::Codec(format!("Error sending packet: {}", e)))?;
        }

        let mut decoded_frame = ffmpeg::frame::Video::empty();

        match self.decoder.receive_frame(&mut decoded_frame) {
            Ok(_) => {
                self.frame_count += 1;
                if self.frame_count.is_multiple_of(DECODER_LOG_INTERVAL) {
                    self.logger.debug(&format!(
                        "Decoded {} VP8 frames (latest: {}x{})",
                        self.frame_count,
                        decoded_frame.width(),
                        decoded_frame.height()
                    ));
                }

                let mat = yuv_frame_to_mat(&decoded_frame)?;
                Ok(Some(VideoFrame::new(mat)))
            }
            Err(_) => {
                // No frame available yet (need more data)
                Ok(None)
            }
        }
    }
}

// Implement VideoDecoder trait for polymorphic usage
impl VideoDecoder for VP8Decoder {
    fn decode(&mut self, data: &[u8]) -> Result<VideoFrame> {
        // Use the existing decode method
        self.decode(data)?
            .ok_or_else(|| MediaError::Codec("No VP8 frame decoded yet".to_string()))
    }

    fn get_codec(&self) -> &str {
        "VP8"
    }

    fn reset(&mut self) {
        self.logger.info("Resetting VP8 decoder");
        self.decoder.flush();
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
        let log_path = dir.path().join("test_vp8_decoder.log");
        Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    #[test]
    fn test_decoder_creation() {
        let logger = create_test_logger();
        let decoder = VP8Decoder::new(logger);
        assert!(decoder.is_ok());
    }

    #[test]
    fn test_decoder_trait() {
        let logger = create_test_logger();
        let mut decoder = VP8Decoder::new(logger).unwrap();

        assert_eq!(decoder.get_codec(), "VP8");

        // Test reset
        decoder.reset();
        assert_eq!(decoder.frame_count, 0);
    }
}
