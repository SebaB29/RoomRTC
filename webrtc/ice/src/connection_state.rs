//! ICE connection state types.
//!
//! Defines the various states that an ICE connection can be in
//! during the connectivity establishment process.

/// Represents the ICE connection state according to RFC 5245.
///
/// The connection state tracks the progress of ICE connectivity establishment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    /// ICE agent is gathering candidates
    #[default]
    New,
    /// ICE agent has gathered candidates and is ready to start checks
    Checking,
    /// At least one working candidate pair has been found
    Connected,
    /// ICE checks have completed and a candidate pair has been selected
    Completed,
    /// ICE checks have failed
    Failed,
    /// ICE connection has been disconnected
    Disconnected,
    /// ICE connection has been closed
    Closed,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::New => write!(f, "new"),
            Self::Checking => write!(f, "checking"),
            Self::Connected => write!(f, "connected"),
            Self::Completed => write!(f, "completed"),
            Self::Failed => write!(f, "failed"),
            Self::Disconnected => write!(f, "disconnected"),
            Self::Closed => write!(f, "closed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_new() {
        // Verifica que el valor por defecto sea ConnectionState::New
        let state = ConnectionState::default();
        assert_eq!(state, ConnectionState::New);
    }

    #[test]
    fn test_display_outputs_correct_strings() {
        // Verifica que cada estado se convierta correctamente en string
        assert_eq!(ConnectionState::New.to_string(), "new");
        assert_eq!(ConnectionState::Checking.to_string(), "checking");
        assert_eq!(ConnectionState::Connected.to_string(), "connected");
        assert_eq!(ConnectionState::Completed.to_string(), "completed");
        assert_eq!(ConnectionState::Failed.to_string(), "failed");
        assert_eq!(ConnectionState::Disconnected.to_string(), "disconnected");
        assert_eq!(ConnectionState::Closed.to_string(), "closed");
    }

    #[test]
    fn test_equality_between_states() {
        // Verifica que la comparación de igualdad funcione correctamente
        assert_eq!(ConnectionState::Connected, ConnectionState::Connected);
        assert_ne!(ConnectionState::Connected, ConnectionState::Failed);
    }

    #[test]
    fn test_debug_format_contains_variant_name() {
        // Verifica que el formato Debug incluya el nombre del estado
        let debug_str = format!("{:?}", ConnectionState::Checking);
        assert!(debug_str.contains("Checking"));
    }
}
