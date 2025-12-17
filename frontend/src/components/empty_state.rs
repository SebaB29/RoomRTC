//! EmptyState Component
//!
//! Displays a centered empty state with icon and message for better UX.

use egui::{Color32, RichText, Ui};

/// EmptyState component for displaying empty content states
pub struct EmptyState {
    icon: String,
    message: String,
    description: Option<String>,
    icon_size: f32,
    message_size: f32,
}

impl EmptyState {
    /// Creates a new EmptyState with an icon and message
    pub fn new(icon: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            icon: icon.into(),
            message: message.into(),
            description: None,
            icon_size: 48.0,
            message_size: 18.0,
        }
    }

    /// Adds an optional description text below the message
    pub fn description(mut self, text: impl Into<String>) -> Self {
        self.description = Some(text.into());
        self
    }

    /// Sets the icon size (default: 48.0)
    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = size;
        self
    }

    /// Sets the message text size (default: 18.0)
    pub fn message_size(mut self, size: f32) -> Self {
        self.message_size = size;
        self
    }

    /// Renders the empty state centered in the available space
    pub fn show(self, ui: &mut Ui) {
        ui.vertical_centered(|ui| {
            ui.add_space(40.0);

            // Icon
            ui.label(RichText::new(&self.icon).size(self.icon_size));

            ui.add_space(15.0);

            // Main message
            ui.label(
                RichText::new(&self.message)
                    .size(self.message_size)
                    .color(Color32::from_rgb(156, 163, 175)),
            );

            // Optional description
            if let Some(desc) = self.description {
                ui.add_space(8.0);
                ui.label(
                    RichText::new(desc)
                        .size(14.0)
                        .color(Color32::from_rgb(107, 114, 128)),
                );
            }

            ui.add_space(40.0);
        });
    }
}
