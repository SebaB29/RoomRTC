//! Frontend application entry point.

// Application modules
mod app;
mod components;
mod config;
mod context;
mod events;
mod infrastructure;
mod logic;
mod models;
mod pages;

use app::App;

fn main() {
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("RoomRTC - Video Conferencing"),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "RoomRTC",
        native_options,
        Box::new(move |cc| {
            cc.egui_ctx.set_theme(egui::Theme::Dark);

            // Set dark theme with blue tones
            let mut style = (*cc.egui_ctx.style()).clone();
            style.visuals.window_fill = egui::Color32::from_rgb(15, 23, 42);
            style.visuals.panel_fill = egui::Color32::from_rgb(15, 23, 42);
            cc.egui_ctx.set_style(style);

            Ok(Box::new(App::new()))
        }),
    );
}
