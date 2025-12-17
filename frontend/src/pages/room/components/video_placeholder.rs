//! Video Placeholder Component
//!
//! Displays a placeholder box when video is not available.

use egui::{Color32, FontId, RichText, Vec2};

/// Renders a placeholder box with a message
pub(super) fn render_placeholder(ui: &mut egui::Ui, width: f32, height: f32, text: &str) {
    egui::Frame::new()
        .fill(Color32::from_rgb(45, 55, 72))
        .corner_radius(8.0)
        .show(ui, |ui| {
            ui.set_min_size(Vec2::new(width, height));
            ui.vertical_centered(|ui| {
                ui.add_space(height / 2.0 - 20.0);
                ui.label(
                    RichText::new(text)
                        .font(FontId::proportional(20.0))
                        .color(Color32::GRAY),
                );
            });
        });
}
