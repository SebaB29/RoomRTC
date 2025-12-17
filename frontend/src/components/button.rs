//! Button Component
//!
//! Provides styled button variants for consistent UI.

use egui::{Color32, FontId, RichText, Vec2};

/// Button variant styles
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Warning,
}

impl ButtonVariant {
    /// Returns the color for this button variant
    fn color(&self) -> Color32 {
        match self {
            ButtonVariant::Primary => Color32::from_rgb(59, 130, 246),
            ButtonVariant::Secondary => Color32::from_rgb(107, 114, 128),
            ButtonVariant::Danger => Color32::from_rgb(239, 68, 68),
            ButtonVariant::Warning => Color32::from_rgb(245, 158, 11),
        }
    }
}

/// A styled button component with configurable appearance
pub struct Button {
    text: String,
    text_size: f32,
    min_size: Option<Vec2>,
    variant: ButtonVariant,
    enabled: bool,
}

impl Button {
    /// Creates a new button with the given label
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            text: label.into(),
            text_size: 16.0,
            min_size: None,
            variant: ButtonVariant::Primary,
            enabled: true,
        }
    }

    /// Sets the minimum size of the button
    pub fn min_size(mut self, size: Vec2) -> Self {
        self.min_size = Some(size);
        self
    }

    /// Sets the button variant (Primary, Secondary, or Danger)
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Sets the text size
    pub fn text_size(mut self, size: f32) -> Self {
        self.text_size = size;
        self
    }

    /// Renders the button and returns the response
    pub fn show(self, ui: &mut egui::Ui) -> egui::Response {
        let mut button = egui::Button::new(
            RichText::new(&self.text)
                .font(FontId::proportional(self.text_size))
                .color(Color32::WHITE),
        )
        .fill(self.variant.color())
        .corner_radius(8.0);

        if let Some(size) = self.min_size {
            button = button.min_size(size);
        }

        ui.add_enabled(self.enabled, button)
    }
}
