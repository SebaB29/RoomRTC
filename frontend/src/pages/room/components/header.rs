//! Room Header Component
//!
//! Displays room ID, user role, and settings toggle button.

use crate::models::{Participant, ParticipantRole};
use egui::{Color32, FontId, RichText};

/// Renders the room header with room ID, role, and settings button
pub fn render_header(
    ui: &mut egui::Ui,
    room_id: &str,
    my_participant: Option<&Participant>,
    sidebar_open: &mut bool,
) {
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        render_room_title(ui, room_id);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(20.0);
            render_settings_button(ui, sidebar_open);
            ui.add_space(10.0);
            render_role_label(ui, my_participant);
        });
    });
}

/// Renders the room title
fn render_room_title(ui: &mut egui::Ui, room_id: &str) {
    ui.label(
        RichText::new(format!("Room: {}", room_id))
            .font(FontId::proportional(32.0))
            .color(Color32::WHITE),
    );
}

/// Renders the settings toggle button
fn render_settings_button(ui: &mut egui::Ui, sidebar_open: &mut bool) {
    let settings_icon = if *sidebar_open { "x" } else { "⚙" };
    if ui
        .button(
            RichText::new(settings_icon)
                .font(FontId::proportional(24.0))
                .color(Color32::WHITE),
        )
        .on_hover_text("Toggle Settings")
        .clicked()
    {
        *sidebar_open = !*sidebar_open;
    }
}

/// Renders the participant role label
fn render_role_label(ui: &mut egui::Ui, my_participant: Option<&Participant>) {
    if let Some(participant) = my_participant {
        let role_text = match participant.role {
            ParticipantRole::Owner => "Owner",
            ParticipantRole::Guest => "Guest",
        };
        ui.label(
            RichText::new(format!("Role: {}", role_text))
                .font(FontId::proportional(18.0))
                .color(Color32::LIGHT_GRAY),
        );
    }
}
