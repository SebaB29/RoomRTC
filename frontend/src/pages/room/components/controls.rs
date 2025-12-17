//! Room Control Buttons
//!
//! Camera toggle, send file, and exit room buttons.

use crate::components::{Button, ButtonVariant};
use crate::events::UiCommand;
use crate::models::Participant;
use egui::Vec2;

/// Renders control buttons (toggle camera, toggle microphone, send file, exit room)
pub fn render_controls(
    ui: &mut egui::Ui,
    my_participant: Option<&Participant>,
) -> Option<UiCommand> {
    let mut command = None;

    ui.horizontal(|ui| {
        let available_width = ui.available_width() - 740.0; // Adjusted for four buttons
        ui.add_space(available_width / 2.0);

        if let Some(cmd) = render_camera_toggle(ui, my_participant) {
            command = Some(cmd);
        }

        if let Some(cmd) = render_microphone_toggle(ui, my_participant) {
            command = Some(cmd);
        }

        if let Some(cmd) = render_send_file_button(ui, my_participant) {
            command = Some(cmd);
        }

        if let Some(cmd) = render_exit_button(ui) {
            command = Some(cmd);
        }
    });

    command
}

/// Renders the camera toggle button
fn render_camera_toggle(
    ui: &mut egui::Ui,
    my_participant: Option<&Participant>,
) -> Option<UiCommand> {
    let participant = my_participant?;

    let (button_text, button_variant) = get_camera_button_config(participant.camera_on);

    let clicked = Button::new(button_text)
        .variant(button_variant)
        .min_size(Vec2::new(170.0, 50.0))
        .show(ui)
        .clicked();

    ui.add_space(20.0);

    if clicked {
        Some(UiCommand::ToggleCamera)
    } else {
        None
    }
}

/// Gets the button text and variant based on camera state
fn get_camera_button_config(camera_on: bool) -> (&'static str, ButtonVariant) {
    if camera_on {
        ("🎥 Turn Off Camera", ButtonVariant::Secondary)
    } else {
        ("🎥 Turn On Camera", ButtonVariant::Primary)
    }
}

/// Renders the microphone toggle button
fn render_microphone_toggle(
    ui: &mut egui::Ui,
    my_participant: Option<&Participant>,
) -> Option<UiCommand> {
    let participant = my_participant?;

    let (button_text, button_variant) =
        get_microphone_button_config(participant.audio_on, participant.audio_muted);

    let clicked = Button::new(button_text)
        .variant(button_variant)
        .min_size(Vec2::new(170.0, 50.0))
        .show(ui)
        .clicked();

    ui.add_space(20.0);

    if clicked {
        Some(UiCommand::ToggleMute)
    } else {
        None
    }
}

/// Gets the button text and variant based on audio state
fn get_microphone_button_config(
    audio_on: bool,
    audio_muted: bool,
) -> (&'static str, ButtonVariant) {
    if !audio_on {
        ("🎤 Unmute", ButtonVariant::Primary)
    } else if audio_muted {
        ("🔇 Unmute", ButtonVariant::Warning)
    } else {
        ("🔊 Mute", ButtonVariant::Secondary)
    }
}

/// Renders the send file button
fn render_send_file_button(
    ui: &mut egui::Ui,
    my_participant: Option<&Participant>,
) -> Option<UiCommand> {
    // Only show if we have a participant (in a call)
    let _participant = my_participant?;

    let clicked = Button::new("📁 Send File")
        .variant(ButtonVariant::Secondary)
        .min_size(Vec2::new(150.0, 50.0))
        .show(ui)
        .clicked();

    ui.add_space(20.0);

    if clicked {
        Some(UiCommand::SendFile)
    } else {
        None
    }
}

/// Renders the exit room button
fn render_exit_button(ui: &mut egui::Ui) -> Option<UiCommand> {
    let clicked = Button::new("Exit Room")
        .variant(ButtonVariant::Danger)
        .min_size(Vec2::new(170.0, 50.0))
        .show(ui)
        .clicked();

    if clicked {
        Some(UiCommand::ExitRoom)
    } else {
        None
    }
}
