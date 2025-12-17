//! In-memory storage for users, connections, and calls

use crate::domain::{Call, CallState, User, UserId, UserState};
use crate::infrastructure::persistence;
use crate::tcp::messages::{Message, UserStateUpdateMsg};
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

/// Thread-safe in-memory storage for the application
///
/// Manages two types of data:
/// 1. Persistent data (users) - loaded from and saved to disk
/// 2. Volatile data (connections, calls) - only exists at runtime
#[derive(Clone)]
pub struct Storage {
    // Persistent data
    users: Arc<Mutex<HashMap<UserId, User>>>,
    username_to_id: Arc<Mutex<HashMap<String, UserId>>>,

    // Runtime data
    connections: Arc<Mutex<HashMap<UserId, Sender<Message>>>>,
    active_calls: Arc<Mutex<HashMap<String, Call>>>,
}

impl Storage {
    pub fn new() -> Self {
        Self {
            users: Arc::new(Mutex::new(HashMap::new())),
            username_to_id: Arc::new(Mutex::new(HashMap::new())),
            connections: Arc::new(Mutex::new(HashMap::new())),
            active_calls: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create storage and load users from file
    pub fn with_persistence() -> Self {
        let storage = Self::new();

        // Load users from file
        match persistence::load_users_from_file() {
            Ok(users) => {
                if let (Ok(mut storage_users), Ok(mut username_map)) =
                    (storage.users.lock(), storage.username_to_id.lock())
                {
                    for (id, user) in users {
                        username_map.insert(user.username.clone(), id.clone());
                        storage_users.insert(id, user);
                    }
                    println!("Loaded {} users from file", storage_users.len());
                }
            }
            Err(e) => {
                eprintln!(" Failed to load users from file: {}", e);
            }
        }

        storage
    }

    // ===== User Management =====

    /// Register a new user
    pub fn create_user(&self, user: User) -> Result<(), String> {
        let mut users = self.users.lock().map_err(|_| "Failed to lock users")?;
        let mut username_map = self
            .username_to_id
            .lock()
            .map_err(|_| "Failed to lock username map")?;

        if username_map.contains_key(&user.username) {
            return Err("Username already exists".to_string());
        }

        username_map.insert(user.username.clone(), user.id.clone());
        let user_id = user.id.clone();
        users.insert(user.id.clone(), user);

        // Persist user to file
        if let Some(user) = users.get(&user_id)
            && let Err(e) = persistence::save_user_to_file(user)
        {
            eprintln!("Failed to persist user: {}", e);
        }

        Ok(())
    }

    /// Get user by ID
    pub fn get_user(&self, user_id: &UserId) -> Option<User> {
        self.users.lock().ok()?.get(user_id).cloned()
    }

    /// Get user by username
    pub fn get_user_by_username(&self, username: &str) -> Option<User> {
        let username_map = self.username_to_id.lock().ok()?;
        let user_id = username_map.get(username)?;
        self.get_user(user_id)
    }

    /// Get all users
    pub fn get_all_users(&self) -> Vec<User> {
        self.users
            .lock()
            .ok()
            .map(|users| users.values().cloned().collect())
            .unwrap_or_default()
    }

    // ===== Connection Management =====

    /// Connect a user (adds connection and broadcasts state change)
    pub fn connect_user(&self, user_id: UserId, sender: Sender<Message>) -> Result<(), String> {
        // Insert connection
        self.connections
            .lock()
            .map_err(|_| "Failed to lock connections")?
            .insert(user_id.clone(), sender);

        // Broadcast state change
        if let Some(user) = self.get_user(&user_id) {
            self.broadcast_user_state_update(&user_id, &user.username, UserState::Available);
        }

        Ok(())
    }

    /// Disconnect a user (removes connection, cleans up calls, broadcasts state)
    pub fn disconnect_user(&self, user_id: &UserId) -> Result<(), String> {
        // Remove connection
        self.connections
            .lock()
            .map_err(|_| "Failed to lock connections")?
            .remove(user_id);

        // Cleanup any active calls involving this user
        self.cleanup_user_calls(user_id);

        // Broadcast state change
        if let Some(user) = self.get_user(user_id) {
            self.broadcast_user_state_update(user_id, &user.username, UserState::Disconnected);
        }

        Ok(())
    }

    /// Remove all calls involving a specific user
    fn cleanup_user_calls(&self, user_id: &UserId) {
        if let Ok(mut calls) = self.active_calls.lock() {
            calls.retain(|_, call| call.caller_id != *user_id && call.callee_id != *user_id);
        }
    }

    /// Forward message to a specific user
    pub fn forward_to_user(&self, target_id: &UserId, message: Message) -> Result<(), String> {
        let conns = self
            .connections
            .lock()
            .map_err(|_| "Failed to lock connections")?;

        let sender = conns
            .get(target_id)
            .ok_or_else(|| format!("User {} not connected", target_id))?;

        sender
            .send(message)
            .map_err(|e| format!("Failed to send message: {}", e))
    }

    /// Broadcast user state update to all connected users
    fn broadcast_user_state_update(&self, user_id: &UserId, username: &str, state: UserState) {
        let update = Message::UserStateUpdate(UserStateUpdateMsg {
            user_id: user_id.clone(),
            username: username.to_string(),
            state: state.to_string(),
        });

        if let Ok(conns) = self.connections.lock() {
            for (uid, sender) in conns.iter() {
                if uid != user_id {
                    let _ = sender.send(update.clone());
                }
            }
        }
    }

    /// Check if user is currently connected
    pub fn is_user_connected(&self, user_id: &UserId) -> bool {
        self.connections
            .lock()
            .ok()
            .map(|conns| conns.contains_key(user_id))
            .unwrap_or(false)
    }

    // ===== State Derivation =====

    /// Get user state (derived from connections and active calls)
    pub fn get_user_state(&self, user_id: &UserId) -> Option<UserState> {
        // Check if user is connected
        if !self.is_user_connected(user_id) {
            return Some(UserState::Disconnected);
        }

        // Check if user is in an active call
        if self.is_user_in_active_call(user_id) {
            return Some(UserState::Busy);
        }

        Some(UserState::Available)
    }

    /// Check if user is in an active call
    fn is_user_in_active_call(&self, user_id: &UserId) -> bool {
        self.active_calls
            .lock()
            .ok()
            .map(|calls| {
                calls.values().any(|call| {
                    (call.caller_id == *user_id || call.callee_id == *user_id)
                        && call.state == CallState::Active
                })
            })
            .unwrap_or(false)
    }

    /// Check if user is currently logged in (connected)
    pub fn is_user_logged_in(&self, user_id: &UserId) -> bool {
        self.is_user_connected(user_id)
    }

    // ===== Call Management =====

    /// Create a new call
    pub fn create_call(&self, caller_id: UserId, callee_id: UserId) -> Result<Call, String> {
        let call = Call::new(caller_id, callee_id);

        let mut calls = self
            .active_calls
            .lock()
            .map_err(|_| "Failed to lock calls")?;

        calls.insert(call.call_id.clone(), call.clone());
        Ok(call)
    }

    /// Get a call by ID
    pub fn get_call(&self, call_id: &str) -> Option<Call> {
        self.active_calls.lock().ok()?.get(call_id).cloned()
    }

    /// Update call state
    pub fn update_call_state(&self, call_id: &str, state: CallState) -> Result<(), String> {
        let mut calls = self
            .active_calls
            .lock()
            .map_err(|_| "Failed to lock calls")?;

        if let Some(call) = calls.get_mut(call_id) {
            call.state = state;
            Ok(())
        } else {
            Err("Call not found".to_string())
        }
    }

    /// Remove a call
    pub fn remove_call(&self, call_id: &str) -> Option<Call> {
        self.active_calls.lock().ok()?.remove(call_id)
    }

    /// Get active call for a user
    pub fn get_user_active_call(&self, user_id: &UserId) -> Option<Call> {
        self.active_calls
            .lock()
            .ok()?
            .values()
            .find(|call| call.caller_id == *user_id || call.callee_id == *user_id)
            .cloned()
    }

    /// Broadcast user state update to all connected users except the user themselves
    pub fn broadcast_state_update(&self, user_id: &UserId, state: UserState) {
        if let Some(user) = self.get_user(user_id) {
            self.broadcast_user_state_update(user_id, &user.username, state);
        }
    }
}

impl Default for Storage {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    #[test]
    fn test_storage_creation() {
        let storage = Storage::new();
        let users = storage.get_all_users();
        assert_eq!(users.len(), 0);
    }

    #[test]
    fn test_create_user_success() {
        let storage = Storage::new();
        let user = User::new("user1".to_string(), "alice".to_string(), "password");

        let result = storage.create_user(user.clone());
        assert!(result.is_ok());

        let retrieved = storage.get_user(&"user1".to_string());
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().username, "alice");
    }

    #[test]
    fn test_create_user_duplicate_username() {
        let storage = Storage::new();
        let user1 = User::new("user1".to_string(), "bob".to_string(), "pass1");
        let user2 = User::new("user2".to_string(), "bob".to_string(), "pass2");

        storage
            .create_user(user1)
            .expect("First user should succeed");
        let result = storage.create_user(user2);

        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Username already exists"));
    }

    #[test]
    fn test_get_user_by_username() {
        let storage = Storage::new();
        let user = User::new("user123".to_string(), "charlie".to_string(), "secret");

        storage.create_user(user).unwrap();

        let retrieved = storage.get_user_by_username("charlie");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().id, "user123");
    }

