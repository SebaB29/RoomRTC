use crate::components::UserAvatar;
use crate::events::UiCommand;
use eframe::egui;

/// User menu dropdown component for logout and user info
pub struct UserMenu {
    is_open: bool,
}

impl UserMenu {
    pub fn new() -> Self {
        Self { is_open: false }
    }

    /// Show the user menu with avatar and dropdown
    pub fn show(&mut self, ui: &mut egui::Ui, username: &str) -> Option<UiCommand> {
        let avatar_response = UserAvatar::new().show(ui, username);

        // Toggle menu on click
        if avatar_response.clicked() {
            self.is_open = !self.is_open;
        }

        // Show dropdown if open
        if self.is_open {
            self.show_dropdown(ui, avatar_response.rect, username)
        } else {
            None
        }
    }

    fn show_dropdown(
        &mut self,
        ui: &egui::Ui,
        button_rect: egui::Rect,
        username: &str,
    ) -> Option<UiCommand> {
        let menu_pos = egui::pos2(button_rect.right() - 220.0, button_rect.bottom() + 8.0);

        egui::Area::new("user_menu".into())
            .fixed_pos(menu_pos)
            .order(egui::Order::Foreground)
            .show(ui.ctx(), |ui| {
                egui::Frame::popup(ui.style())
                    .fill(egui::Color32::from_rgb(30, 41, 59))
                    .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 65, 85)))
                    .corner_radius(8.0)
                    .inner_margin(egui::Margin::same(12))
                    .show(ui, |ui| {
                        ui.set_min_width(200.0);
                        self.render_menu_content(ui, username)
                    })
                    .inner
            })
            .inner
    }

    fn render_menu_content(&mut self, ui: &mut egui::Ui, username: &str) -> Option<UiCommand> {
        ui.vertical(|ui| {
            // Username label
            ui.label(
                egui::RichText::new(username)
                    .size(16.0)
                    .strong()
                    .color(egui::Color32::from_rgb(226, 232, 240)),
            );
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(5.0);

            // Logout button
            let logout_clicked = ui
                .add(
                    egui::Button::new(
                        egui::RichText::new("🚪 Logout")
                            .size(14.0)
                            .color(egui::Color32::WHITE),
                    )
                    .fill(egui::Color32::from_rgb(239, 68, 68))
                    .corner_radius(6.0)
                    .min_size([180.0, 36.0].into()),
                )
                .clicked();

            if logout_clicked {
                self.is_open = false;
                Some(UiCommand::Logout)
            } else {
                None
            }
        })
        .inner
    }
}

impl Default for UserMenu {
    fn default() -> Self {
        Self::new()
    }
}
