//! Data models for the application
//!
//! This module contains all the data structures used throughout the app.
//! Models are pure data structures that can be serialized/deserialized.

mod participant;
pub mod protocol;
mod room;

pub use participant::{Participant, ParticipantRole};
pub use room::RoomData;
