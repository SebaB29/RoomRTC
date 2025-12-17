//! Dialog Component
//!
//! Modal dialog window with consistent styling

use egui::{Align2, Color32, Margin, Vec2};

/// Modal dialog window builder
pub struct Dialog {
    title: String,
    width: f32,
    height: f32,
}

impl Dialog {
    /// Creates a new dialog with title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            width: 400.0,
            height: 220.0,
        }
    }

    /// Sets the dialog width
    pub fn width(mut self, width: f32) -> Self {
        self.width = width;
        self
    }

    /// Sets the dialog height
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Shows the dialog and runs the content closure
    pub fn show<R>(
        self,
        ctx: &egui::Context,
        content: impl FnOnce(&mut egui::Ui) -> R,
    ) -> Option<R> {
        let mut result = None;

        egui::Window::new(&self.title)
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size([self.width, self.height])
            .frame(
                egui::Frame::new()
                    .fill(Color32::from_rgb(30, 41, 59))
                    .stroke(egui::Stroke::new(1.0, Color32::from_rgb(51, 65, 85)))
                    .corner_radius(12.0)
                    .inner_margin(Margin::same(24)),
            )
            .show(ctx, |ui| {
                result = Some(content(ui));
            });

        result
    }
}

/// Helper to show an action button with consistent sizing
pub fn action_button(ui: &mut egui::Ui, text: &str, color: Color32, width: f32) -> egui::Response {
    let button = egui::Button::new(egui::RichText::new(text).size(16.0).color(Color32::WHITE))
        .fill(color)
        .corner_radius(8.0)
        .min_size(Vec2::new(width, 44.0));

    ui.add(button)
}
