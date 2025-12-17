//! RTCP Statistics Display Panel
//! Displays real-time call quality metrics:
//! - Bitrate (Mbps)
//! - Packet Loss (%)
//! - Jitter (ms)
//! - RTT (ms)
//!
//! Color coding: Green (good), Yellow (warning), Red (poor)

use egui::{Color32, RichText, Ui};

/// Statistics data structure
#[derive(Clone, PartialEq, Default, Debug)]
pub struct CallStats {
    pub bitrate_mbps: f64,
    pub packet_loss_percent: f64,
    pub jitter_ms: f64,
    pub rtt_ms: f64,
    pub packets_sent: u32,
    pub packets_received: u32,
}

/// Quality indicator levels
#[derive(Clone, Copy, PartialEq)]
pub enum QualityLevel {
    Good,
    Warning,
    Poor,
}

impl QualityLevel {
    fn color(&self) -> Color32 {
        match self {
            QualityLevel::Good => Color32::from_rgb(76, 175, 80), // Green
            QualityLevel::Warning => Color32::from_rgb(255, 152, 0), // Orange
            QualityLevel::Poor => Color32::from_rgb(244, 67, 54), // Red
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            QualityLevel::Good => "✅",
            QualityLevel::Warning => "💡",
            QualityLevel::Poor => "❌",
        }
    }
}

/// Determine quality level based on thresholds
fn bitrate_quality(mbps: f64) -> QualityLevel {
    if mbps >= 2.0 {
        QualityLevel::Good
    } else if mbps >= 1.0 {
        QualityLevel::Warning
    } else {
        QualityLevel::Poor
    }
}

fn packet_loss_quality(percent: f64) -> QualityLevel {
    if percent <= 1.0 {
        QualityLevel::Good
    } else if percent <= 3.0 {
        QualityLevel::Warning
    } else {
        QualityLevel::Poor
    }
}

fn jitter_quality(ms: f64) -> QualityLevel {
    if ms <= 20.0 {
        QualityLevel::Good
    } else if ms <= 50.0 {
        QualityLevel::Warning
    } else {
        QualityLevel::Poor
    }
}

fn rtt_quality(ms: f64) -> QualityLevel {
    if ms <= 150.0 {
        QualityLevel::Good
    } else if ms <= 300.0 {
        QualityLevel::Warning
    } else {
        QualityLevel::Poor
    }
}

/// Renders the statistics panel
///
/// # Arguments
/// * `ui` - The egui UI context
/// * `stats` - Current call statistics
/// * `visible` - Whether the panel should be displayed
pub fn render_stats_panel(ui: &mut Ui, stats: &CallStats, visible: bool) {
    if !visible {
        return;
    }

    egui::Window::new("📊 Call Statistics")
        .default_width(280.0)
        .default_pos([10.0, 10.0])
        .resizable(false)
        .collapsible(true)
        .show(ui.ctx(), |ui| {
            ui.vertical(|ui| {
                render_stat_row(
                    ui,
                    "Bitrate",
                    &format!("{:.2} Mbps", stats.bitrate_mbps),
                    bitrate_quality(stats.bitrate_mbps),
                );

                ui.add_space(5.0);

                render_stat_row(
                    ui,
                    "Packet Loss",
                    &format!("{:.2}%", stats.packet_loss_percent),
                    packet_loss_quality(stats.packet_loss_percent),
                );

                ui.add_space(5.0);

                render_stat_row(
                    ui,
                    "Jitter",
                    &format!("{:.1} ms", stats.jitter_ms),
                    jitter_quality(stats.jitter_ms),
                );

                ui.add_space(5.0);

                render_stat_row(
                    ui,
                    "RTT",
                    &format!("{:.0} ms", stats.rtt_ms),
                    rtt_quality(stats.rtt_ms),
                );

                ui.add_space(5.0);
                ui.separator();
                ui.add_space(5.0);

                // Packet counters
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Packets:")
                            .size(12.0)
                            .color(Color32::LIGHT_GRAY),
                    );
                    ui.label(
                        RichText::new(format!(
                            "⬆{} ⬇{}",
                            stats.packets_sent, stats.packets_received
                        ))
                        .size(12.0)
                        .color(Color32::WHITE),
                    );
                });
            });
        });
}

/// Renders a single statistic row with quality indicator
fn render_stat_row(ui: &mut Ui, label: &str, value: &str, quality: QualityLevel) {
    egui::Frame::new()
        .fill(Color32::from_rgb(45, 55, 72))
        .inner_margin(8.0)
        .corner_radius(4.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Quality icon
                ui.label(
                    RichText::new(quality.icon())
                        .size(16.0)
                        .color(quality.color()),
                );

                // Label
                ui.label(
                    RichText::new(format!("{}:", label))
                        .size(13.0)
                        .color(Color32::LIGHT_GRAY),
                );

                // Value with quality color
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(
                        RichText::new(value)
                            .size(14.0)
                            .color(quality.color())
                            .strong(),
                    );
                });
            });
        });
}
