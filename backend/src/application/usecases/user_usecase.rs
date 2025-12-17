//! User information and listing use cases.

use std::io;

use crate::domain::{UserId, UserState};
use crate::infrastructure::storage::Storage;
use crate::tcp::messages::{Message, UserInfoMsg, UserListResponse};

/// User information use case handler
pub struct UserUseCase {
    storage: Storage,
    logger: logging::Logger,
}

impl UserUseCase {
    pub fn new(storage: Storage, logger: logging::Logger) -> Self {
        UserUseCase { storage, logger }
    }

    /// Handle user list request
    pub fn handle_user_list(&self) -> io::Result<Message> {
        self.logger.info("Fetching user list");

        let users = self.storage.get_all_users();
        let user_list: Vec<UserInfoMsg> = users
            .into_iter()
            .map(|u| self.create_user_info(&u.id, &u.username))
            .collect();

        self.logger
            .info(&format!("Returning {} users", user_list.len()));

        Ok(Message::UserListResponse(UserListResponse {
            users: user_list,
        }))
    }

    /// Create user info message with current state
    fn create_user_info(&self, user_id: &UserId, username: &str) -> UserInfoMsg {
        let state = self
            .storage
            .get_user_state(user_id)
            .unwrap_or(UserState::Disconnected);

        UserInfoMsg {
            user_id: user_id.clone(),
            username: username.to_string(),
            state: state.to_string(),
        }
    }
}
