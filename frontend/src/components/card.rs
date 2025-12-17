//! Card Component
//!
//! Provides a container with rounded corners and shadow effects.

use egui::Color32;

/// A reusable card component with shadow and rounded corners
pub struct Card {
    max_width: Option<f32>,
    inner_margin: f32,
}

impl Card {
    /// Creates a new card with default settings
    pub fn new() -> Self {
        Self {
            max_width: None,
            inner_margin: 10.0,
        }
    }

    /// Sets the maximum width of the card
    pub fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    /// Sets the inner margin (padding) of the card
    pub fn inner_margin(mut self, margin: f32) -> Self {
        self.inner_margin = margin;
        self
    }

    /// Renders the card with custom content
    pub fn show<R>(
        self,
        ui: &mut egui::Ui,
        add_contents: impl FnOnce(&mut egui::Ui) -> R,
    ) -> egui::InnerResponse<R> {
        egui::Frame::new()
            .fill(Color32::from_rgb(30, 41, 59))
            .corner_radius(12.0)
            .inner_margin(self.inner_margin)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(51, 65, 85)))
            .shadow(egui::epaint::Shadow {
                offset: [0, 4],
                blur: 24,
                spread: 0,
                color: Color32::from_black_alpha(40),
            })
            .show(ui, |ui| {
                if let Some(width) = self.max_width {
                    ui.set_max_width(width);
                }
                add_contents(ui)
            })
    }
}

impl Default for Card {
    fn default() -> Self {
        Self::new()
    }
}
