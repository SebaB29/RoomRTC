//! Integration tests for authentication flows
//!
//! Tests the complete registration and login workflows including:
//! - User registration
//! - User login with valid credentials
//! - Login with invalid credentials
//! - Duplicate username handling

use roomrtc_server::domain::{User, UserState};
use roomrtc_server::infrastructure::storage::Storage;
use std::sync::mpsc;

#[test]
fn test_user_registration_flow() {
    let storage = Storage::new();

    // Create a new user (registration)
    let user = User::new("user001".to_string(), "alice".to_string(), "password123");

    let result = storage.create_user(user);
    assert!(result.is_ok(), "User registration should succeed");

    // Verify user exists in storage
    let retrieved = storage.get_user_by_username("alice");
    assert!(retrieved.is_some(), "Registered user should be retrievable");

    let retrieved_user = retrieved.unwrap();
    assert_eq!(retrieved_user.username, "alice");
    assert_eq!(retrieved_user.id, "user001");
}

#[test]
fn test_duplicate_registration() {
    let storage = Storage::new();

    // Register first user
    let user1 = User::new("user001".to_string(), "bob".to_string(), "password1");
    storage
        .create_user(user1)
        .expect("First registration should succeed");

    // Try to register with same username
    let user2 = User::new("user002".to_string(), "bob".to_string(), "password2");
    let result = storage.create_user(user2);

    assert!(result.is_err(), "Duplicate username should be rejected");
    assert!(result.unwrap_err().contains("already exists"));
}

#[test]
fn test_login_with_valid_credentials() {
    let storage = Storage::new();

    // Register user
    let password = "secure_password";
    let user = User::new("user001".to_string(), "charlie".to_string(), password);
    storage
        .create_user(user)
        .expect("Registration should succeed");

    // Attempt login - retrieve user and verify password
    let retrieved = storage.get_user_by_username("charlie");
    assert!(retrieved.is_some(), "User should exist");

    let user = retrieved.unwrap();
    assert!(user.verify_password(password), "Password should be valid");
}

#[test]
fn test_login_with_invalid_password() {
    let storage = Storage::new();

    // Register user
    let password = "correct_password";
    let user = User::new("user001".to_string(), "dave".to_string(), password);
    storage
        .create_user(user)
        .expect("Registration should succeed");

    // Attempt login with wrong password
    let retrieved = storage.get_user_by_username("dave");
    assert!(retrieved.is_some());

    let user = retrieved.unwrap();
    assert!(
        !user.verify_password("wrong_password"),
        "Invalid password should fail"
    );
}

#[test]
fn test_login_nonexistent_user() {
    let storage = Storage::new();

    let retrieved = storage.get_user_by_username("nonexistent");
    assert!(retrieved.is_none(), "Nonexistent user should not be found");
}

#[test]
fn test_complete_authentication_flow() {
    let storage = Storage::new();

    // Step 1: User registration
    let password = "my_secret";
    let user = User::new("user001".to_string(), "eve".to_string(), password);
    storage
        .create_user(user)
        .expect("Registration should succeed");

    // Step 2: User login (credential verification)
    let retrieved = storage.get_user_by_username("eve");
    assert!(retrieved.is_some());
    let user = retrieved.unwrap();
    assert!(user.verify_password(password));

    // Step 3: Establish connection (user becomes available)
    let (tx, _rx) = mpsc::channel();
    storage
        .connect_user("user001".to_string(), tx)
        .expect("Connection should succeed");

    // Step 4: Verify user state is Available
    let state = storage.get_user_state(&"user001".to_string());
    assert_eq!(state, Some(UserState::Available));

    // Step 5: User logout (disconnect)
    storage
        .disconnect_user(&"user001".to_string())
        .expect("Disconnect should succeed");

    // Step 6: Verify user state is Disconnected
    let state = storage.get_user_state(&"user001".to_string());
    assert_eq!(state, Some(UserState::Disconnected));
}

#[test]
fn test_multiple_user_registration_and_login() {
    let storage = Storage::new();

    // Register multiple users
    let users = vec![
        ("user1", "alice", "pass1"),
        ("user2", "bob", "pass2"),
        ("user3", "charlie", "pass3"),
    ];

    for (id, username, password) in &users {
        let user = User::new(id.to_string(), username.to_string(), password);
        storage
            .create_user(user)
            .expect("Registration should succeed");
    }

    // Verify all users exist and can authenticate
    for (id, username, password) in &users {
        let retrieved = storage.get_user_by_username(username);
        assert!(retrieved.is_some(), "User {} should exist", username);

        let user = retrieved.unwrap();
        assert_eq!(user.id, *id);
        assert!(
            user.verify_password(password),
            "Password for {} should be valid",
            username
        );
    }

    // Verify user count
    let all_users = storage.get_all_users();
    assert_eq!(all_users.len(), 3);
}

#[test]
fn test_case_sensitive_usernames() {
    let storage = Storage::new();

    // Register with lowercase
    let user1 = User::new("user1".to_string(), "frank".to_string(), "pass");
    storage
        .create_user(user1)
        .expect("Registration should succeed");

    // Try to retrieve with different case
    let retrieved_lower = storage.get_user_by_username("frank");
    let retrieved_upper = storage.get_user_by_username("Frank");

    assert!(retrieved_lower.is_some());
    assert!(
        retrieved_upper.is_none(),
        "Usernames should be case-sensitive"
    );
}

#[test]
fn test_password_hashing_consistency() {
    let storage = Storage::new();

    let password = "test_password";
    let user = User::new("user1".to_string(), "george".to_string(), password);

    // Store the hash
    let original_hash = user.password_hash.clone();

    storage
        .create_user(user)
        .expect("Registration should succeed");

    // Retrieve and verify hash is consistent
    let retrieved = storage.get_user_by_username("george").unwrap();
    assert_eq!(retrieved.password_hash, original_hash);
    assert!(retrieved.verify_password(password));
}
