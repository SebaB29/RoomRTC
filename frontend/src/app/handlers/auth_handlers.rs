//! Authentication Command Handlers
//!
//! Handlers for login, registration, and logout commands.

use crate::app::state::App;
use crate::infrastructure::TcpClient;
use crate::pages::Page;

impl App {
    /// Handle user registration with the signaling server
    pub(in crate::app) fn handle_register(&mut self, username: String, password: String) {
        self.logger.info(&format!(
            "[AUTH] Registration attempt for user '{}'",
            username
        ));

        // Connect to server
        let client = match self.connect_to_server() {
            Ok(c) => c,
            Err(e) => {
                self.logger
                    .error(&format!("[AUTH] Failed to connect for registration: {}", e));
                self.show_warning(
                    "Signaling server not available. Use local mode (simple login) for offline P2P."
                        .to_string(),
                );
                return;
            }
        };

        // Send registration request
        if let Err(e) = client.register(&username, &password) {
            self.logger.error(&format!(
                "[AUTH] Registration request failed for '{}': {}",
                username, e
            ));
            self.show_error(format!("Registration failed: {}", e));
            return;
        }

        // Send login request
        if let Err(e) = client.login(&username, &password) {
            self.logger.error(&format!(
                "[AUTH] Login request failed for '{}': {}",
                username, e
            ));
            self.show_error(format!("Login failed: {}", e));
            return;
        }

        self.tcp_client = Some(client);
        self.user_context.set_name(username);
        self.show_success("Registration complete...".to_string());
    }

    /// Handle login with password authentication
    pub(in crate::app) fn handle_login_with_password(
        &mut self,
        username: String,
        password: String,
    ) {
        self.logger
            .info(&format!("[AUTH] Login attempt for user '{}'", username));

        // Connect to server
        let client = match self.connect_to_server() {
            Ok(c) => c,
            Err(e) => {
                self.logger
                    .error(&format!("[AUTH] Failed to connect for login: {}", e));
                self.show_error(e);
                return;
            }
        };

        // Send login request
        if let Err(e) = client.login(&username, &password) {
            self.logger.error(&format!(
                "[AUTH] Login request failed for '{}': {}",
                username, e
            ));
            self.show_error(format!("Login failed: {}", e));
            return;
        }

        // Save client and wait async server response
        self.tcp_client = Some(client);
        self.user_context.set_name(username);
        self.show_success("Logging in...".to_string());
    }

    /// Handle logout
    pub(in crate::app) fn handle_logout(&mut self) {
        let username = self
            .user_context
            .get_name()
            .unwrap_or("unknown")
            .to_string();
        self.logger
            .info(&format!("[AUTH] Logout initiated for user '{}'", username));

        // Exit room if in one
        if self.user_context.current_room_id.is_some() {
            self.handle_exit_room();
        }

        // Send logout request if connected
        if let Some(ref client) = self.tcp_client
            && client.logout().is_err()
        {
            self.logger.error(&format!(
                "[AUTH] Failed to send logout request for '{}'",
                username
            ));
        }

        // Clean up session
        self.logger
            .info(&format!("[AUTH] Clearing session for user '{}'", username));
        self.user_context.clear();

        // Disconnect TCP client
        if let Some(mut tcp_client) = self.tcp_client.take() {
            self.logger.info("[AUTH] Disconnecting TCP client");
            tcp_client.disconnect();
        }

        // Return to login page
        self.current_page = Page::Login;
        self.logger.info(&format!(
            "[AUTH] User '{}' logged out successfully",
            username
        ));
        self.show_success("Logged out successfully".to_string());
    }

    /// Handles registration response from server
    pub(in crate::app) fn handle_register_response(
        &mut self,
        username: &str,
        success: bool,
        user_id: Option<String>,
        error: Option<String>,
    ) {
        if !success {
            let err_msg = error.unwrap_or_else(|| "Unknown error".to_string());
            self.logger.error(&format!(
                "[AUTH] Registration failed for '{}': {}",
                username, err_msg
            ));
            self.show_error(format!("Registration failed: {}", err_msg));

            // Disconnect if failed
            self.tcp_client = None;
            return;
        }

        self.logger.info(&format!(
            "[AUTH] Registration successful for user '{}'",
            username
        ));

        let uid = user_id.unwrap_or_else(|| username.to_string());
        self.process_to_lobby(uid, username);
    }

    /// Handles login response and transitions to lobby on success
    pub(in crate::app) fn handle_login_response(
        &mut self,
        username: &String,
        success: bool,
        user_id: Option<String>,
        error: Option<String>,
    ) {
        if !success {
            let err_msg = error.unwrap_or_else(|| "Unknown error".to_string());
            self.logger.error(&format!(
                "[AUTH] Login failed for '{}': {}",
                username, err_msg
            ));
            self.show_error(format!("Login failed: {}", err_msg));

            // Disconnect if failed
            self.tcp_client = None;
            return;
        }

        let uid = user_id.unwrap_or_else(|| username.to_string());
        self.logger.info(&format!(
            "[AUTH] Login successful - user: '{}', user_id: '{}'",
            username, uid
        ));

        self.process_to_lobby(uid, username);
    }

    /// Handles logout response from server
    pub(in crate::app) fn handle_logout_response(
        &mut self,
        username: &String,
        success: bool,
        error: Option<String>,
    ) {
        if !success {
            let err_msg = error.unwrap_or_else(|| "Unknown error".to_string());
            self.logger.error(&format!(
                "[AUTH] Logout failed for '{}': {}",
                username, err_msg
            ));
            return;
        }

        self.logger
            .info(&format!("[AUTH] Logout successful for user '{}'", username));
    }

    // --- Private Helpers ---

    /// Establishes TCP connection with error handling
    fn connect_to_server(&mut self) -> Result<TcpClient, String> {
        let tcp_address = self.config.server_address.clone();
        self.logger
            .info(&format!("[AUTH] Connecting to server at '{}'", tcp_address));

        TcpClient::connect(&tcp_address, &self.logger)
            .map_err(|e| format!("Connection failed: {}", e))
    }

    fn process_to_lobby(&mut self, user_id: String, username: &str) {
        // Store update user info
        self.user_context.set_user_id(user_id);

        // Request user list
        if let Some(ref c) = self.tcp_client {
            self.logger.info("[AUTH] Requesting user list from server");
            let _ = c.request_user_list();
        }

        // Navigate to lobby
        self.current_page = Page::Lobby;
        self.show_success(format!("Welcome, {}!", username));
        self.logger
            .info(&format!("[AUTH] User '{}' navigated to Lobby", username));
    }
}
