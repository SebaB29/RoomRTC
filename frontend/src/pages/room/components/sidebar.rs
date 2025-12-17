//! Settings Sidebar
//!
//! This module contains the settings sidebar component for camera configuration.
//! Users can adjust camera device ID and FPS settings using a dropdown selector.

use crate::components::{Button, ButtonVariant, EmptyState};
use crate::events::UiCommand;
use crate::models::Participant;
use egui::{Color32, ComboBox, FontId, RichText};

const SIDEBAR_WIDTH: f32 = 320.0;

pub const SIDEBAR_CONSTANT: f32 = SIDEBAR_WIDTH;

/// Renders the camera settings sidebar
pub fn render_settings_sidebar(
    ui: &mut egui::Ui,
    my_participant: Option<&Participant>,
) -> Option<UiCommand> {
    let mut command = None;

    egui::Frame::new()
        .fill(Color32::from_rgb(31, 41, 55))
        .inner_margin(20.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                render_header(ui);

                let (current_device, current_fps) = get_current_settings(my_participant);
                let mut settings = load_temp_settings(ui, current_device, current_fps);

                render_camera_selector(ui, &mut settings.0);
                ui.add_space(20.0);

                render_fps_setting(ui, &mut settings.1);
                ui.add_space(10.0);

                render_info_note(ui);
                ui.add_space(10.0);

                command = handle_settings_changes(ui, &settings, current_device, current_fps);
                save_temp_settings(ui, settings);
            });
        });

    command
}

/// Renders the header section
fn render_header(ui: &mut egui::Ui) {
    ui.label(
        RichText::new("⚙ Camera Settings")
            .font(FontId::proportional(24.0))
            .color(Color32::WHITE),
    );
    ui.add_space(10.0);
    ui.separator();
    ui.add_space(10.0);
}

/// Gets current settings from participant or defaults
fn get_current_settings(my_participant: Option<&Participant>) -> (i32, f64) {
    my_participant
        .map(|p| (p.selected_camera_device, p.camera_fps))
        .unwrap_or((0, 30.0))
}

/// Loads temporary settings from UI storage
fn load_temp_settings(ui: &egui::Ui, current_device: i32, current_fps: f64) -> (i32, f64) {
    ui.data(|data| {
        data.get_temp::<(i32, f64)>(egui::Id::new("temp_camera_settings"))
            .unwrap_or((current_device, current_fps))
    })
}

/// Saves temporary settings to UI storage
fn save_temp_settings(ui: &egui::Ui, settings: (i32, f64)) {
    ui.data_mut(|data| {
        data.insert_temp(egui::Id::new("temp_camera_settings"), settings);
    });
}

/// Handles settings changes and returns command if apply is clicked
fn handle_settings_changes(
    ui: &mut egui::Ui,
    settings: &(i32, f64),
    current_device: i32,
    current_fps: f64,
) -> Option<UiCommand> {
    let settings_changed = settings.0 != current_device || (settings.1 - current_fps).abs() > 0.01;

    if settings_changed {
        render_settings_changed_warning(ui);
        if render_apply_button(ui) {
            return Some(UiCommand::UpdateCameraSettings(settings.0, settings.1));
        }
    }
    None
}

/// Renders warning when settings have changed
fn render_settings_changed_warning(ui: &mut egui::Ui) {
    ui.vertical_centered(|ui| {
        ui.label(
            RichText::new("⚠ Settings changed")
                .size(14.0)
                .color(Color32::YELLOW),
        );
        ui.add_space(5.0);
        ui.label(
            RichText::new("Turn camera OFF, apply settings, then turn camera ON")
                .size(12.0)
                .color(Color32::LIGHT_GRAY),
        );
        ui.add_space(10.0);
    });
}

/// Renders apply button and returns true if clicked
fn render_apply_button(ui: &mut egui::Ui) -> bool {
    ui.vertical_centered(|ui| {
        Button::new("Apply Settings")
            .variant(ButtonVariant::Primary)
            .show(ui)
            .clicked()
    })
    .inner
}

/// Renders camera selector with dropdown showing detected cameras
fn render_camera_selector(ui: &mut egui::Ui, device_id: &mut i32) {
    ui.label(
        RichText::new("Select Camera Device")
            .font(FontId::proportional(16.0))
            .color(Color32::LIGHT_GRAY),
    );
    ui.add_space(5.0);

    let detected_ids = get_detected_camera_ids(ui);

    if detected_ids.is_empty() {
        render_no_cameras_detected(ui, device_id);
    } else {
        render_camera_dropdown(ui, device_id, &detected_ids);
        ui.add_space(10.0);
        render_refresh_button(ui);
        ui.add_space(10.0);
        render_custom_id_section(ui, device_id);
    }
}

