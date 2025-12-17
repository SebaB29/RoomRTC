use super::json_helpers::{get_bool_field, get_string_field, insert_string};
use json_parser::JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CallRequest {
    pub to_user_id: String,
}

impl CallRequest {
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("Expected object")?;
        let to_user_id = get_string_field(obj, "to_user_id")?;
        Ok(CallRequest { to_user_id })
    }
}

#[derive(Debug, Clone)]
pub struct CallResponseMsg {
    pub call_id: String,
    pub accepted: bool,
}

impl CallResponseMsg {
    pub fn from_json(json: &JsonValue) -> Result<Self, String> {
        let obj = json.as_object().ok_or("Expected object")?;
        let call_id = get_string_field(obj, "call_id")?;
        let accepted = get_bool_field(obj, "accepted")?;
        Ok(CallResponseMsg { call_id, accepted })
    }
}

#[derive(Debug, Clone)]
pub struct CallNotificationMsg {
    pub call_id: String,
    pub from_user_id: String,
    pub from_username: String,
}

impl CallNotificationMsg {
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        insert_string(&mut map, "call_id", self.call_id.clone());
        insert_string(&mut map, "from_user_id", self.from_user_id.clone());
        insert_string(&mut map, "from_username", self.from_username.clone());
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone)]
pub struct CallAcceptedMsg {
    pub call_id: String,
    pub peer_user_id: String,
    pub peer_username: String,
}

impl CallAcceptedMsg {
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        insert_string(&mut map, "call_id", self.call_id.clone());
        insert_string(&mut map, "peer_user_id", self.peer_user_id.clone());
        insert_string(&mut map, "peer_username", self.peer_username.clone());
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone)]
pub struct CallDeclinedMsg {
    pub call_id: String,
    pub peer_user_id: String,
    pub peer_username: String,
}

impl CallDeclinedMsg {
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        insert_string(&mut map, "call_id", self.call_id.clone());
        insert_string(&mut map, "peer_user_id", self.peer_user_id.clone());
        insert_string(&mut map, "peer_username", self.peer_username.clone());
        JsonValue::Object(map)
    }
}
