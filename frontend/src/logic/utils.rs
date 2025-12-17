//! Utility Functions
//!
//! Helper functions for frame conversion and other operations.

use egui::{Color32, ColorImage, Vec2};

/// Converts RGB pixel data to EGUI ColorImage
/// WebRTC now provides RGB data directly, so we just need to wrap it
pub fn rgb_to_color_image(width: usize, height: usize, rgb_pixels: Vec<u8>) -> ColorImage {
    let pixels: Vec<Color32> = rgb_pixels
        .chunks_exact(3)
        .map(|rgb| Color32::from_rgb(rgb[0], rgb[1], rgb[2]))
        .collect();

    ColorImage {
        size: [width, height],
        pixels,
        source_size: Vec2::new(width as f32, height as f32),
    }
}
