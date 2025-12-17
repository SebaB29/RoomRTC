//! User Card Component
//!
//! Card displaying user information with state indicator

use egui::{Color32, Margin, RichText};

/// User state with associated color and icon
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserState {
    Available,
    Busy,
    Offline,
}

impl UserState {
    /// Normalizes a state string to UserState enum
    pub fn from_str(state: &str) -> Self {
        match state {
            "Available" | "available" => Self::Available,
            "Busy" | "busy" => Self::Busy,
            _ => Self::Offline,
        }
    }

    /// Gets the display text for this state
    pub fn text(&self) -> &'static str {
        match self {
            Self::Available => "Available",
            Self::Busy => "Busy",
            Self::Offline => "Offline",
        }
    }

    /// Gets the color for this state
    pub fn color(&self) -> Color32 {
        match self {
            Self::Available => Color32::from_rgb(34, 197, 94),
            Self::Busy => Color32::from_rgb(251, 191, 36),
            Self::Offline => Color32::from_rgb(100, 116, 139),
        }
    }

    /// Gets the icon for this state
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Available => "✅",
            Self::Busy => "⏳",
            Self::Offline => "🚫",
        }
    }
}

/// User card component
pub struct UserCard {
    username: String,
    state: UserState,
    is_self: bool,
    show_call_button: bool,
}

impl UserCard {
    /// Creates a new user card
    pub fn new(username: String, state: UserState) -> Self {
        Self {
            username,
            state,
            is_self: false,
            show_call_button: true,
        }
    }

    /// Marks this user as the current user
    pub fn mark_as_self(mut self, is_self: bool) -> Self {
        self.is_self = is_self;
        self
    }

    /// Controls whether to show the call button
    pub fn show_call_button(mut self, show: bool) -> Self {
        self.show_call_button = show;
        self
    }

    /// Shows the user card and returns the call button response if clicked
    pub fn show(self, ui: &mut egui::Ui) -> Option<egui::Response> {
        let mut call_response = None;

        let frame = egui::Frame::new()
            .fill(Color32::from_rgb(30, 41, 59))
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(51, 65, 85)))
            .corner_radius(8.0)
            .inner_margin(Margin::symmetric(12, 10));

        frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                // User info (left side)
                ui.vertical(|ui| {
                    let username_text = if self.is_self {
                        format!("{} (You)", &self.username)
                    } else {
                        self.username.clone()
                    };

                    ui.label(
                        RichText::new(&username_text)
                            .size(16.0)
                            .strong()
                            .color(Color32::from_rgb(226, 232, 240)),
                    );
                    ui.add_space(4.0);

                    ui.colored_label(
                        self.state.color(),
                        RichText::new(format!("{}{}", &self.state.icon(), &self.state.text()))
                            .size(12.0),
                    );
                });

                // Call button (right side)
                if self.show_call_button && !self.is_self {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if self.state == UserState::Available {
                            let call_btn = egui::Button::new(
                                RichText::new("📞 Call").size(14.0).color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(59, 130, 246))
                            .corner_radius(8.0)
                            .min_size([100.0, 36.0].into());

                            call_response = Some(ui.add(call_btn));
                        } else {
                            let disabled_btn = egui::Button::new(
                                RichText::new("📞 Call")
                                    .size(14.0)
                                    .color(Color32::from_rgb(100, 116, 139)),
                            )
                            .fill(Color32::from_rgb(51, 65, 85))
                            .corner_radius(8.0)
                            .min_size([100.0, 36.0].into());

                            ui.add_enabled(false, disabled_btn);
                        }
                    });
                }
            });
        });

        call_response
    }
}
