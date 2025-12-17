//! Application State
//!
//! This module defines the main application state and initialization logic.
//! It implements the MVU (Model-View-Update) pattern's Controller component.
//!
//! # Architecture
//!
//! The `App` struct contains:
//! - **State**: user context, current page, rooms
//! - **Current Room**: video textures for the active room only
//! - **Communication**: mpsc channels for logic thread coordination
//! - **Notifications**: toast system for user feedback
//!
//! # MVU Loop
//!
//! The `eframe::App::update()` implementation follows this flow:
//! 1. Process events from logic thread (non-blocking)
//! 2. Render current page view (pure function)
//! 3. Handle UI commands from view (state mutations)
//!
//! This ensures unidirectional data flow and predictable state management.

use crate::components::Toast;
use crate::config::AppConfig;
use crate::context::UserContext;
use crate::events::{LogicCommand, LogicEvent};
use crate::infrastructure::TcpClient;
use crate::models::RoomData;
use crate::pages::room::RoomState;
use crate::pages::{Lobby, Page};
use logging::Logger;
use std::sync::mpsc::{Receiver, Sender, channel};

/// Main application state - MVU Controller
pub struct App {
    // Config
    pub(super) config: AppConfig,

    // Logger
    pub(super) logger: Logger,

    // User State
    pub(super) user_context: UserContext,

    // UI State
    pub(super) current_page: Page,
    pub(super) current_toast: Option<Toast>,

    // TCP Client (handles all server communication)
    pub(super) tcp_client: Option<TcpClient>,

    // Lobby State
    pub(super) lobby: Lobby,

    // Active Room
    pub(super) current_room: Option<RoomData>,
    pub(super) current_room_state: Option<RoomState>,

    // WebRTC Logic Thread Communication
    pub(super) logic_cmd_tx: Sender<LogicCommand>,
    pub(super) logic_evt_rx: Receiver<LogicEvent>,

    // File Transfer Dialog State
    pub(super) file_send_dialog_open: bool,
    pub(super) file_send_path_input: String,
    pub(super) incoming_file_offer: Option<IncomingFileOffer>,
    pub(super) active_file_transfer: Option<ActiveFileTransfer>,
    pub(super) pending_send_file: Option<(String, u64)>, // (filename, size) for tracking sender
    pub(super) file_transfer_completion_time: Option<std::time::Instant>, // Track when transfer reaches 100%
}

/// Incoming file offer data for display in dialog
#[derive(Debug, Clone)]
pub struct IncomingFileOffer {
    pub transfer_id: u64,
    pub filename: String,
    pub size: u64,
    // pub mime_type: String, -> If we want to do a security check by tyype we should add this
}

/// Active file transfer (sending or receiving)
#[derive(Debug, Clone)]
pub struct ActiveFileTransfer {
    pub transfer_id: u64,
    pub filename: String,
    pub total_size: u64,
    pub bytes_transferred: u64,
    pub is_sending: bool,
}

