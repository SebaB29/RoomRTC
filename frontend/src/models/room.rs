//! Room Data Model
//!
//! Defines the room data structure for video call sessions.
//! Rooms support a maximum of 2 participants: one Owner and one Guest.

use super::participant::{Participant, ParticipantRole};

// Room configuration constants
const MAX_PARTICIPANTS: usize = 2;
const ROOM_ID_LENGTH: usize = 8;

/// Room data structure (max 2 participants: owner + guest)
#[derive(Clone, Debug)]
pub struct RoomData {
    pub id: String,
    pub participants: Vec<Participant>,
    pub stats: Option<crate::components::CallStats>,
}

// Manual implementation to handle the stats field which is not serialized
impl json_parser::Serialize for RoomData {
    fn serialize(&self) -> json_parser::JsonValue {
        let mut map = std::collections::HashMap::new();
        map.insert(
            "id".to_string(),
            json_parser::Serialize::serialize(&self.id),
        );
        map.insert(
            "participants".to_string(),
            json_parser::Serialize::serialize(&self.participants),
        );
        json_parser::JsonValue::Object(map)
    }
}

impl json_parser::Deserialize for RoomData {
    fn deserialize(value: &json_parser::JsonValue) -> Result<Self, json_parser::JsonError> {
        let obj = value.as_object().ok_or_else(|| {
            json_parser::JsonError::TypeMismatch("Expected object for RoomData".to_string())
        })?;

        let id = obj
            .get("id")
            .ok_or_else(|| json_parser::JsonError::MissingField("id".to_string()))?;
        let participants = obj
            .get("participants")
            .ok_or_else(|| json_parser::JsonError::MissingField("participants".to_string()))?;

        Ok(Self {
            id: json_parser::Deserialize::deserialize(id)?,
            participants: json_parser::Deserialize::deserialize(participants)?,
            stats: None, // Runtime field, always starts as None when deserialized
        })
    }
}

impl RoomData {
    /// Creates a new room with a random alphanumeric ID
    pub fn new() -> Self {
        Self {
            id: Self::generate_room_id(),
            participants: Vec::with_capacity(MAX_PARTICIPANTS),
            stats: None,
        }
    }

    /// Creates a new room with a specific ID
    pub fn new_with_id(id: String) -> Self {
        Self {
            id,
            participants: Vec::with_capacity(MAX_PARTICIPANTS),
            stats: None,
        }
    }

    /// Generates a random alphanumeric room ID (kept for compatibility)
    fn generate_room_id() -> String {
        use rand::Rng;
        rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(ROOM_ID_LENGTH)
            .map(char::from)
            .collect()
    }

    /// Adds a participant to the room
    pub fn add_participant(&mut self, name: String) -> Result<ParticipantRole, String> {
        if self.is_full() {
            return Err("Room is full (maximum 2 participants)".to_string());
        }

        let role = if self.is_empty() {
            ParticipantRole::Owner
        } else {
            ParticipantRole::Guest
        };

        self.participants.push(Participant::new(name, role));
        Ok(role)
    }

    /// Removes a participant from the room by name
    pub fn remove_participant(&mut self, name: &str) {
        self.participants.retain(|p| p.name != name);
    }

    /// Gets a reference to a participant by name
    pub fn get_participant(&self, name: &str) -> Option<&Participant> {
        self.participants.iter().find(|p| p.name == name)
    }

    /// Gets a mutable reference to a participant by name
    pub fn get_participant_mut(&mut self, name: &str) -> Option<&mut Participant> {
        self.participants.iter_mut().find(|p| p.name == name)
    }

    /// Checks if the room is empty
    pub fn is_empty(&self) -> bool {
        self.participants.is_empty()
    }

    /// Checks if the room is full (has maximum participants)
    pub fn is_full(&self) -> bool {
        self.participants.len() >= MAX_PARTICIPANTS
    }
}

impl Default for RoomData {
    fn default() -> Self {
        Self::new()
    }
}
