use super::json_helpers::insert_string;
use json_parser::JsonValue;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct UserListResponse {
    pub users: Vec<UserInfoMsg>,
}

impl UserListResponse {
    pub fn to_json(&self) -> JsonValue {
        let users_array: Vec<JsonValue> = self
            .users
            .iter()
            .map(|u| {
                let mut map = HashMap::new();
                insert_string(&mut map, "user_id", u.user_id.clone());
                insert_string(&mut map, "username", u.username.clone());
                insert_string(&mut map, "state", u.state.clone());
                JsonValue::Object(map)
            })
            .collect();

        let mut map = HashMap::new();
        map.insert("users".to_string(), JsonValue::Array(users_array));
        JsonValue::Object(map)
    }
}

#[derive(Debug, Clone)]
pub struct UserInfoMsg {
    pub user_id: String,
    pub username: String,
    pub state: String,
}

#[derive(Debug, Clone)]
pub struct UserStateUpdateMsg {
    pub user_id: String,
    pub username: String,
    pub state: String,
}

impl UserStateUpdateMsg {
    pub fn to_json(&self) -> JsonValue {
        let mut map = HashMap::new();
        insert_string(&mut map, "user_id", self.user_id.clone());
        insert_string(&mut map, "username", self.username.clone());
        insert_string(&mut map, "state", self.state.clone());
        JsonValue::Object(map)
    }
}