impl App {
    /// Create a new App instance with configuration and logger
    pub fn new() -> Self {
        // Load application configuration
        let config = AppConfig::load();

        // Initialize logger from configuration
        let logger = match logging::Logger::with_component(
            config.log_path.clone(),
            config.log_level,
            "Frontend".to_string(),
            false,
        ) {
            Ok(logger) => logger,
            Err(e) => {
                eprintln!("Failed to initialize logger: {}", e);
                std::process::exit(1);
            }
        };

        logger.info("[APP] Initializing application...");
        logger.info(&format!(
            "[APP] Configuration loaded - server: {}, log_level: {:?}",
            config.server_address, config.log_level
        ));

        let (logic_cmd_tx, logic_cmd_rx) = channel();
        let (logic_evt_tx, logic_evt_rx) = channel();

        logger.info("[APP] Starting WebRTC logic thread...");
        let logic_logger = logger.clone();
        std::thread::spawn(move || {
            crate::logic::run_logic_thread(logic_cmd_rx, logic_evt_tx, logic_logger);
        });

        let app = Self {
            config,
            logger: logger.clone(),
            user_context: UserContext::new(),
            current_page: Page::Login,
            current_toast: None,
            tcp_client: None, // Connect on login
            lobby: Lobby::new(),
            current_room: None,
            current_room_state: None,
            logic_cmd_tx,
            logic_evt_rx,
            file_send_dialog_open: false,
            file_send_path_input: String::new(),
            incoming_file_offer: None,
            active_file_transfer: None,
            pending_send_file: None,
            file_transfer_completion_time: None,
        };

        logger.info("[APP] Application initialized successfully");
        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- MVU UPDATE LOOP ---

        // 1. Poll TCP messages (non-blocking)
        if let Some(ref client) = self.tcp_client {
            for message in client.poll_messages() {
                self.handle_server_message(message);
            }
        }

        // 2. Process all pending logic events (from background threads)
        while let Ok(event) = self.logic_evt_rx.try_recv() {
            self.handle_logic_event(ctx, event);
        }

        // 3. Update repaint for active pages
        if self.current_page == Page::Room {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }

        // 4. Request continuous repaints during file transfer for smooth progress updates
        if self.active_file_transfer.is_some() || self.incoming_file_offer.is_some() {
            ctx.request_repaint();
        }

        // 5. Render the view and collect UI commands
        let ui_command = self.render_view(ctx);

        // 5. Process UI command (if any)
        if let Some(command) = ui_command {
            self.handle_ui_command(command);
        }

        // 6. Render toast notification (if any)
        self.render_toast(ctx);

        // 7. Request continuous repaints in Lobby page
        if self.current_page == Page::Lobby {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    /// Called when the app is about to close
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.logger.info("[APP] Application shutting down...");

        if self.tcp_client.is_some() {
            self.logger
                .info("[APP] TCP client will disconnect automatically");
        }

        // Stop any active WebRTC connection
        if self.user_context.current_room_id.is_some() {
            self.logger.info("[APP] Stopping active WebRTC connection");
            let _ = self
                .logic_cmd_tx
                .send(crate::events::LogicCommand::StopConnection);
        }

        self.logger.info("[APP] Cleanup complete, goodbye!");
    }
}

impl App {
    /// Renders the current page view and returns any UI command
    fn render_view(&mut self, ctx: &egui::Context) -> Option<crate::events::UiCommand> {
        use crate::pages::Login;

        // Handle Room page separately due to state initialization logic
        if self.current_page == Page::Room {
            return self.render_room_page(ctx);
        }

        let mut ui_command = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            ui_command = match self.current_page {
                Page::Lobby => self.lobby.show(ctx, self.user_context.get_name()),
                Page::Login => Login::show(ui),
                Page::Room => unreachable!(), // Handled above
            };
        });