    #[test]
    fn test_get_user_by_username_not_found() {
        let storage = Storage::new();
        let not_found = storage.get_user_by_username("nonexistent");
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_all_users() {
        let storage = Storage::new();

        let user1 = User::new("u1".to_string(), "alice".to_string(), "pass");
        let user2 = User::new("u2".to_string(), "bob".to_string(), "pass");

        storage.create_user(user1).unwrap();
        storage.create_user(user2).unwrap();

        let users = storage.get_all_users();
        assert_eq!(users.len(), 2);
    }

    #[test]
    fn test_connect_user() {
        let storage = Storage::new();
        let user = User::new("user1".to_string(), "dave".to_string(), "password");
        storage.create_user(user).unwrap();

        let (tx, _rx) = mpsc::channel();

        let result = storage.connect_user("user1".to_string(), tx);
        assert!(result.is_ok());
        assert!(storage.is_user_connected(&"user1".to_string()));
    }

    #[test]
    fn test_disconnect_user() {
        let storage = Storage::new();
        let user = User::new("user1".to_string(), "eve".to_string(), "password");
        storage.create_user(user).unwrap();

        let (tx, _rx) = mpsc::channel();
        storage.connect_user("user1".to_string(), tx).unwrap();

        assert!(storage.is_user_connected(&"user1".to_string()));

        storage.disconnect_user(&"user1".to_string()).unwrap();
        assert!(!storage.is_user_connected(&"user1".to_string()));
    }

    #[test]
    fn test_user_state_transitions() {
        let storage = Storage::new();
        let user = User::new("user1".to_string(), "frank".to_string(), "password");
        storage.create_user(user).unwrap();

        // Initially disconnected
        let state = storage.get_user_state(&"user1".to_string());
        assert_eq!(state, Some(UserState::Disconnected));

        // Connect user
        let (tx, _rx) = mpsc::channel();
        storage.connect_user("user1".to_string(), tx).unwrap();

        // Should be available
        let state = storage.get_user_state(&"user1".to_string());
        assert_eq!(state, Some(UserState::Available));
    }

    #[test]
    fn test_create_call() {
        let storage = Storage::new();

        let result = storage.create_call("caller1".to_string(), "callee1".to_string());
        assert!(result.is_ok());

        let call = result.unwrap();
        assert_eq!(call.caller_id, "caller1");
        assert_eq!(call.callee_id, "callee1");
        assert_eq!(call.state, CallState::Ringing);
        assert!(call.call_id.starts_with("call_"));
    }

    #[test]
    fn test_get_call() {
        let storage = Storage::new();

        let call = storage
            .create_call("caller1".to_string(), "callee1".to_string())
            .unwrap();

        let call_id = call.call_id.clone();
        let retrieved = storage.get_call(&call_id);

        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().call_id, call_id);
    }

