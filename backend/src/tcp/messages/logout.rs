use super::json_helpers::{insert_bool, insert_optional_string};
use json_parser::JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LogoutRequest;

impl LogoutRequest {
    pub fn from_json(_json: &JsonValue) -> Result<Self, String> {
        Ok(LogoutRequest)
    }
}

#[derive(Debug, Clone)]
pub struct LogoutResponse {
    pub success: bool,
    pub error: Option<String>,
}

impl LogoutResponse {
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        insert_bool(&mut map, "success", self.success);
        insert_optional_string(&mut map, "error", &self.error);
        JsonValue::Object(map)
    }
}
