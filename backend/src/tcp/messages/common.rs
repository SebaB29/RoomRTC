use super::json_helpers::{insert_number, insert_string};
use json_parser::JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HeartbeatMsg {}

impl HeartbeatMsg {
    pub fn from_json(_json: &JsonValue) -> Result<Self, String> {
        Ok(HeartbeatMsg {})
    }
}

#[derive(Debug, Clone)]
pub struct ErrorMsg {
    pub code: u16,
    pub message: String,
}

impl ErrorMsg {
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        insert_number(&mut map, "code", self.code as f64);
        insert_string(&mut map, "message", self.message.clone());
        JsonValue::Object(map)
    }
}
