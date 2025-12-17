use crate::components::{
    Dialog, EmptyState, SearchBar, UserCard, UserMenu, UserState, action_button,
};
use crate::models::protocol::UserInfo;
use eframe::egui;

/// Lobby page showing available users
pub struct Lobby {
    pub users: Vec<UserInfo>,
    pub incoming_call: Option<IncomingCall>,
    search_query: String,
    user_menu: UserMenu,
}

#[derive(Clone)]
pub struct IncomingCall {
    pub call_id: String,
    pub from_user_id: String,
    pub from_username: String,
}

impl Lobby {
    pub fn new() -> Self {
        Lobby {
            users: Vec::new(),
            incoming_call: None,
            search_query: String::new(),
            user_menu: UserMenu::new(),
        }
    }

    /// Render the lobby UI
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        username: Option<&str>,
    ) -> Option<crate::events::UiCommand> {
        let mut command = None;

        // Show incoming call dialog if present
        if let Some(ref call) = self.incoming_call.clone() {
            command = self.show_incoming_call_dialog(ctx, call);
            if command.is_some() {
                self.incoming_call = None; // Clear after handling
            }
            return command;
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical(|ui| {
                // Header section with title and user info
                self.show_header(ui, username, &mut command);
                ui.add_space(30.0);

                // Search bar
                self.render_search_bar(ui);
                ui.add_space(10.0);

                // User list
                self.render_user_list(ui, &mut command, username);
                ui.add_space(10.0);
            });
        });

        command
    }

    /// Show header section
    fn show_header(
        &mut self,
        ui: &mut egui::Ui,
        username: Option<&str>,
        command: &mut Option<crate::events::UiCommand>,
    ) {
        ui.horizontal(|ui| {
            ui.add_space(ui.available_width() - 60.0);
            // User menu with avatar
            if let Some(name) = username
                && let Some(cmd) = self.user_menu.show(ui, name) {
                    *command = Some(cmd);
                }

            ui.add_space(30.0);
        });

        // Main title centered
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                let total_width = ui.available_width();
                ui.add_space((total_width - 260.0) / 2.0);

                let title = egui::RichText::new("🏠Lobby")
                    .size(64.0)
                    .color(egui::Color32::from_rgb(96, 165, 250))
                    .font(egui::FontId::monospace(64.0))
                    .strong();

                ui.heading(title);
            });
        });
    }

    /// Render search bar
    fn render_search_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let available_width = ui.available_width();
            let card_width = 700.0f32.min(available_width - 40.0);
            let left_margin = (available_width - card_width) / 2.0;

            ui.add_space(left_margin);

            ui.horizontal(|ui| {
                ui.set_width(card_width);
                SearchBar::new(&mut self.search_query)
                    .placeholder("Type user name...")
                    .width(card_width)
                    .show(ui);
            });
        });
    }

    fn render_user_list(
        &mut self,
        ui: &mut egui::Ui,
        command: &mut Option<crate::events::UiCommand>,
        username: Option<&str>,
    ) {
        ui.horizontal(|ui| {
            // Calculate centered card width
            let available_width = ui.available_width();
            let card_width = 700.0f32.min(available_width - 40.0);
            let left_margin = (available_width - card_width) / 2.0;

            ui.add_space(left_margin);

            // Container card - fixed height for 5 users
            let card_frame = egui::Frame::new()
                .fill(egui::Color32::from_rgb(30, 41, 59))
                .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(51, 65, 85)))
                .corner_radius(12.0)
                .inner_margin(egui::Margin::same(20));

            card_frame.show(ui, |ui| {
                ui.set_width(card_width - 40.0);
                ui.set_height(360.0); // Approximate height for 5 users (60px per user + spacing)

                if self.users.is_empty() {
                    self.show_empty_users_state(ui);
                } else {
                    self.show_users_list(ui, command, username);
                }
            });
        });
    }

    /// Show empty state message
    fn show_empty_users_state(&self, ui: &mut egui::Ui) {
        EmptyState::new("👥", "No users online")
            .description("Users will appear here when they connect")
            .show(ui);
    }

    /// Show users list with pagination and scroll
    fn show_users_list(
        &mut self,
        ui: &mut egui::Ui,
        command: &mut Option<crate::events::UiCommand>,
        current_username: Option<&str>,
    ) {
        // Get filtered users (excluding current user)
        let filtered_users = self.get_filtered_users(current_username);

        // Add ScrollArea to allow scrolling within the page
        egui::ScrollArea::vertical()
            .max_height(360.0)
            .auto_shrink([false, false])
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for user in filtered_users {
                        if let Some(cmd) = self.show_user_row(ui, user, current_username) {
                            *command = Some(cmd);
                        }
                    }
                });
            });
    }

    /// Get filtered and sorted users (connected first, then disconnected)
    fn get_filtered_users(&self, current_username: Option<&str>) -> Vec<&UserInfo> {
        let mut users: Vec<&UserInfo> = self
            .users
            .iter()
            .filter(|user| self.should_include_user(user, current_username))
            .collect();

        // Sort: connected users first (available before busy, alphabetically), then offline (alphabetically)
        users.sort_by(|a, b| {
            let a_connected = Self::is_connected(&a.state);
            let b_connected = Self::is_connected(&b.state);

            match (a_connected, b_connected) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                (true, true) => Self::compare_users(a, b),
                (false, false) => a.username.to_lowercase().cmp(&b.username.to_lowercase()),
            }
        });

        users
    }

    /// Normalize state string to compare consistently
    fn normalize_state(state: &str) -> &str {
        match state {
            "Available" | "available" => "available",
            "Busy" | "busy" => "busy",
            "Offline" | "offline" => "offline",
            _ => "offline",
        }
    }

    /// Check if user is connected (available or busy)
    fn is_connected(state: &str) -> bool {
        matches!(Self::normalize_state(state), "available" | "busy")
    }

    /// Check if user matches search query
    fn matches_search(&self, user: &UserInfo) -> bool {
        if self.search_query.is_empty() {
            return true;
        }
        let query_lower = self.search_query.to_lowercase();
        user.username.to_lowercase().contains(&query_lower)
            || user.user_id.to_lowercase().contains(&query_lower)
    }

    /// Check if user should be included in the list
    fn should_include_user(&self, user: &UserInfo, current_username: Option<&str>) -> bool {
        let is_not_self = current_username.is_none_or(|name| user.username != name);
        is_not_self && self.matches_search(user)
    }

    /// Compare users for sorting (available < busy, then alphabetical)
    fn compare_users(a: &UserInfo, b: &UserInfo) -> std::cmp::Ordering {
        let a_state = Self::normalize_state(&a.state);
        let b_state = Self::normalize_state(&b.state);

        // Available < Busy, then alphabetical
        match (a_state, b_state) {
            ("available", "busy") => std::cmp::Ordering::Less,
            ("busy", "available") => std::cmp::Ordering::Greater,
            _ => a.username.to_lowercase().cmp(&b.username.to_lowercase()),
        }
    }

    /// Show a single user row
    fn show_user_row(
        &self,
        ui: &mut egui::Ui,
        user: &UserInfo,
        current_username: Option<&str>,
    ) -> Option<crate::events::UiCommand> {
        let is_self = current_username.is_some() && current_username == Some(&user.username);
        let state = UserState::from_str(&user.state);

        let card = UserCard::new(user.username.clone(), state)
            .mark_as_self(is_self)
            .show_call_button(!is_self);

        let call_clicked = card.show(ui).is_some_and(|response| response.clicked());

        ui.add_space(10.0);

        if call_clicked {
            return Some(crate::events::UiCommand::CallUser(user.user_id.clone()));
        }

        None
    }

    /// Show incoming call dialog
    fn show_incoming_call_dialog(
        &self,
        ctx: &egui::Context,
        call: &IncomingCall,
    ) -> Option<crate::events::UiCommand> {
        Dialog::new("incoming_call_dialog")
            .width(400.0)
            .height(220.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    ui.label(
                        egui::RichText::new("📞 Incoming Call")
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::from_rgb(96, 165, 250)),
                    );

                    ui.add_space(20.0);

                    ui.label(
                        egui::RichText::new(format!("👤 {} is calling you", call.from_username))
                            .size(16.0)
                            .color(egui::Color32::from_rgb(226, 232, 240)),
                    );

                    ui.add_space(28.0);

                    // Botones centrados
                    ui.horizontal(|ui| {
                        let button_width = 120.0;
                        let spacing = 16.0;
                        let total_width = button_width * 2.0 + spacing;
                        let available = ui.available_width();
                        let margin = (available - total_width) / 2.0;

                        ui.add_space(margin);

                        let mut command = None;

                        // Accept button (green)
                        if action_button(
                            ui,
                            "✅ Accept",
                            egui::Color32::from_rgb(34, 197, 94),
                            button_width,
                        )
                        .clicked()
                        {
                            command = Some(crate::events::UiCommand::AcceptCall {
                                call_id: call.call_id.clone(),
                                caller_id: call.from_user_id.clone(),
                                caller_name: call.from_username.clone(),
                            });
                        }

                        ui.add_space(spacing);

                        // Decline button (red)
                        if action_button(
                            ui,
                            "❌ Decline",
                            egui::Color32::from_rgb(239, 68, 68),
                            button_width,
                        )
                        .clicked()
                        {
                            command =
                                Some(crate::events::UiCommand::DeclineCall(call.call_id.clone()));
                        }

                        command
                    })
                    .inner
                })
                .inner
            })
            .flatten()
    }

    /// Update user list
    pub fn update_users(&mut self, users: Vec<UserInfo>) {
        for new_user in users {
            if let Some(existing) = self
                .users
                .iter_mut()
                .find(|u| u.user_id == new_user.user_id)
            {
                existing.username = new_user.username;
                existing.state = new_user.state;
            } else {
                self.users.push(new_user);
            }
        }

        // Remove users no longer in the new list
        let new_user_ids: Vec<String> = self.users.iter().map(|u| u.user_id.clone()).collect();
        self.users.retain(|u| new_user_ids.contains(&u.user_id));
    }

    /// Add/update a single user (for state updates)
    pub fn update_user_state(&mut self, user_id: &str, username: &str, state: &str) {
        // Find existing user
        if let Some(user) = self.users.iter_mut().find(|u| u.user_id == user_id) {
            user.state = state.to_string();
        } else {
            // Add new user
            self.users.push(UserInfo {
                user_id: user_id.to_string(),
                username: username.to_string(),
                state: state.to_string(),
            });
        }
    }

    /// Set incoming call
    pub fn set_incoming_call(
        &mut self,
        call_id: String,
        from_user_id: String,
        from_username: String,
    ) {
        self.incoming_call = Some(IncomingCall {
            call_id,
            from_user_id,
            from_username,
        });
    }

    /// Clear incoming call
    pub fn clear_incoming_call(&mut self) {
        self.incoming_call = None;
    }
}

impl Default for Lobby {
    fn default() -> Self {
        Self::new()
    }
}
