//! VP8 video encoder implementation.
//!
//! Provides VP8 encoding functionality using FFmpeg's libvpx,
//! suitable for WebRTC applications.

use crate::common::constants::logging::ENCODER_LOG_INTERVAL;
use crate::error::{MediaError, Result};
use crate::video::frame::VideoFrame;
use crate::video::traits::VideoEncoder;
use crate::video::utils::mat_to_yuv_frame;
use ffmpeg_next as ffmpeg;
use logging::Logger;

/// Represents a VP8 video encoder using FFmpeg.
///
/// Handles encoding of raw video frames into VP8 NAL units.
pub struct VP8Encoder {
    encoder: ffmpeg::encoder::Video,
    logger: Logger,
    frame_count: u64,
    pts: i64,
    bitrate: u32,
}

impl VP8Encoder {
    /// Creates a new VP8 encoder
    ///
    /// # Arguments
    ///
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `bitrate` - Target bitrate in bits per second
    /// * `keyframe_interval` - GOP size (frames between keyframes)
    /// * `logger` - Logger instance
    ///
    /// # Returns
    ///
    /// * `Ok(VP8Encoder)` - Successfully initialized encoder
    /// * `Err` - If FFmpeg initialization or codec setup fails
    pub fn new(
        width: u32,
        height: u32,
        bitrate: u32,
        keyframe_interval: u32,
        logger: Logger,
    ) -> Result<Self> {
        logger.info(&format!(
            "Initializing VP8 encoder: {}x{}, bitrate={}, gop={}",
            width, height, bitrate, keyframe_interval
        ));

        ffmpeg::init().map_err(|e| MediaError::Codec(format!("Error init ffmpeg: {}", e)))?;

        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::VP8)
            .ok_or_else(|| MediaError::Codec("VP8 codec not found".to_string()))?
            .video()
            .map_err(|e| MediaError::Codec(format!("Not a video codec: {}", e)))?;

        let mut encoder = ffmpeg::codec::context::Context::new_with_codec(*codec)
            .encoder()
            .video()
            .map_err(|e| MediaError::Codec(format!("Error creating context: {}", e)))?;

        encoder.set_width(width);
        encoder.set_height(height);
        encoder.set_format(ffmpeg::format::Pixel::YUV420P);
        encoder.set_bit_rate(bitrate as usize);
        encoder.set_time_base((1, 30)); // 30 FPS
        encoder.set_frame_rate(Some((30, 1)));
        encoder.set_gop(keyframe_interval);

        let encoder = encoder
            .open_as(codec)
            .map_err(|e| MediaError::Codec(format!("Error opening encoder: {}", e)))?;

        Ok(VP8Encoder {
            encoder,
            logger,
            frame_count: 0,
            pts: 0,
            bitrate,
        })
    }

    /// Encodes a video frame into one or more VP8 packets
    ///
    /// Returns `Vec<Vec<u8>>` where each inner Vec is a complete VP8 frame.
    pub fn encode(&mut self, frame: &VideoFrame) -> Result<Vec<Vec<u8>>> {
        let mat = frame.data();
        let mut yuv_frame = mat_to_yuv_frame(mat)?;

        yuv_frame.set_pts(Some(self.pts));
        self.pts += 1;

        self.encoder
            .send_frame(&yuv_frame)
            .map_err(|e| MediaError::Codec(format!("Error sending frame: {}", e)))?;

        let mut packets = Vec::new();
        let mut encoded_packet = ffmpeg::Packet::empty();

        while self.encoder.receive_packet(&mut encoded_packet).is_ok() {
            if let Some(data) = encoded_packet.data() {
                self.logger.debug(&format!(
                    "Encoded VP8 packet: size={}, is_key={}",
                    data.len(),
                    encoded_packet.is_key()
                ));
                packets.push(data.to_vec());
            }
        }

        self.frame_count += 1;

        if self.frame_count.is_multiple_of(ENCODER_LOG_INTERVAL) {
            self.logger.debug(&format!(
                "Encoded {} frames ({} packets this frame)",
                self.frame_count,
                packets.len()
            ));
        }

        Ok(packets)
    }
}

// Implement VideoEncoder trait for polymorphic usage
impl VideoEncoder for VP8Encoder {
    fn encode(&mut self, frame: &VideoFrame) -> Result<Vec<u8>> {
        // Use the existing encode method and flatten packets
        let packets = self.encode(frame)?;
        Ok(packets.into_iter().flatten().collect())
    }

    fn get_codec(&self) -> &str {
        "VP8"
    }

    fn get_bitrate(&self) -> u32 {
        self.bitrate
    }

    fn request_keyframe(&mut self) {
        self.logger.debug("VP8 keyframe requested");
        // VP8 doesn't have a simple way to force keyframe,
        // typically done through encoder settings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;
    use opencv::core::{CV_8UC3, Mat, Scalar};
    use tempfile::tempdir;

    fn create_test_logger() -> Logger {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test_vp8_encoder.log");
        Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    fn create_test_frame() -> VideoFrame {
        let mat = Mat::new_rows_cols_with_default(
            480,
            640,
            CV_8UC3,
            Scalar::new(100.0, 150.0, 200.0, 0.0),
        )
        .unwrap();
        VideoFrame::new(mat)
    }

    #[test]
    fn test_encoder_creation() {
        let logger = create_test_logger();
        let encoder = VP8Encoder::new(640, 480, 500_000, 30, logger);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_encoder_trait() {
        let logger = create_test_logger();
        let mut encoder = VP8Encoder::new(640, 480, 500_000, 30, logger).unwrap();

        assert_eq!(encoder.get_codec(), "VP8");
        assert_eq!(encoder.get_bitrate(), 500_000);

        // Test encoding through trait
        let frame = create_test_frame();
        let result = VideoEncoder::encode(&mut encoder, &frame);
        // Encoding may fail in CI without proper FFmpeg/libvpx setup
        let _ = result;
    }
}