        ui_command
    }

    /// Renders the Room page with proper state management
    fn render_room_page(&mut self, ctx: &egui::Context) -> Option<crate::events::UiCommand> {
        use crate::pages::Room;

        // Get current room info
        let room_id = self
            .user_context
            .current_room_id
            .as_deref()
            .unwrap_or("unknown");
        let user_name = self.user_context.get_name().unwrap_or_default();

        // Initialize room state if needed (for video textures)
        if self.current_room_state.is_none() {
            self.current_room_state = Some(RoomState::new());
        }

        let mut ui_command = None;

        egui::CentralPanel::default().show(ctx, |ui| {
            // Get room data and textures
            let room_data = self.current_room.as_ref();

            // Get textures from current room state
            let (my_texture, other_texture) = match &self.current_room_state {
                Some(state) => (state.my_texture.as_ref(), state.other_texture.as_ref()),
                None => (None, None),
            };

            ui_command = Room::show(ui, user_name, room_id, room_data, my_texture, other_texture);
        });

        // Render file send dialog if open
        if self.file_send_dialog_open {
            ui_command = self.render_file_send_dialog(ctx).or(ui_command);
        }

        // Render incoming file offer dialog if present
        if self.incoming_file_offer.is_some() {
            ui_command = self.render_incoming_file_dialog(ctx).or(ui_command);
        }

        // Render file transfer progress popup if present
        if self.active_file_transfer.is_some() {
            ui_command = self.render_file_transfer_progress(ctx).or(ui_command);
        }

        ui_command
    }

    /// Renders the file send dialog modal
    fn render_file_send_dialog(&mut self, ctx: &egui::Context) -> Option<crate::events::UiCommand> {
        use crate::components::{Button, ButtonVariant};
        use crate::events::UiCommand;
        use egui::{Align2, Color32, FontId, RichText, Vec2};
        use std::path::PathBuf;

        let mut ui_command = None;

        egui::Window::new("Send File")
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .fixed_size(Vec2::new(400.0, 150.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(10.0);

                    ui.label(
                        RichText::new("Enter file path to send:")
                            .font(FontId::proportional(16.0))
                            .color(Color32::WHITE),
                    );

                    ui.add_space(10.0);

                    // File path input
                    let response = ui.add_sized(
                        [350.0, 30.0],
                        egui::TextEdit::singleline(&mut self.file_send_path_input)
                            .hint_text("e.g., /home/user/file.txt"),
                    );

                    // Submit on Enter key
                    if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter))
                        && !self.file_send_path_input.is_empty() {
                            let path = PathBuf::from(&self.file_send_path_input);
                            ui_command = Some(UiCommand::SendFileSelected(path));
                        }

                    ui.add_space(15.0);

                    // Buttons
                    ui.horizontal(|ui| {
                        ui.add_space(80.0);

                        if Button::new("Send")
                            .variant(ButtonVariant::Primary)
                            .min_size(Vec2::new(100.0, 35.0))
                            .show(ui)
                            .clicked()
                            && !self.file_send_path_input.is_empty() {
                                let path = PathBuf::from(&self.file_send_path_input);
                                ui_command = Some(UiCommand::SendFileSelected(path));
                            }

                        ui.add_space(10.0);

                        if Button::new("Cancel")
                            .variant(ButtonVariant::Secondary)
                            .min_size(Vec2::new(100.0, 35.0))
                            .show(ui)
                            .clicked()
                        {
                            self.file_send_dialog_open = false;
                        }
                    });
                });
            });

        ui_command
    }

    /// Renders the incoming file offer dialog modal
    fn render_incoming_file_dialog(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<crate::events::UiCommand> {
        use crate::components::{Dialog, action_button};
        use crate::events::UiCommand;

        let offer = match &self.incoming_file_offer {
            Some(offer) => offer.clone(),
            None => return None,
        };

        Dialog::new("incoming_file_dialog")
            .width(420.0)
            .height(240.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    ui.label(
                        egui::RichText::new("📁 Incoming File")
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::from_rgb(96, 165, 250)),
                    );

                    ui.add_space(20.0);

                    ui.label(
                        egui::RichText::new(format!("📄 {}", offer.filename))
                            .size(18.0)
                            .color(egui::Color32::from_rgb(226, 232, 240)),
                    );

                    ui.add_space(8.0);

                    let size_str = if offer.size < 1024 {
                        format!("{} bytes", offer.size)
                    } else if offer.size < 1024 * 1024 {
                        format!("{:.1} KB", offer.size as f64 / 1024.0)
                    } else {
                        format!("{:.1} MB", offer.size as f64 / (1024.0 * 1024.0))
                    };

                    ui.label(
                        egui::RichText::new(format!("Size: {}", size_str))
                            .size(14.0)
                            .color(egui::Color32::from_rgb(148, 163, 184)),
                    );

                    ui.add_space(28.0);

                    // Centered buttons
                    ui.horizontal(|ui| {
                        let button_width = 130.0;
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
                            // Generate save path (Downloads folder or current directory)
                            let save_path = dirs::download_dir()
                                .unwrap_or_else(|| std::env::current_dir().unwrap_or_default())
                                .join(&offer.filename);

                            command = Some(UiCommand::AcceptFileTransfer {
                                transfer_id: offer.transfer_id,
                                save_path,
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
                            command = Some(UiCommand::RejectFileTransfer {
                                transfer_id: offer.transfer_id,
                            });
                            // Clear offer immediately on reject
                            self.incoming_file_offer = None;
                        }

                        command
                    })
                    .inner
                })
                .inner
            })
            .flatten()
    }

    /// Renders the file transfer progress dialog
    fn render_file_transfer_progress(
        &mut self,
        ctx: &egui::Context,
    ) -> Option<crate::events::UiCommand> {
        use crate::components::{Dialog, action_button};
        use crate::events::UiCommand;

        let transfer = match &self.active_file_transfer {
            Some(transfer) => transfer.clone(),
            None => return None,
        };

        let percent = if transfer.total_size > 0 {
            (transfer.bytes_transferred as f32 / transfer.total_size as f32) * 100.0
        } else {
            0.0
        };

        // Auto-dismiss after 2 seconds at 100%
        if percent >= 99.9 {
            if self.file_transfer_completion_time.is_none() {
                // Just reached 100%, start timer
                self.file_transfer_completion_time = Some(std::time::Instant::now());
            } else if let Some(completion_time) = self.file_transfer_completion_time {
                // Check if 2 seconds have passed
                if completion_time.elapsed().as_secs() >= 2 {
                    self.active_file_transfer = None;
                    self.file_transfer_completion_time = None;
                }
            }
        } else {
            // Reset timer if progress drops below 100%
            self.file_transfer_completion_time = None;
        }

        let size_str = |bytes: u64| {
            if bytes < 1024 {
                format!("{} B", bytes)
            } else if bytes < 1024 * 1024 {
                format!("{:.1} KB", bytes as f64 / 1024.0)
            } else {
                format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
            }
        };

        Dialog::new("file_transfer_progress")
            .width(420.0)
            .height(240.0)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(16.0);

                    let icon = if transfer.is_sending { "📤" } else { "📥" };
                    let title = if transfer.is_sending {
                        "Sending File"
                    } else {
                        "Receiving File"
                    };

                    ui.label(
                        egui::RichText::new(format!("{} {}", icon, title))
                            .size(24.0)
                            .strong()
                            .color(egui::Color32::from_rgb(96, 165, 250)),
                    );

                    ui.add_space(12.0);

                    ui.label(
                        egui::RichText::new(format!("📄 {}", transfer.filename))
                            .size(16.0)
                            .color(egui::Color32::from_rgb(226, 232, 240)),
                    );

                    ui.add_space(16.0);

                    // Progress bar
                    let progress_bar = egui::ProgressBar::new(percent / 100.0)
                        .show_percentage()
                        .desired_width(340.0)
                        .desired_height(24.0);
                    ui.add(progress_bar);

                    ui.add_space(8.0);

                    ui.label(
                        egui::RichText::new(format!(
                            "{} / {}",
                            size_str(transfer.bytes_transferred),
                            size_str(transfer.total_size)
                        ))
                        .size(13.0)
                        .color(egui::Color32::from_rgb(148, 163, 184)),
                    );

                    ui.add_space(20.0);

                    // Cancel button
                    let mut command = None;
                    if action_button(ui, "❌ Cancel", egui::Color32::from_rgb(239, 68, 68), 130.0)
                        .clicked()
                    {
                        command = Some(UiCommand::CancelFileTransfer {
                            transfer_id: transfer.transfer_id,
                        });
                    }

                    command
                })
                .inner
            })
            .flatten()
    }

    /// Renders a toast notification if one exists
    fn render_toast(&mut self, ctx: &egui::Context) {
        // Check if we have a toast to display
        if let Some(toast) = &self.current_toast {
            // show() returns true if user clicked dismiss OR toast expired
            if toast.show(ctx) {
                self.current_toast = None;
            }
        }
    }

    /// Shows an error toast notification to the user
    pub(super) fn show_error(&mut self, message: String) {
        self.current_toast = Some(Toast::error(message));
    }

    /// Shows a warning toast notification to the user
    pub(super) fn show_warning(&mut self, message: String) {
        self.current_toast = Some(Toast::warning(message));
    }

    /// Shows a success toast notification to the user
    pub(super) fn show_success(&mut self, message: String) {
        self.current_toast = Some(Toast::success(message));
    }

    /// Shows an info toast notification to the user
    pub(super) fn show_info(&mut self, message: String) {
        self.current_toast = Some(Toast::info(message));
    }

    // --- Helper Methods ---

    /// Gets mutable reference to current room data
    pub(super) fn get_current_room_mut(&mut self) -> Option<&mut RoomData> {
        self.current_room.as_mut()
    }

    /// Gets mutable reference to current user's participant data in current room
    pub(super) fn get_current_participant_mut(
        &mut self,
    ) -> Option<&mut crate::models::Participant> {
        let user_name = self.user_context.get_name()?.to_string();
        let room = self.current_room.as_mut()?;
        room.get_participant_mut(&user_name)
    }

    /// Creates a room with two participants and sets it as current
    pub(super) fn create_room(&mut self, room_id: String, owner_name: String, guest_name: String) {
        let mut room_data = RoomData::new_with_id(room_id.clone());
        room_data.participants.push(crate::models::Participant::new(
            owner_name,
            crate::models::ParticipantRole::Owner,
        ));
        room_data.participants.push(crate::models::Participant::new(
            guest_name,
            crate::models::ParticipantRole::Guest,
        ));
        self.current_room = Some(room_data);
        self.user_context.current_room_id = Some(room_id);
    }

    /// Updates camera state for local participant
    pub(super) fn update_local_camera_state(&mut self, camera_on: bool) {
        if let Some(participant) = self.get_current_participant_mut() {
            participant.camera_on = camera_on;
        }
    }

    /// Updates camera state for remote participant (the other person in the room)
    pub(super) fn update_remote_camera_state(&mut self, camera_on: bool) {
        let username = match self.user_context.get_name() {
            Some(name) => name.to_string(),
            None => return,
        };

        if let Some(room) = self.get_current_room_mut()
            && let Some(participant) = room.participants.iter_mut().find(|p| p.name != username)
        {
            participant.camera_on = camera_on;
        }
    }

    /// Updates audio state for local participant
    pub(super) fn update_local_audio_state(&mut self, audio_on: bool, audio_muted: bool) {
        if let Some(participant) = self.get_current_participant_mut() {
            participant.audio_on = audio_on;
            participant.audio_muted = audio_muted;
        }
    }

    /// Updates audio state for remote participant (the other person in the room)
    pub(super) fn update_remote_audio_state(&mut self, audio_on: bool, audio_muted: bool) {
        let username = match self.user_context.get_name() {
            Some(name) => name.to_string(),
            None => return,
        };

        if let Some(room) = self.get_current_room_mut()
            && let Some(participant) = room.participants.iter_mut().find(|p| p.name != username)
        {
            participant.audio_on = audio_on;
            participant.audio_muted = audio_muted;
        }
    }
}
