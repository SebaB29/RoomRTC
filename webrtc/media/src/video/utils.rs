//! Video codec utilities
//!
//! Shared helper functions for video encoding and decoding operations.

use super::constants::{h264::*, yuv::*};
use crate::error::{MediaError, Result};
use ffmpeg_next as ffmpeg;
use opencv::core::Mat;
use opencv::prelude::*;

/// Extracts NAL unit type from H.264 data
///
/// Searches for NAL start codes (4-byte or 3-byte) and extracts the NAL type.
///
/// # NAL Types
/// - 1: Non-IDR coded slice
/// - 5: IDR coded slice (keyframe)
/// - 7: SPS (Sequence Parameter Set)
/// - 8: PPS (Picture Parameter Set)
///
/// # Arguments
/// * `data` - H.264 NAL unit data
///
/// # Returns
/// * NAL unit type (0 if no start code found)
pub fn extract_nal_type(data: &[u8]) -> u8 {
    for i in 0..data.len().saturating_sub(START_CODE_4_LEN) {
        // Check for 4-byte start code
        if data[i..i + START_CODE_4_LEN] == NAL_START_CODE_4 && i + START_CODE_4_LEN < data.len() {
            return data[i + START_CODE_4_LEN] & NAL_TYPE_MASK;
        }
        // Check for 3-byte start code
        if data[i..i + START_CODE_3_LEN] == NAL_START_CODE_3 && i + START_CODE_3_LEN < data.len() {
            return data[i + START_CODE_3_LEN] & NAL_TYPE_MASK;
        }
    }
    0
}

/// Checks if NAL unit is a parameter set (SPS or PPS)
///
/// # Arguments
/// * `nal_type` - NAL unit type
///
/// # Returns
/// * `true` if NAL is SPS (7) or PPS (8), `false` otherwise
pub fn is_parameter_set(nal_type: u8) -> bool {
    nal_type == NAL_TYPE_SPS || nal_type == NAL_TYPE_PPS
}

/// Converts OpenCV BGR Mat to FFmpeg YUV420P frame
///
/// Uses OpenCV's color space conversion to transform BGR to YUV420P format,
/// then copies the planes to an FFmpeg video frame.
///
/// # Arguments
/// * `mat` - OpenCV matrix in BGR format
///
/// # Returns
/// * `Ok(ffmpeg::frame::Video)` - YUV420P frame ready for encoding
/// * `Err` - If conversion or data copy fails
pub fn mat_to_yuv_frame(mat: &Mat) -> Result<ffmpeg::frame::Video> {
    use opencv::imgproc::{COLOR_BGR2YUV_I420, cvt_color_def};

    let mut yuv_mat = Mat::default();
    cvt_color_def(mat, &mut yuv_mat, COLOR_BGR2YUV_I420)?;

    let width = mat.cols() as u32;
    let height = mat.rows() as u32;

    let mut frame = ffmpeg::frame::Video::new(ffmpeg::format::Pixel::YUV420P, width, height);

    let data = yuv_mat
        .data_bytes()
        .map_err(|e| MediaError::Codec(format!("Error getting YUV data: {}", e)))?;

    let y_size = (width * height) as usize;
    let uv_size = y_size / UV_PLANE_DIVISOR;

    // Validate data size
    if data.len() < y_size + uv_size * 2 {
        return Err(MediaError::Codec(format!(
            "Insufficient YUV data: got {} bytes, expected {} bytes",
            data.len(),
            y_size + uv_size * 2
        )));
    }

    // Copy Y, U, V planes
    frame.data_mut(Y_PLANE_INDEX)[..y_size].copy_from_slice(&data[..y_size]);
    frame.data_mut(U_PLANE_INDEX)[..uv_size].copy_from_slice(&data[y_size..y_size + uv_size]);
    frame.data_mut(V_PLANE_INDEX)[..uv_size]
        .copy_from_slice(&data[y_size + uv_size..y_size + uv_size * 2]);

    Ok(frame)
}

/// Converts FFmpeg YUV420P frame to OpenCV BGR Mat
///
/// Uses FFmpeg's software scaler to transform YUV420P to BGR24,
/// then reshapes into an OpenCV-compatible matrix.
///
/// # Arguments
/// * `frame` - FFmpeg video frame in YUV420P format
///
/// # Returns
/// * `Ok(Mat)` - OpenCV matrix in BGR format
/// * `Err` - If scaling or reshaping fails
pub fn yuv_frame_to_mat(frame: &ffmpeg::frame::Video) -> Result<Mat> {
    let width = frame.width();
    let height = frame.height();

    // Create scaler for YUV420P to BGR24 conversion
    let mut scaler = ffmpeg::software::scaling::Context::get(
        ffmpeg::format::Pixel::YUV420P,
        width,
        height,
        ffmpeg::format::Pixel::BGR24,
        width,
        height,
        ffmpeg::software::scaling::Flags::BICUBIC,
    )
    .map_err(|e| MediaError::Codec(format!("Error creating scaler: {}", e)))?;

    // Create output frame in BGR24
    let mut output_frame = ffmpeg::frame::Video::new(ffmpeg::format::Pixel::BGR24, width, height);

    // Scale the frame
    scaler
        .run(frame, &mut output_frame)
        .map_err(|e| MediaError::Codec(format!("Error scaling frame: {}", e)))?;

    // Create OpenCV Mat from BGR data
    let bgr_data = output_frame.data(0);
    let mat = Mat::from_slice(bgr_data)
        .map_err(|e| MediaError::Codec(format!("Error creating Mat from BGR data: {}", e)))?;

    // Reshape to height x width x 3 (BGR)
    let mat = mat
        .reshape(3, height as i32)
        .map_err(|e| MediaError::Codec(format!("Error reshaping Mat: {}", e)))?;

    mat.try_clone()
        .map_err(|e| MediaError::Codec(format!("Error cloning Mat: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_nal_type_4_byte() {
        let data = vec![0x00, 0x00, 0x00, 0x01, 0x67, 0x42]; // SPS
        assert_eq!(extract_nal_type(&data), NAL_TYPE_SPS);
    }

    #[test]
    fn test_extract_nal_type_3_byte() {
        let data = vec![0x00, 0x00, 0x01, 0x68, 0x42]; // PPS
        assert_eq!(extract_nal_type(&data), NAL_TYPE_PPS);
    }

    #[test]
    fn test_extract_nal_type_idr() {
        let data = vec![0x00, 0x00, 0x00, 0x01, 0x65]; // IDR
        assert_eq!(extract_nal_type(&data), NAL_TYPE_IDR);
    }

    #[test]
    fn test_extract_nal_type_no_start_code() {
        let data = vec![0x01, 0x02, 0x03, 0x04];
        assert_eq!(extract_nal_type(&data), 0);
    }

    #[test]
    fn test_is_parameter_set() {
        assert!(is_parameter_set(NAL_TYPE_SPS));
        assert!(is_parameter_set(NAL_TYPE_PPS));
        assert!(!is_parameter_set(NAL_TYPE_IDR));
        assert!(!is_parameter_set(NAL_TYPE_NON_IDR));
    }
}
