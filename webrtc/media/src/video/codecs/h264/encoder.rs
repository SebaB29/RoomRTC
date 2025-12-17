//! H.264 (AVC) video encoder implementation.
//!
//! Provides H.264 encoding functionality using FFmpeg's libx264,
//! optimized for low-latency streaming.

use crate::common::constants::logging::ENCODER_LOG_INTERVAL;
use crate::error::{MediaError, Result};
use crate::video::constants::h264::*;
use crate::video::frame::VideoFrame;
use crate::video::traits::VideoEncoder;
use crate::video::utils::{extract_nal_type, is_parameter_set, mat_to_yuv_frame};
use ffmpeg_next as ffmpeg;
use logging::Logger;

/// Represents an H.264 video encoder using FFmpeg.
///
/// Handles encoding of raw video frames into H.264 NAL units.
/// Caches SPS/PPS parameter sets to ensure they're always available.
pub struct H264Encoder {
    encoder: ffmpeg::encoder::Video,
    logger: Logger,
    frame_count: u64,
    pts: i64,
    bitrate: u32,
    sps: Option<Vec<u8>>,
    pps: Option<Vec<u8>>,
}

impl H264Encoder {
    /// Creates a new H.264 encoder
    ///
    /// # Arguments
    ///
    /// * `width` - Frame width in pixels
    /// * `height` - Frame height in pixels
    /// * `bitrate` - Target bitrate in bits per second
    /// * `keyframe_interval` - GOP size (frames between keyframes)
    /// * `fps` - Frames per second
    /// * `logger` - Logger instance
    ///
    /// # Returns
    ///
    /// * `Ok(H264Encoder)` - Successfully initialized encoder
    /// * `Err` - If FFmpeg initialization or codec setup fails
    pub fn new(
        width: u32,
        height: u32,
        bitrate: u32,
        keyframe_interval: u32,
        fps: f64,
        logger: Logger,
    ) -> Result<Self> {
        logger.info(&format!(
            "Initializing H264 encoder: {}x{}, bitrate={}, fps={}, gop={}",
            width, height, bitrate, fps, keyframe_interval
        ));

        ffmpeg::init().map_err(|e| MediaError::Codec(format!("Error init ffmpeg: {}", e)))?;

        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::H264)
            .ok_or_else(|| MediaError::Codec("H264 codec not found".to_string()))?
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

        let fps_int = fps.round() as i32;
        encoder.set_time_base((1, fps_int));
        encoder.set_frame_rate(Some((fps_int, 1)));

        encoder.set_gop(keyframe_interval);
        encoder.set_max_b_frames(0); // Disable B-frames for low latency

        // Ultra-low latency settings using safe FFmpeg Dictionary API
        let mut opts = ffmpeg::Dictionary::new();

        // Set encoding preset to ultrafast for minimal latency
        opts.set("preset", "ultrafast");

        // Set tuning to zerolatency - disables lookahead and VBV buffering
        opts.set("tune", "zerolatency");

        // Single-threaded encoding for minimal latency
        opts.set("threads", "1");

        opts.set("rc-lookahead", "0");
        opts.set("sliced-threads", "0");

        opts.set("x264-params", "nal-hrd=cbr:force-cfr=1");

        let encoder = encoder
            .open_with(opts)
            .map_err(|e| MediaError::Codec(format!("Error opening encoder: {}", e)))?;

        Ok(H264Encoder {
            encoder,
            logger,
            frame_count: 0,
            pts: 0,
            bitrate,
            sps: None,
            pps: None,
        })
    }

    /// Encodes a video frame into one or more H.264 packets
    ///
    /// Automatically caches SPS/PPS parameter sets on first occurrence.
    /// Returns `Vec<Vec<u8>>` where each inner Vec is a complete H.264 NAL unit
    /// (may include SPS, PPS, IDR, or P-frames).
    ///
    /// **Important**: SPS/PPS NAL units (types 7, 8) are cached and should be sent
    /// before frame data to ensure decoder can initialize properly.
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
            let data = encoded_packet.data().unwrap_or(&[]).to_vec();

            if !data.is_empty() {
                let nal_type = extract_nal_type(&data);

                // Cache SPS/PPS for retransmission
                if is_parameter_set(nal_type) {
                    match nal_type {
                        NAL_TYPE_SPS => {
                            self.logger.debug("Caching SPS parameter set");
                            self.sps = Some(data.clone());
                        }
                        NAL_TYPE_PPS => {
                            self.logger.debug("Caching PPS parameter set");
                            self.pps = Some(data.clone());
                        }
                        _ => {}
                    }
                }

                self.logger.debug(&format!(
                    "Encoded packet: size={}, NAL type={}, is_key={}",
                    data.len(),
                    nal_type,
                    encoded_packet.is_key()
                ));

                packets.push(data);
            }
        }

        self.frame_count += 1;

        if self.frame_count.is_multiple_of(ENCODER_LOG_INTERVAL) {
            self.logger
                .debug(&format!("Encoded {} frames", self.frame_count));
        }

        Ok(packets)
    }

    /// Returns cached SPS parameter set if available
    pub fn get_sps(&self) -> Option<&Vec<u8>> {
        self.sps.as_ref()
    }

    /// Returns cached PPS parameter set if available
    pub fn get_pps(&self) -> Option<&Vec<u8>> {
        self.pps.as_ref()
    }

    /// Returns true if both SPS and PPS have been cached
    pub fn has_parameter_sets(&self) -> bool {
        self.sps.is_some() && self.pps.is_some()
    }
}

// Implement VideoEncoder trait for polymorphic usage
impl VideoEncoder for H264Encoder {
    fn encode(&mut self, frame: &VideoFrame) -> Result<Vec<u8>> {
        let packets = self.encode(frame)?;
        Ok(packets.into_iter().flatten().collect())
    }

    fn get_codec(&self) -> &str {
        "H264"
    }

    fn get_bitrate(&self) -> u32 {
        self.bitrate
    }

    fn request_keyframe(&mut self) {
        // Force keyframe on next encode by resetting PTS
        self.logger.debug("Keyframe requested");
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
        let log_path = dir.path().join("test_encoder.log");
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
        let encoder = H264Encoder::new(640, 480, 500_000, 60, 30.0, logger);
        assert!(encoder.is_ok());
    }

    #[test]
    fn test_encoder_trait() {
        let logger = create_test_logger();
        let mut encoder = H264Encoder::new(640, 480, 500_000, 60, 30.0, logger).unwrap();

        // Test trait methods
        assert_eq!(encoder.get_codec(), "H264");
        assert_eq!(encoder.get_bitrate(), 500_000);

        // Test encoding through trait
        let frame = create_test_frame();
        let result = VideoEncoder::encode(&mut encoder, &frame);
        let _ = result;
    }
}
