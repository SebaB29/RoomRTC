use json_parser::JsonValue;
use std::collections::HashMap;

pub fn get_string_field(obj: &HashMap<String, JsonValue>, field: &str) -> Result<String, String> {
    obj.get(field)
        .and_then(|v| v.as_string())
        .ok_or_else(|| format!("Missing {}", field))
        .map(|s| s.to_string())
}

pub fn get_bool_field(obj: &HashMap<String, JsonValue>, field: &str) -> Result<bool, String> {
    obj.get(field)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| format!("Missing {}", field))
}

pub fn get_number_field(obj: &HashMap<String, JsonValue>, field: &str) -> Result<f64, String> {
    obj.get(field)
        .and_then(|v| {
            if let JsonValue::Number(n) = v {
                Some(*n)
            } else {
                None
            }
        })
        .ok_or_else(|| format!("Missing {}", field))
}

pub fn insert_string(map: &mut HashMap<String, JsonValue>, key: &str, value: String) {
    map.insert(key.to_string(), JsonValue::String(value));
}

pub fn insert_bool(map: &mut HashMap<String, JsonValue>, key: &str, value: bool) {
    map.insert(key.to_string(), JsonValue::Bool(value));
}

pub fn insert_number(map: &mut HashMap<String, JsonValue>, key: &str, value: f64) {
    map.insert(key.to_string(), JsonValue::Number(value));
}

pub fn insert_optional_string(
    map: &mut HashMap<String, JsonValue>,
    key: &str,
    value: &Option<String>,
) {
    if let Some(val) = value {
        map.insert(key.to_string(), JsonValue::String(val.clone()));
    }
}
