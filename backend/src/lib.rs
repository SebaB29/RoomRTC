//! RoomRTC Server Library
//!
//! Core library exposing domain models and infrastructure for integration testing.

pub mod application;
pub mod config;
pub mod domain;
pub mod infrastructure;
pub mod tcp;

// Re-export commonly used types for integration tests
pub use domain::{Call, CallState, User, UserState};
pub use infrastructure::{persistence, storage::Storage};
