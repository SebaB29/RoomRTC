//! User Avatar Component
//!
//! Circular avatar button displaying user's initial

use egui::{Color32, RichText, Vec2};

/// User avatar with initial letter
pub struct UserAvatar {
    size: f32,
    background_color: Color32,
    text_color: Color32,
}

impl UserAvatar {
    /// Creates a new avatar with default styling
    pub fn new() -> Self {
        Self {
            size: 50.0,
            background_color: Color32::from_rgb(59, 130, 246), // Blue
            text_color: Color32::WHITE,
        }
    }

    /// Shows the avatar and returns whether it was clicked
    pub fn show(self, ui: &mut egui::Ui, username: &str) -> egui::Response {
        let first_letter = username
            .chars()
            .next()
            .unwrap_or('U')
            .to_uppercase()
            .to_string();

        let text_size = self.size * 0.4;

        ui.add(
            egui::Button::new(
                RichText::new(&first_letter)
                    .size(text_size)
                    .color(self.text_color)
                    .strong(),
            )
            .fill(self.background_color)
            .corner_radius(self.size)
            .min_size(Vec2::splat(self.size)),
        )
    }
}

impl Default for UserAvatar {
    fn default() -> Self {
        Self::new()
    }
}
