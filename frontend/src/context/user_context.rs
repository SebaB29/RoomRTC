//! User Context
//!
//! Stores the current user's authentication state and room information.

/// User context to store the current user's name and room info
#[derive(Clone, Debug)]
pub struct UserContext {
    pub name: Option<String>,
    pub user_id: Option<String>,
    pub current_room_id: Option<String>,
    pub peer_user_id: Option<String>,
    pub outgoing_call_to: Option<String>,
}

impl UserContext {
    /// Creates a new UserContext instance
    pub fn new() -> Self {
        Self {
            name: None,
            user_id: None,
            current_room_id: None,
            peer_user_id: None,
            outgoing_call_to: None,
        }
    }

    /// Sets the user's name
    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    /// Sets the user's ID
    pub fn set_user_id(&mut self, user_id: String) {
        self.user_id = Some(user_id);
    }

    /// Gets the user's name as a reference
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Gets the user's ID as a reference
    pub fn get_user_id(&self) -> Option<&str> {
        self.user_id.as_deref()
    }

    /// Clears the user context (logout)
    pub fn clear(&mut self) {
        self.name = None;
        self.user_id = None;
        self.current_room_id = None;
        self.peer_user_id = None;
    }
}

impl Default for UserContext {
    fn default() -> Self {
        Self::new()
    }
}
