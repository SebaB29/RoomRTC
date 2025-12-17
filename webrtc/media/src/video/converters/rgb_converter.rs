//! BGR to RGB conversion
//!
//! High-performance conversion from OpenCV's BGR format to RGB

use crate::video::frame::VideoFrame;
use opencv::prelude::{MatTraitConst, MatTraitConstManual};
use std::error::Error;

/// Converts a BGR VideoFrame to RGB pixel data
///
/// # Arguments
/// * `frame` - VideoFrame with BGR data (from OpenCV)
///
/// # Returns
/// * `Ok((width, height, rgb_pixels))` - Frame dimensions and RGB pixel data
/// * `Err` - If frame data cannot be accessed
pub fn frame_to_rgb(frame: &VideoFrame) -> Result<(usize, usize, Vec<u8>), Box<dyn Error>> {
    let mat = frame.data();
    let width = mat.cols() as usize;
    let height = mat.rows() as usize;

    let bgr_data = mat
        .data_bytes()
        .map_err(|e| format!("Failed to get frame data: {}", e))?;

    let rgb_pixels = convert_bgr_to_rgb(bgr_data);

    Ok((width, height, rgb_pixels))
}

/// Converts BGR pixel data to RGB format
///
/// Processes in chunks of 4 pixels (12 bytes) for better CPU cache utilization.
///
/// # Arguments
/// * `bgr_data` - BGR pixel data from OpenCV
///
/// # Returns
/// * RGB pixel data
fn convert_bgr_to_rgb(bgr_data: &[u8]) -> Vec<u8> {
    const CHUNK_SIZE: usize = 12; // 4 pixels
    const PIXEL_SIZE: usize = 3;

    let total_bytes = bgr_data.len();
    let mut rgb_pixels = vec![0u8; total_bytes];

    let full_chunks = total_bytes / CHUNK_SIZE;
    let remainder_pixels = (total_bytes % CHUNK_SIZE) / PIXEL_SIZE;

    // Process 4 pixels at a time
    process_pixel_chunks(bgr_data, &mut rgb_pixels, full_chunks);

    // Handle remaining pixels (< 4 pixels)
    let remainder_base = full_chunks * CHUNK_SIZE;
    process_remaining_pixels(bgr_data, &mut rgb_pixels, remainder_base, remainder_pixels);

    rgb_pixels
}

/// Processes full 4-pixel chunks for efficient conversion
fn process_pixel_chunks(bgr_data: &[u8], rgb_pixels: &mut [u8], chunks: usize) {
    for i in 0..chunks {
        let base = i * 12;
        // Pixel 1
        rgb_pixels[base] = bgr_data[base + 2];
        rgb_pixels[base + 1] = bgr_data[base + 1];
        rgb_pixels[base + 2] = bgr_data[base];
        // Pixel 2
        rgb_pixels[base + 3] = bgr_data[base + 5];
        rgb_pixels[base + 4] = bgr_data[base + 4];
        rgb_pixels[base + 5] = bgr_data[base + 3];
        // Pixel 3
        rgb_pixels[base + 6] = bgr_data[base + 8];
        rgb_pixels[base + 7] = bgr_data[base + 7];
        rgb_pixels[base + 8] = bgr_data[base + 6];
        // Pixel 4
        rgb_pixels[base + 9] = bgr_data[base + 11];
        rgb_pixels[base + 10] = bgr_data[base + 10];
        rgb_pixels[base + 11] = bgr_data[base + 9];
    }
}

