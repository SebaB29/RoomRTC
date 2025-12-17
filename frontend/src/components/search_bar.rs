//! SearchBar Component
//!
//! Reusable search input with consistent styling and icon.

use egui::{Color32, FontId, RichText, TextEdit, Ui};

/// SearchBar component for consistent search input across the app
pub struct SearchBar<'a> {
    query: &'a mut String,
    placeholder: String,
    width: Option<f32>,
}

impl<'a> SearchBar<'a> {
    /// Creates a new SearchBar with the given query string
    pub fn new(query: &'a mut String) -> Self {
        Self {
            query,
            placeholder: "Search...".to_string(),
            width: None,
        }
    }

    /// Sets the placeholder text
    pub fn placeholder(mut self, text: impl Into<String>) -> Self {
        self.placeholder = text.into();
        self
    }

    /// Sets a specific width for the search bar
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Renders the search bar and returns the response
    pub fn show(self, ui: &mut Ui) -> egui::Response {
        ui.horizontal(|ui| {
            // Search icon
            ui.label(
                RichText::new("🔍")
                    .size(18.0)
                    .color(Color32::from_rgb(156, 163, 175)),
            );

            // Search label
            ui.label(
                egui::RichText::new("Search: ")
                    .size(16.0)
                    .color(egui::Color32::from_rgb(226, 232, 240)),
            );
            ui.add_space(2.0);

            // Search input
            let mut text_edit = TextEdit::singleline(self.query)
                .hint_text(self.placeholder)
                .font(FontId::proportional(16.0));

            if let Some(w) = self.width {
                text_edit = text_edit.desired_width(w);
            }

            text_edit.show(ui).response
        })
        .inner
    }
}
