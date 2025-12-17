//! Video frame representation.
//!
//! Provides the core `VideoFrame` type for representing raw video frames
//! with metadata throughout the media processing pipeline.

use opencv::core::Mat;
use opencv::prelude::*;

/// Raw video frame
///
/// Wraps an OpenCV Mat with additional metadata (dimensions, timestamp).
/// Used throughout the media pipeline for frame processing and encoding.
#[derive(Clone)]
pub struct VideoFrame {
    data: Mat,
    width: i32,
    height: i32,
    timestamp: std::time::Instant,
}

impl VideoFrame {
    /// Creates a new video frame from an OpenCV Mat
    ///
    /// Automatically captures dimensions and timestamp at creation time.
    ///
    /// # Arguments
    /// * `mat` - OpenCV Matrix containing the frame data (typically BGR format)
    pub fn new(mat: Mat) -> Self {
        let width = mat.cols();
        let height = mat.rows();

        VideoFrame {
            data: mat,
            width,
            height,
            timestamp: std::time::Instant::now(),
        }
    }

    /// Returns frame width in pixels
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns frame height in pixels
    pub fn height(&self) -> i32 {
        self.height
    }

    /// Returns capture timestamp
    ///
    /// Useful for synchronization and latency measurements.
    pub fn timestamp(&self) -> std::time::Instant {
        self.timestamp
    }

    /// Returns reference to the internal OpenCV matrix
    ///
    /// Allows direct manipulation or analysis without cloning.
    pub fn data(&self) -> &Mat {
        &self.data
    }

    /// Consumes the frame and returns the internal Mat
    ///
    /// Use when transferring ownership to another component.
    pub fn into_mat(self) -> Mat {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use opencv::core::{CV_8UC3, Mat, Scalar};

    #[test]
    fn test_frame_creation() {
        let mat = Mat::default();
        let frame = VideoFrame::new(mat);

        assert_eq!(frame.width(), 0);
        assert_eq!(frame.height(), 0);
    }

    #[test]
    fn test_frame_with_data() {
        let mat = Mat::new_rows_cols_with_default(
            480,
            640,
            CV_8UC3,
            Scalar::new(100.0, 150.0, 200.0, 0.0),
        )
        .unwrap();
        let frame = VideoFrame::new(mat);

        assert_eq!(frame.width(), 640);
        assert_eq!(frame.height(), 480);
    }

    #[test]
    fn test_frame_into_mat() {
        let mat = Mat::new_rows_cols_with_default(
            480,
            640,
            CV_8UC3,
            Scalar::new(100.0, 150.0, 200.0, 0.0),
        )
        .unwrap();
        let frame = VideoFrame::new(mat);
        let recovered_mat = frame.into_mat();

        assert_eq!(recovered_mat.cols(), 640);
        assert_eq!(recovered_mat.rows(), 480);
    }

    #[test]
    fn test_frame_data_reference() {
        let mat = Mat::new_rows_cols_with_default(
            480,
            640,
            CV_8UC3,
            Scalar::new(100.0, 150.0, 200.0, 0.0),
        )
        .unwrap();
        let frame = VideoFrame::new(mat);
        let data_ref = frame.data();

        assert_eq!(data_ref.cols(), 640);
        assert_eq!(data_ref.rows(), 480);
    }

    #[test]
    fn test_frame_timestamp() {
        let mat = Mat::default();
        let before = std::time::Instant::now();
        let frame = VideoFrame::new(mat);
        let after = std::time::Instant::now();

        assert!(frame.timestamp() >= before && frame.timestamp() <= after);
    }
}
