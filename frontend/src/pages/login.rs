use crate::components::{Button, Card};
use crate::events::UiCommand;

#[derive(Clone)]
struct LoginState {
    username: String,
    password: String,
    is_login_mode: bool, // true = login, false = register
    password_response: Option<egui::Response>,
}

impl Default for LoginState {
    fn default() -> Self {
        Self {
            username: String::new(),
            password: String::new(),
            is_login_mode: true,
            password_response: None,
        }
    }
}

pub struct Login;

impl Login {
    pub fn show(ui: &mut egui::Ui) -> Option<UiCommand> {
        let mut login_state = ui.data_mut(|data| {
            data.get_temp::<LoginState>(egui::Id::new("login_state"))
                .unwrap_or_default()
        });

        let mut command = None;

        ui.vertical_centered(|ui| {
            ui.add_space(80.0);

            // Header with title and subtitle
            Self::render_header(ui);
            ui.add_space(40.0);

            // Auth form card
            Card::new()
                .max_width(420.0)
                .inner_margin(25.0)
                .show(ui, |ui| {
                    Self::render_mode_tabs(ui, &mut login_state);
                    ui.add_space(20.0);

                    Self::render_username_input(ui, &mut login_state);
                    ui.add_space(15.0);

                    Self::render_password_input(ui, &mut login_state);
                    ui.add_space(20.0);

                    command = Self::render_submit_button(ui, &mut login_state);
                });
        });

        ui.data_mut(|data| {
            data.insert_temp(egui::Id::new("login_state"), login_state);
        });

        command
    }

    fn render_header(ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            let title = egui::RichText::new("RoomRTC")
                .size(80.0)
                .color(egui::Color32::from_rgb(96, 165, 250))
                .font(egui::FontId::proportional(100.0))
                .strong();

            ui.heading(title);
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("WebRTC Video Conferencing")
                    .size(24.0)
                    .color(egui::Color32::from_rgb(148, 163, 184)),
            );
        });
    }

    fn render_mode_tabs(ui: &mut egui::Ui, login_state: &mut LoginState) {
        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut login_state.is_login_mode,
                true,
                egui::RichText::new("🔐 Login").size(18.0).strong(),
            );
            ui.selectable_value(
                &mut login_state.is_login_mode,
                false,
                egui::RichText::new("📝 Register").size(18.0).strong(),
            );
        });
    }

    fn render_username_input(ui: &mut egui::Ui, login_state: &mut LoginState) {
        Self::render_text_input(
            ui,
            "Username",
            &mut login_state.username,
            "Enter your username",
            false,
        );
    }

    fn render_password_input(ui: &mut egui::Ui, login_state: &mut LoginState) {
        login_state.password_response = Some(Self::render_text_input(
            ui,
            "Password",
            &mut login_state.password,
            "Enter your password",
            true,
        ));
    }

    fn render_text_input(
        ui: &mut egui::Ui,
        label: &str,
        value: &mut String,
        hint: &str,
        is_password: bool,
    ) -> egui::Response {
        ui.label(
            egui::RichText::new(label)
                .font(egui::FontId::proportional(16.0))
                .strong()
                .color(egui::Color32::from_rgb(226, 232, 240)),
        );
        ui.add_space(8.0);

        let text_edit = egui::TextEdit::singleline(value)
            .hint_text(hint)
            .password(is_password)
            .font(egui::FontId::proportional(16.0))
            .margin(egui::Margin::same(12));

        ui.add_sized(egui::vec2(340.0, 40.0), text_edit)
    }

    fn render_submit_button(ui: &mut egui::Ui, login_state: &mut LoginState) -> Option<UiCommand> {
        let button_label = if login_state.is_login_mode {
            "🔐 Login"
        } else {
            "📝 Register"
        };

        let button_clicked = Button::new(button_label)
            .text_size(18.0)
            .min_size(egui::vec2(340.0, 48.0))
            .show(ui)
            .clicked();

        let enter_pressed = Self::check_enter_pressed(login_state, ui);

        if (button_clicked || enter_pressed) && Self::is_valid_input(login_state) {
            return Self::create_command(login_state);
        }

        None
    }

    fn check_enter_pressed(login_state: &LoginState, ui: &egui::Ui) -> bool {
        login_state
            .password_response
            .as_ref()
            .is_some_and(|r| r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))
    }

    fn is_valid_input(login_state: &LoginState) -> bool {
        !login_state.username.trim().is_empty() && !login_state.password.trim().is_empty()
    }

    fn create_command(login_state: &mut LoginState) -> Option<UiCommand> {
        if login_state.is_login_mode {
            Some(UiCommand::LoginWithPassword {
                username: std::mem::take(&mut login_state.username),
                password: std::mem::take(&mut login_state.password),
            })
        } else {
            Some(UiCommand::Register {
                username: std::mem::take(&mut login_state.username),
                password: std::mem::take(&mut login_state.password),
            })
        }
    }
}
