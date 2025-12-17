//! User state domain model

/// Represents the current state of a user in the system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserState {
    /// User is not connected to the server
    Disconnected,
    /// User is connected and available for calls
    Available,
    /// User is connected and currently in a call
    Busy,
}

impl std::fmt::Display for UserState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UserState::Disconnected => write!(f, "Disconnected"),
            UserState::Available => write!(f, "Available"),
            UserState::Busy => write!(f, "Busy"),
        }
    }
}
