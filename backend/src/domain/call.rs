//! Call domain model - Core business entity for video call management

use crate::domain::{CallState, UserId};

/// Call entity representing an active video call session
#[derive(Debug, Clone)]
pub struct Call {
    pub call_id: String,
    pub caller_id: UserId,
    pub callee_id: UserId,
    pub state: CallState,
}

impl Call {
    /// Creates a new call in Ringing state
    pub fn new(caller_id: UserId, callee_id: UserId) -> Self {
        let call_id = Self::generate_call_id();

        Call {
            call_id,
            caller_id,
            callee_id,
            state: CallState::Ringing,
        }
    }

    /// Generates a unique call ID using timestamp and random number
    fn generate_call_id() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};

        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time before UNIX epoch")
            .as_millis();

        let random: u32 = rand::random();

        format!("call_{}_{:08x}", timestamp, random)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_creation() {
        let caller = "alice".to_string();
        let callee = "bob".to_string();

        let call = Call::new(caller.clone(), callee.clone());

        assert_eq!(call.caller_id, caller);
        assert_eq!(call.callee_id, callee);
        assert_eq!(call.state, CallState::Ringing);
        assert!(call.call_id.starts_with("call_"));
    }

    #[test]
    fn test_call_id_uniqueness() {
        let caller = "alice".to_string();
        let callee = "bob".to_string();

        let call1 = Call::new(caller.clone(), callee.clone());
        let call2 = Call::new(caller.clone(), callee.clone());

        assert_ne!(
            call1.call_id, call2.call_id,
            "Each call should have a unique ID"
        );
    }
}
