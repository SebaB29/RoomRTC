//! Room Page

mod components;
mod state;

pub use state::RoomState;

use crate::events::UiCommand;
use crate::models::{Participant, RoomData};
use egui::TextureHandle;

/// Parameters for rendering the room layout
struct RoomRenderParams<'a> {
    user_name: &'a str,
    room_id: &'a str,
    my_participant: Option<&'a Participant>,
    other_participant: Option<&'a Participant>,
    my_texture: Option<&'a TextureHandle>,
    other_texture: Option<&'a TextureHandle>,
}

pub struct Room;

impl Room {
    pub fn show(
        ui: &mut egui::Ui,
        user_name: &str,
        room_id: &str,
        room_data: Option<&RoomData>,
        my_texture: Option<&TextureHandle>,
        other_texture: Option<&TextureHandle>,
    ) -> Option<UiCommand> {
        let Some(room) = room_data else {
            return Self::render_error(ui);
        };

        let my_participant = room.get_participant(user_name);
        let other_participant = Self::get_other_participant(room, user_name);
        let sidebar_open = Self::load_sidebar_state(ui);
        let stats_visible = Self::load_stats_visibility(ui);
        let (device_id, fps) = Self::get_camera_settings(ui, my_participant);

        let params = RoomRenderParams {
            user_name,
            room_id,
            my_participant,
            other_participant,
            my_texture,
            other_texture,
        };

        let (command, updated_sidebar_open) =
            Self::render_layout(ui, params, room_data?, sidebar_open, stats_visible);

        Self::save_state(
            ui,
            updated_sidebar_open,
            stats_visible,
            my_participant,
            device_id,
            fps,
        );
        ui.ctx().request_repaint();

        command
    }

    /// Gets the other participant in the room
    fn get_other_participant<'a>(room: &'a RoomData, user_name: &str) -> Option<&'a Participant> {
        room.participants.iter().find(|p| p.name != user_name)
    }

    /// Loads sidebar state from UI storage
    fn load_sidebar_state(ui: &egui::Ui) -> bool {
        ui.data(|data| {
            data.get_temp::<bool>(egui::Id::new("settings_sidebar_open"))
                .unwrap_or(false)
        })
    }

    /// Loads stats panel visibility from UI storage
    fn load_stats_visibility(ui: &egui::Ui) -> bool {
        ui.data(|data| {
            data.get_temp::<bool>(egui::Id::new("stats_panel_visible"))
                .unwrap_or(true) // Visible by default
        })
    }

    /// Renders the main layout with sidebar and central panel
    fn render_layout(
        ui: &mut egui::Ui,
        params: RoomRenderParams,
        room_data: &crate::models::RoomData,
        mut sidebar_open: bool,
        stats_visible: bool,
    ) -> (Option<UiCommand>, bool) {
        let mut command = None;

        Self::render_sidebar(ui, params.my_participant, sidebar_open, &mut command);
        Self::render_central_panel(ui, params, &mut sidebar_open, &mut command);
        Self::render_stats_panel(ui, room_data, stats_visible);

        (command, sidebar_open)
    }

    /// Renders the settings sidebar
    fn render_sidebar(
        ui: &mut egui::Ui,
        my_participant: Option<&Participant>,
        sidebar_open: bool,
        command: &mut Option<UiCommand>,
    ) {
        egui::SidePanel::right("settings_sidebar")
            .resizable(false)
            .exact_width(components::SIDEBAR_CONSTANT)
            .show_animated_inside(ui, sidebar_open, |ui| {
                if let Some(settings_cmd) = components::render_settings_sidebar(ui, my_participant)
                {
                    *command = Some(settings_cmd);
                }
            });
    }

    /// Renders the central panel with video grid and controls
    fn render_central_panel(
        ui: &mut egui::Ui,
        params: RoomRenderParams,
        sidebar_open: &mut bool,
        command: &mut Option<UiCommand>,
    ) {
        egui::CentralPanel::default().show_inside(ui, |ui| {
            ui.vertical(|ui| {
                ui.add_space(20.0);
                components::render_header(ui, params.room_id, params.my_participant, sidebar_open);

                ui.add_space(30.0);
                components::render_video_grid(
                    ui,
                    params.user_name,
                    params.my_participant,
                    params.other_participant,
                    params.my_texture,
                    params.other_texture,
                );
                ui.add_space(20.0);

                if let Some(control_cmd) = components::render_controls(ui, params.my_participant) {
                    *command = Some(control_cmd);
                }
            });
        });
    }

    /// Renders the statistics panel (floating window)
    fn render_stats_panel(ui: &mut egui::Ui, room_data: &crate::models::RoomData, visible: bool) {
        use crate::components::{CallStats, render_stats_panel};

        // Use real statistics from room data, or defaults if not available
        let stats = room_data.stats.clone().unwrap_or(CallStats {
            bitrate_mbps: 0.0,
            packet_loss_percent: 0.0,
            jitter_ms: 0.0,
            rtt_ms: 0.0,
            packets_sent: 0,
            packets_received: 0,
        });

        render_stats_panel(ui, &stats, visible);
    }

    /// Saves sidebar state and camera settings to UI storage
    fn save_state(
        ui: &egui::Ui,
        sidebar_open: bool,
        stats_visible: bool,
        my_participant: Option<&Participant>,
        device_id: i32,
        fps: f64,
    ) {
        ui.data_mut(|data| {
            data.insert_temp(egui::Id::new("settings_sidebar_open"), sidebar_open);
            data.insert_temp(egui::Id::new("stats_panel_visible"), stats_visible);
            if my_participant.is_some() {
                data.insert_temp(egui::Id::new("camera_device_id"), (device_id, fps));
            }
        });
    }

    /// Retrieves camera settings from temporary storage or participant data
    fn get_camera_settings(ui: &egui::Ui, my_participant: Option<&Participant>) -> (i32, f64) {
        ui.data(|data| {
            data.get_temp::<(i32, f64)>(egui::Id::new("camera_device_id"))
                .or_else(|| my_participant.map(|p| (p.selected_camera_device, p.camera_fps)))
                .unwrap_or((0, 30.0))
        })
    }

    /// Renders error state when room is not found
    fn render_error(ui: &mut egui::Ui) -> Option<UiCommand> {
        use crate::components::{Button, ButtonVariant};
        use egui::{Color32, FontId, RichText};

        let mut command = None;

        ui.vertical_centered(|ui| {
            ui.add_space(100.0);
            ui.label(
                RichText::new("Room not found")
                    .font(FontId::proportional(32.0))
                    .color(Color32::RED),
            );
            ui.add_space(20.0);
            if Button::new("Exit Room")
                .variant(ButtonVariant::Primary)
                .show(ui)
                .clicked()
            {
                command = Some(UiCommand::ExitRoom);
            }
        });

        command
    }
}
