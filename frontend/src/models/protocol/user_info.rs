//! User Information DTO
//!
//! Represents user information received from the server.

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub user_id: String,
    pub username: String,
    pub state: String,
}
