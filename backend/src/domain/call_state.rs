//! Call state domain model

/// Possible states of a call in its lifecycle
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CallState {
    /// Call initiated, waiting for callee response
    Ringing,
    /// Call accepted and in progress
    Active,
}

impl std::fmt::Display for CallState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CallState::Ringing => write!(f, "Ringing"),
            CallState::Active => write!(f, "Active"),
        }
    }
}
