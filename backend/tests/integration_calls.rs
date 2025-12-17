//! Integration tests for call management flows
//!
//! Tests the complete call lifecycle including:
//! - Call creation and initiation
//! - Call acceptance
//! - Call state transitions
//! - Call termination
//! - Multi-user call scenarios

use roomrtc_server::domain::{CallState, User, UserState};
use roomrtc_server::infrastructure::storage::Storage;
use std::sync::mpsc;

#[test]
fn test_call_creation_flow() {
    let storage = Storage::new();

    // Register and connect two users
    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass1");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass2");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    // Create call
    let call = storage
        .create_call("user1".to_string(), "user2".to_string())
        .expect("Call creation should succeed");

    assert_eq!(call.caller_id, "user1");
    assert_eq!(call.callee_id, "user2");
    assert_eq!(call.state, CallState::Ringing);
}

#[test]
fn test_call_acceptance_flow() {
    let storage = Storage::new();

    // Setup users
    let user1 = User::new("user1".to_string(), "caller".to_string(), "pass1");
    let user2 = User::new("user2".to_string(), "callee".to_string(), "pass2");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    // Create call (Ringing state)
    let call = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();

    let call_id = call.call_id.clone();

    // Accept call (transition to Active)
    storage
        .update_call_state(&call_id, CallState::Active)
        .expect("Call acceptance should succeed");

    // Verify call is active
    let updated_call = storage.get_call(&call_id).unwrap();
    assert_eq!(updated_call.state, CallState::Active);
}

#[test]
fn test_call_state_transitions() {
    let storage = Storage::new();

    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    // Initial state: both users available
    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Available)
    );
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Available)
    );

    // Create call: both users still available (ringing)
    let call = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();

    // Accept call: both users become busy
    storage
        .update_call_state(&call.call_id, CallState::Active)
        .unwrap();

    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Busy)
    );
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Busy)
    );

    // End call: both users become available again
    storage.remove_call(&call.call_id);

    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Available)
    );
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Available)
    );
}

#[test]
fn test_call_termination() {
    let storage = Storage::new();

    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    let call = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();

    let call_id = call.call_id.clone();

    // Terminate call
    let removed = storage.remove_call(&call_id);
    assert!(removed.is_some());

    // Verify call no longer exists
    assert!(storage.get_call(&call_id).is_none());
}

#[test]
fn test_user_disconnect_during_call() {
    let storage = Storage::new();

    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    // Create active call
    let call = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();

    let call_id = call.call_id.clone();

    storage
        .update_call_state(&call_id, CallState::Active)
        .unwrap();

    // User1 disconnects
    storage.disconnect_user(&"user1".to_string()).unwrap();

    // Call should be cleaned up
    assert!(storage.get_call(&call_id).is_none());

    // User1 should be disconnected
    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Disconnected)
    );

    // User2 should be available again
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Available)
    );
}

#[test]
fn test_get_user_active_call() {
    let storage = Storage::new();

    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    // Initially no active calls
    assert!(storage.get_user_active_call(&"user1".to_string()).is_none());
    assert!(storage.get_user_active_call(&"user2".to_string()).is_none());

    // Create call
    let call = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();

    // Both users should have active call
    let user1_call = storage.get_user_active_call(&"user1".to_string());
    let user2_call = storage.get_user_active_call(&"user2".to_string());

    assert!(user1_call.is_some());
    assert!(user2_call.is_some());

    assert_eq!(user1_call.unwrap().call_id, call.call_id);
    assert_eq!(user2_call.unwrap().call_id, call.call_id);
}

#[test]
fn test_multiple_sequential_calls() {
    let storage = Storage::new();

    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    // First call
    let call1 = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();
    storage
        .update_call_state(&call1.call_id, CallState::Active)
        .unwrap();

    // End first call
    storage.remove_call(&call1.call_id);

    // Second call
    let call2 = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();

    assert_ne!(call1.call_id, call2.call_id, "Call IDs should be unique");
    assert_eq!(call2.state, CallState::Ringing);
}

#[test]
fn test_call_between_disconnected_users() {
    let storage = Storage::new();

    // Create users but don't connect them
    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    // Try to create call (should succeed - validation happens at application layer)
    let result = storage.create_call("user1".to_string(), "user2".to_string());
    assert!(result.is_ok());
}

#[test]
fn test_complete_call_lifecycle() {
    let storage = Storage::new();

    // Step 1: Register users
    let user1 = User::new("user1".to_string(), "alice".to_string(), "pass1");
    let user2 = User::new("user2".to_string(), "bob".to_string(), "pass2");

    storage.create_user(user1).unwrap();
    storage.create_user(user2).unwrap();

    // Step 2: Users connect
    let (tx1, _rx1) = mpsc::channel();
    let (tx2, _rx2) = mpsc::channel();

    storage.connect_user("user1".to_string(), tx1).unwrap();
    storage.connect_user("user2".to_string(), tx2).unwrap();

    // Step 3: Verify users are available
    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Available)
    );
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Available)
    );

    // Step 4: User1 calls User2
    let call = storage
        .create_call("user1".to_string(), "user2".to_string())
        .unwrap();
    assert_eq!(call.state, CallState::Ringing);

    // Step 5: User2 accepts the call
    storage
        .update_call_state(&call.call_id, CallState::Active)
        .unwrap();

    // Step 6: Verify both users are busy
    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Busy)
    );
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Busy)
    );

    // Step 7: Call ends
    storage.remove_call(&call.call_id);

    // Step 8: Verify both users are available again
    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Available)
    );
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Available)
    );

    // Step 9: Users disconnect
    storage.disconnect_user(&"user1".to_string()).unwrap();
    storage.disconnect_user(&"user2".to_string()).unwrap();

    // Step 10: Verify users are disconnected
    assert_eq!(
        storage.get_user_state(&"user1".to_string()),
        Some(UserState::Disconnected)
    );
    assert_eq!(
        storage.get_user_state(&"user2".to_string()),
        Some(UserState::Disconnected)
    );
}