/// Renders UI when no cameras are detected
fn render_no_cameras_detected(ui: &mut egui::Ui, device_id: &mut i32) {
    EmptyState::new("⚠️", "No cameras detected")
        .description("Enter a camera ID manually below")
        .icon_size(32.0)
        .message_size(14.0)
        .show(ui);

    ui.add_space(10.0);
    render_manual_camera_input(ui, device_id);
}

/// Renders camera dropdown with detected devices
fn render_camera_dropdown(ui: &mut egui::Ui, device_id: &mut i32, detected_ids: &[i32]) {
    let selected_text = if detected_ids.contains(device_id) {
        format!("Camera Device {}", device_id)
    } else {
        format!("Custom ID: {}", device_id)
    };

    ComboBox::from_id_salt("camera_device_selector")
        .selected_text(selected_text)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            for id in detected_ids {
                let label = get_camera_label(*id);
                ui.selectable_value(device_id, *id, label);
            }
        });
}

/// Renders refresh button for camera list
fn render_refresh_button(ui: &mut egui::Ui) {
    if ui.button("Refresh Camera List").clicked() {
        ui.data_mut(|data| {
            data.remove::<Vec<i32>>(egui::Id::new("detected_camera_ids"));
        });
    }
}

/// Renders collapsible custom ID section
fn render_custom_id_section(ui: &mut egui::Ui, device_id: &mut i32) {
    ui.collapsing("⚙ Advanced: Enter Custom ID", |ui| {
        render_manual_camera_input(ui, device_id);
    });
}

/// Gets a user-friendly label for a camera device ID
fn get_camera_label(device_id: i32) -> String {
    match device_id {
        0 => "Camera 0 (Built-in)".to_string(),
        10 => "Camera 10 (Virtual)".to_string(),
        _ => format!("Camera Device {}", device_id),
    }
}

/// Renders manual camera ID input
fn render_manual_camera_input(ui: &mut egui::Ui, device_id: &mut i32) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new("Camera ID:")
                .size(12.0)
                .color(Color32::LIGHT_GRAY),
        );

        let mut device_id_str = device_id.to_string();
        if egui::TextEdit::singleline(&mut device_id_str)
            .desired_width(60.0)
            .font(FontId::proportional(14.0))
            .show(ui)
            .response
            .changed()
            && let Ok(id) = device_id_str.parse::<i32>()
            && id >= 0
        {
            *device_id = id;
        }
    });

    ui.add_space(3.0);
    ui.label(
        RichText::new("Common: 0 (built-in)")
            .size(11.0)
            .color(Color32::DARK_GRAY),
    );
}

/// Gets detected camera IDs with caching (fast - no connection tests)
fn get_detected_camera_ids(ui: &egui::Ui) -> Vec<i32> {
    if let Some(ids) =
        ui.data(|data| data.get_temp::<Vec<i32>>(egui::Id::new("detected_camera_ids")))
    {
        return ids;
    }

    // Get camera IDs (fast - only checks /dev/video* or returns 0-3)
    let ids = webrtc::WebRtcConnection::list_camera_ids_fast();

    let ids_clone = ids.clone();
    ui.data_mut(|data| {
        data.insert_temp(egui::Id::new("detected_camera_ids"), ids_clone);
    });

    ids
}

/// Renders FPS setting
fn render_fps_setting(ui: &mut egui::Ui, fps: &mut f64) {
    ui.label(
        RichText::new("Frames Per Second (FPS)")
            .font(FontId::proportional(16.0))
            .color(Color32::LIGHT_GRAY),
    );
    ui.add_space(10.0);

    // FPS dropdown with common values
    let selected_fps_text = format!("{:.0} FPS", fps);

    ComboBox::from_id_salt("fps_selector")
        .selected_text(selected_fps_text)
        .width(ui.available_width())
        .show_ui(ui, |ui| {
            // ui.selectable_value(fps, 15.0, "15 FPS");
            // ui.selectable_value(fps, 24.0, "24 FPS");
            ui.selectable_value(fps, 30.0, "30 FPS (Recommended)");
            // ui.selectable_value(fps, 60.0, "60 FPS");
        });
}

/// Renders info note
fn render_info_note(ui: &mut egui::Ui) {
    egui::Frame::new()
        .fill(Color32::from_rgb(45, 55, 72))
        .inner_margin(10.0)
        .outer_margin(10.0)
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(
                    RichText::new("ℹ Camera Tips")
                        .size(14.0)
                        .color(Color32::from_rgb(147, 197, 253))
                        .strong(),
                );
                ui.add_space(5.0);
                ui.label(
                    RichText::new("• Most built-in cameras are ID: 0")
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                );
                ui.label(
                    RichText::new("• Virtual cameras (OBS, etc.) are usually ID: 10-13")
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                );
                ui.label(
                    RichText::new("• Resolution auto-adjusts to HD (720p) or best available")
                        .size(12.0)
                        .color(Color32::LIGHT_GRAY),
                );
            });
        });
}
