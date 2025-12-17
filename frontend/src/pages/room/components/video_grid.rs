//! Video Grid Component
//!
//! Manages the 2-column layout for local and remote video streams.

use super::video_placeholder::render_placeholder;
use crate::models::Participant;
use egui::{Color32, FontId, RichText, TextureHandle};

/// Renders the 2-column video grid layout
pub fn render_video_grid(
    ui: &mut egui::Ui,
    user_name: &str,
    my_participant: Option<&Participant>,
    other_participant: Option<&Participant>,
    my_texture: Option<&TextureHandle>,
    other_texture: Option<&TextureHandle>,
) {
    let available_width = ui.available_width();
    let video_width = (available_width - 60.0) / 2.0;
    let video_height = video_width * 0.75;

    ui.horizontal(|ui| {
        ui.add_space(20.0);

        // My video
        render_my_video(
            ui,
            user_name,
            my_participant,
            my_texture,
            video_width,
            video_height,
        );

        ui.add_space(20.0);

        // Other participant's video
        render_other_video(
            ui,
            other_participant,
            other_texture,
            video_width,
            video_height,
        );

        ui.add_space(20.0);
    });
}

/// Renders the local user's video frame
fn render_my_video(
    ui: &mut egui::Ui,
    user_name: &str,
    my_participant: Option<&Participant>,
    my_texture: Option<&TextureHandle>,
    width: f32,
    height: f32,
) {
    ui.vertical(|ui| {
        ui.set_width(width);

        let camera_enabled = my_participant.is_some_and(|p| p.camera_on);
        let label = format!("{} (You)", user_name);

        if camera_enabled {
            if let Some(texture) = my_texture {
                // Render video texture
                ui.image((texture.id(), egui::vec2(width, height)));
            } else {
                render_placeholder(ui, width, height, "Camera Starting...");
            }
        } else {
            render_placeholder(ui, width, height, "Camera Off");
        }

        ui.add_space(10.0);
        ui.label(
            RichText::new(label)
                .font(FontId::proportional(20.0))
                .color(Color32::WHITE),
        );
    });
}

/// Renders the remote participant's video frame
fn render_other_video(
    ui: &mut egui::Ui,
    other_participant: Option<&Participant>,
    other_texture: Option<&TextureHandle>,
    width: f32,
    height: f32,
) {
    ui.vertical(|ui| {
        ui.set_width(width);

        if let Some(other) = other_participant {
            if other.camera_on {
                if let Some(texture) = other_texture {
                    // Render video texture
                    ui.image((texture.id(), egui::vec2(width, height)));
                } else {
                    render_placeholder(ui, width, height, "Waiting for video...");
                }
            } else {
                render_placeholder(ui, width, height, "Camera Off");
            }

            ui.add_space(10.0);
            ui.label(
                RichText::new(&other.name)
                    .font(FontId::proportional(20.0))
                    .color(Color32::WHITE),
            );
        } else {
            render_placeholder(ui, width, height, "Waiting for participant...");
            ui.add_space(10.0);
            ui.label(
                RichText::new("Waiting for participant...")
                    .font(FontId::proportional(20.0))
                    .color(Color32::GRAY),
            );
        }
    });
}