    #[test]
    fn test_update_call_state() {
        let storage = Storage::new();

        let call = storage
            .create_call("caller1".to_string(), "callee1".to_string())
            .unwrap();

        let call_id = call.call_id.clone();

        // Update to Active
        let result = storage.update_call_state(&call_id, CallState::Active);
        assert!(result.is_ok());

        let updated = storage.get_call(&call_id).unwrap();
        assert_eq!(updated.state, CallState::Active);
    }

    #[test]
    fn test_remove_call() {
        let storage = Storage::new();

        let call = storage
            .create_call("caller1".to_string(), "callee1".to_string())
            .unwrap();

        let call_id = call.call_id.clone();

        let removed = storage.remove_call(&call_id);
        assert!(removed.is_some());

        let not_found = storage.get_call(&call_id);
        assert!(not_found.is_none());
    }

    #[test]
    fn test_get_user_active_call() {
        let storage = Storage::new();

        let call = storage
            .create_call("caller1".to_string(), "callee1".to_string())
            .unwrap();

        let call_id = call.call_id.clone();

        // Caller should have active call
        let caller_call = storage.get_user_active_call(&"caller1".to_string());
        assert!(caller_call.is_some());
        assert_eq!(caller_call.unwrap().call_id, call_id);

        // Callee should have active call
        let callee_call = storage.get_user_active_call(&"callee1".to_string());
        assert!(callee_call.is_some());

        // Unrelated user should not have active call
        let other_call = storage.get_user_active_call(&"other_user".to_string());
        assert!(other_call.is_none());
    }

    #[test]
    fn test_user_state_busy_when_in_call() {
        let storage = Storage::new();

        let user1 = User::new("user1".to_string(), "alice".to_string(), "pass");
        let user2 = User::new("user2".to_string(), "bob".to_string(), "pass");

        storage.create_user(user1).unwrap();
        storage.create_user(user2).unwrap();

        let (tx1, _rx1) = mpsc::channel();
        let (tx2, _rx2) = mpsc::channel();

        storage.connect_user("user1".to_string(), tx1).unwrap();
        storage.connect_user("user2".to_string(), tx2).unwrap();

        // Create and activate call
        let call = storage
            .create_call("user1".to_string(), "user2".to_string())
            .unwrap();
        storage
            .update_call_state(&call.call_id, CallState::Active)
            .unwrap();

        // Both users should be busy
        assert_eq!(
            storage.get_user_state(&"user1".to_string()),
            Some(UserState::Busy)
        );
        assert_eq!(
            storage.get_user_state(&"user2".to_string()),
            Some(UserState::Busy)
        );
    }

    #[test]
    fn test_cleanup_user_calls_on_disconnect() {
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

        // Disconnect user1
        storage.disconnect_user(&"user1".to_string()).unwrap();

        // Call should be removed
        assert!(storage.get_call(&call_id).is_none());
    }
}
