//! Authentication use cases for login and registration.

use std::io::{self, Read, Write};
use std::sync::mpsc::channel;

use crate::domain::{User, UserId};
use crate::infrastructure::storage::Storage;
use crate::tcp::messages::{
    ErrorMsg, LoginRequest, LoginResponse, LogoutResponse, Message, RegisterRequest,
    RegisterResponse,
};
use crate::tcp::protocol::write_message;

/// Authentication use case handler
pub struct AuthUseCase {
    storage: Storage,
    logger: logging::Logger,
}

impl AuthUseCase {
    pub fn new(storage: Storage, logger: logging::Logger) -> Self {
        AuthUseCase { storage, logger }
    }

    /// Handle login request for any stream type (TLS or plain TCP)
    pub fn handle_login<S: Read + Write>(
        &self,
        req: &LoginRequest,
        stream: &mut S,
    ) -> io::Result<Option<(UserId, std::sync::mpsc::Receiver<Message>)>> {
        self.logger.info(&format!(
            "Received login request for username: {}",
            req.username
        ));

        // Verify credentials
        let user = match self.verify_credentials(&req.username, &req.password_hash) {
            Some(u) => u,
            None => {
                self.logger.error(&format!(
                    "Invalid login attempt for username: {}",
                    req.username
                ));
                self.send_login_failure(stream)?;
                return Ok(None);
            }
        };

        // Check if already logged in
        if self.storage.is_user_logged_in(&user.id) {
            self.logger.error(&format!(
                "User {} already logged in from another session",
                user.username
            ));
            self.send_error(stream, 409, "User already logged in from another session");
            return Ok(None);
        }

        // Complete login process
        self.complete_login(stream, user)
    }

    /// Verify user credentials
    fn verify_credentials(&self, username: &str, password_hash: &str) -> Option<User> {
        self.storage
            .get_user_by_username(username)
            .filter(|user| user.verify_password(password_hash))
    }

    /// Complete login: connect user, send response, return channel
    fn complete_login<S: Read + Write>(
        &self,
        stream: &mut S,
        user: User,
    ) -> io::Result<Option<(UserId, std::sync::mpsc::Receiver<Message>)>> {
        let user_id = user.id.clone();
        let username = user.username.clone();

        // Create channel for sending messages to this client
        let (tx, rx) = channel::<Message>();

        // Connect user (adds connection and broadcasts state change)
        if let Err(e) = self.storage.connect_user(user_id.clone(), tx) {
            self.logger.error(&format!("Failed to connect user: {}", e));
            self.send_error(stream, 500, &format!("Failed to connect user: {}", e));
            return Ok(None);
        }

        // Send success response
        self.send_login_success(stream, &user_id, &username)?;

        self.logger.info(&format!("User {} logged in", username));
        Ok(Some((user_id, rx)))
    }

    /// Send login success response
    fn send_login_success<S: Read + Write>(
        &self,
        stream: &mut S,
        user_id: &str,
        username: &str,
    ) -> io::Result<()> {
        let response = Message::LoginResponse(LoginResponse {
            success: true,
            user_id: Some(user_id.to_string()),
            username: Some(username.to_string()),
            error: None,
        });
        write_message(stream, &response).map_err(io::Error::other)
    }

    /// Send login failure response
    fn send_login_failure<S: Read + Write>(&self, stream: &mut S) -> io::Result<()> {
        let response = Message::LoginResponse(LoginResponse {
            success: false,
            user_id: None,
            username: None,
            error: Some("Invalid credentials".to_string()),
        });
        write_message(stream, &response).map_err(io::Error::other)
    }

    /// Handle logout request
    pub fn handle_register(&self, req: &RegisterRequest) -> io::Result<Message> {
        self.logger.info(&format!(
            "Received registration request for username: {}",
            req.username
        ));

        // Check if username exists
        if self.storage.get_user_by_username(&req.username).is_some() {
            self.logger.error(&format!(
                "Registration failed: username {} already exists",
                req.username
            ));
            return Ok(Message::RegisterResponse(RegisterResponse {
                success: false,
                user_id: None,
                username: None,
                error: Some("Username already exists".to_string()),
            }));
        }

        // Create new user
        let user_id = format!("user_{}", rand::random::<u64>());
        let username = req.username.clone();
        let user = User::new(user_id.clone(), username.clone(), &req.password_hash);

        if let Err(e) = self.storage.create_user(user) {
            self.logger.error(&format!("Failed to create user: {}", e));
            return Ok(Message::RegisterResponse(RegisterResponse {
                success: false,
                user_id: None,
                username: None,
                error: Some(format!("Failed to create user: {}", e)),
            }));
        }

        self.logger
            .info(&format!("User {} registered", req.username));

        Ok(Message::RegisterResponse(RegisterResponse {
            success: true,
            user_id: Some(user_id),
            username: Some(username),
            error: None,
        }))
    }

    /// Handle logout request
    pub fn handle_logout(&self, user_id: &UserId) -> io::Result<Message> {
        self.logger
            .info(&format!("Received logout request for user: {}", user_id));

        // Disconnect user (removes connection, cleans up calls, broadcasts state)
        if let Err(e) = self.storage.disconnect_user(user_id) {
            self.logger
                .error(&format!("Failed to disconnect user during logout: {}", e));
            return Ok(Message::LogoutResponse(LogoutResponse {
                success: false,
                error: Some(format!("Failed to disconnect: {}", e)),
            }));
        }

        self.logger.info(&format!("User {} logged out", user_id));

        Ok(Message::LogoutResponse(LogoutResponse {
            success: true,
            error: None,
        }))
    }

    /// Send error message to client
    fn send_error<S: Read + Write>(&self, stream: &mut S, code: u16, message: &str) {
        let error_msg = Message::Error(ErrorMsg {
            code,
            message: message.to_string(),
        });
        let _ = write_message(stream, &error_msg);
    }
}
