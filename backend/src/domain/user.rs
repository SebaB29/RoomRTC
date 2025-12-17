//! User model for authentication and user management

use crate::domain::UserId;

/// User information stored in memory
#[derive(Debug, Clone)]
pub struct User {
    pub id: UserId,
    pub username: String,
    pub password_hash: String,
    pub created_at: u64,
}

impl User {
    /// Create a new user with hashed password
    pub fn new(id: UserId, username: String, password: &str) -> Self {
        Self {
            id,
            username,
            password_hash: simple_hash(password),
            created_at: current_timestamp(),
        }
    }

    /// Verify provided password against stored hash
    pub fn verify_password(&self, password: &str) -> bool {
        self.password_hash == simple_hash(password)
    }
}

/// Simple hash function using std library only
fn simple_hash(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut hash: u64 = 0;
    for (i, &byte) in bytes.iter().enumerate() {
        hash = hash.rotate_left(5) ^ (byte as u64) ^ (i as u64);
    }
    format!("{:016x}", hash)
}

/// Get current Unix timestamp
fn current_timestamp() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("System time is before UNIX_EPOCH - clock may be incorrect")
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_creation() {
        let user = User::new("user123".to_string(), "alice".to_string(), "password123");

        assert_eq!(user.id, "user123");
        assert_eq!(user.username, "alice");
        assert_ne!(user.password_hash, "password123"); // Hash should differ
        assert!(user.created_at > 0);
    }

    #[test]
    fn test_password_verification_success() {
        let user = User::new("user456".to_string(), "bob".to_string(), "secret");

        assert!(user.verify_password("secret"));
    }

    #[test]
    fn test_password_verification_failure() {
        let user = User::new("user789".to_string(), "charlie".to_string(), "correct");

        assert!(!user.verify_password("wrong"));
        assert!(!user.verify_password(""));
        assert!(!user.verify_password("Correct")); // Case sensitive
    }
}