/// Processes remaining pixels that don't fit in a full chunk
fn process_remaining_pixels(
    bgr_data: &[u8],
    rgb_pixels: &mut [u8],
    base_offset: usize,
    pixel_count: usize,
) {
    for i in 0..pixel_count {
        let base = base_offset + i * 3;
        rgb_pixels[base] = bgr_data[base + 2];
        rgb_pixels[base + 1] = bgr_data[base + 1];
        rgb_pixels[base + 2] = bgr_data[base];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::core::{CV_8UC3, Mat, Scalar};

    #[test]
    fn test_bgr_to_rgb_conversion() {
        // BGR input: Blue=255, Green=128, Red=0
        let bgr = [255, 128, 0];
        let rgb: Vec<u8> = bgr
            .chunks_exact(3)
            .flat_map(|bgr| [bgr[2], bgr[1], bgr[0]])
            .collect();

        // RGB output: Red=0, Green=128, Blue=255
        assert_eq!(rgb, vec![0, 128, 255]);
    }

    #[test]
    fn test_frame_to_rgb_single_pixel() {
        // Create a 1x1 BGR frame with specific color
        let mat =
            Mat::new_rows_cols_with_default(1, 1, CV_8UC3, Scalar::new(100.0, 150.0, 200.0, 0.0))
                .unwrap();
        let frame = VideoFrame::new(mat);

        let (width, height, rgb_pixels) = frame_to_rgb(&frame).unwrap();

        assert_eq!(width, 1);
        assert_eq!(height, 1);
        assert_eq!(rgb_pixels.len(), 3);
        assert_eq!(rgb_pixels[0], 200); // R
        assert_eq!(rgb_pixels[1], 150); // G
        assert_eq!(rgb_pixels[2], 100); // B
    }

    #[test]
    fn test_frame_to_rgb_multiple_pixels() {
        // Create a 2x2 BGR frame
        let mat =
            Mat::new_rows_cols_with_default(2, 2, CV_8UC3, Scalar::new(50.0, 100.0, 150.0, 0.0))
                .unwrap();
        let frame = VideoFrame::new(mat);

        let (width, height, rgb_pixels) = frame_to_rgb(&frame).unwrap();

        assert_eq!(width, 2);
        assert_eq!(height, 2);
        assert_eq!(rgb_pixels.len(), 12); // 2x2 pixels * 3 channels
    }

    #[test]
    fn test_frame_to_rgb_black_frame() {
        let mat = Mat::new_rows_cols_with_default(10, 10, CV_8UC3, Scalar::all(0.0)).unwrap();
        let frame = VideoFrame::new(mat);

        let (width, height, rgb_pixels) = frame_to_rgb(&frame).unwrap();

        assert_eq!(width, 10);
        assert_eq!(height, 10);
        assert!(rgb_pixels.iter().all(|&p| p == 0));
    }

    #[test]
    fn test_frame_to_rgb_white_frame() {
        let mat = Mat::new_rows_cols_with_default(10, 10, CV_8UC3, Scalar::all(255.0)).unwrap();
        let frame = VideoFrame::new(mat);

        let (width, height, rgb_pixels) = frame_to_rgb(&frame).unwrap();

        assert_eq!(width, 10);
        assert_eq!(height, 10);
        assert!(rgb_pixels.iter().all(|&p| p == 255));
    }

    #[test]
    fn test_frame_to_rgb_dimensions() {
        let test_cases = vec![(1, 1), (10, 10), (100, 50), (640, 480), (1280, 720)];

        for (width, height) in test_cases {
            let mat = Mat::new_rows_cols_with_default(height, width, CV_8UC3, Scalar::all(128.0))
                .unwrap();
            let frame = VideoFrame::new(mat);

            let (w, h, pixels) = frame_to_rgb(&frame).unwrap();

            assert_eq!(w, width as usize);
            assert_eq!(h, height as usize);
            assert_eq!(pixels.len(), (width * height * 3) as usize);
        }
    }

    #[test]
    fn test_frame_to_rgb_color_accuracy() {
        // Test specific BGR colors
        let test_colors = vec![
            (0.0, 0.0, 255.0, 255, 0, 0),     // Pure red
            (0.0, 255.0, 0.0, 0, 255, 0),     // Pure green
            (255.0, 0.0, 0.0, 0, 0, 255),     // Pure blue
            (255.0, 255.0, 0.0, 0, 255, 255), // Cyan
            (255.0, 0.0, 255.0, 255, 0, 255), // Magenta
            (0.0, 255.0, 255.0, 255, 255, 0), // Yellow
        ];

        for (b, g, r, exp_r, exp_g, exp_b) in test_colors {
            let mat =
                Mat::new_rows_cols_with_default(1, 1, CV_8UC3, Scalar::new(b, g, r, 0.0)).unwrap();
            let frame = VideoFrame::new(mat);

            let (_, _, rgb_pixels) = frame_to_rgb(&frame).unwrap();

            assert_eq!(
                rgb_pixels[0], exp_r,
                "Red mismatch for BGR({},{},{})",
                b, g, r
            );
            assert_eq!(
                rgb_pixels[1], exp_g,
                "Green mismatch for BGR({},{},{})",
                b, g, r
            );
            assert_eq!(
                rgb_pixels[2], exp_b,
                "Blue mismatch for BGR({},{},{})",
                b, g, r
            );
        }
    }

    #[test]
    fn test_frame_to_rgb_chunk_processing() {
        // Test with 4 pixels (12 bytes) - exactly one chunk
        let mat =
            Mat::new_rows_cols_with_default(1, 4, CV_8UC3, Scalar::new(10.0, 20.0, 30.0, 0.0))
                .unwrap();
        let frame = VideoFrame::new(mat);

        let (width, height, rgb_pixels) = frame_to_rgb(&frame).unwrap();

        assert_eq!(width, 4);
        assert_eq!(height, 1);
        assert_eq!(rgb_pixels.len(), 12);

        // Check that all pixels were converted correctly
        for i in 0..4 {
            assert_eq!(rgb_pixels[i * 3], 30); // R
            assert_eq!(rgb_pixels[i * 3 + 1], 20); // G
            assert_eq!(rgb_pixels[i * 3 + 2], 10); // B
        }
    }

    #[test]
    fn test_frame_to_rgb_remainder_processing() {
        // Test with 5 pixels (15 bytes) - 1 chunk + 1 remainder pixel
        let mat =
            Mat::new_rows_cols_with_default(1, 5, CV_8UC3, Scalar::new(40.0, 50.0, 60.0, 0.0))
                .unwrap();
        let frame = VideoFrame::new(mat);

        let (width, height, rgb_pixels) = frame_to_rgb(&frame).unwrap();

        assert_eq!(width, 5);
        assert_eq!(height, 1);
        assert_eq!(rgb_pixels.len(), 15);

        // Check last pixel (remainder)
        assert_eq!(rgb_pixels[12], 60); // R
        assert_eq!(rgb_pixels[13], 50); // G
        assert_eq!(rgb_pixels[14], 40); // B
    }
}
