use super::json_helpers::{get_string_field, insert_bool, insert_optional_string};
use json_parser::JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct RegisterRequest {
    pub username: String,
    pub password_hash: String,
}

impl RegisterRequest {
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("Expected object")?;
        let username = get_string_field(obj, "username")?;
        let password_hash = get_string_field(obj, "password_hash")?;
        Ok(RegisterRequest {
            username,
            password_hash,
        })
    }
}

#[derive(Debug, Clone)]
pub struct RegisterResponse {
    pub success: bool,
    pub user_id: Option<String>,
    pub username: Option<String>,
    pub error: Option<String>,
}

impl RegisterResponse {
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        insert_bool(&mut map, "success", self.success);
        insert_optional_string(&mut map, "user_id", &self.user_id);
        insert_optional_string(&mut map, "username", &self.username);
        insert_optional_string(&mut map, "error", &self.error);
        JsonValue::Object(map)
    }
}
