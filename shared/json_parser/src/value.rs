//! JSON value representation and manipulation.

use std::collections::HashMap;
use std::fmt;

/// Represents a JSON value that can be an object, array, string, number, boolean, or null.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    /// JSON object: `{"key": "value"}`
    Object(HashMap<String, JsonValue>),
    /// JSON array: `["item1", "item2"]`
    Array(Vec<JsonValue>),
    /// JSON string: `"hello"`
    String(String),
    /// JSON number: `42` or `3.14`
    Number(f64),
    /// JSON boolean: `true` or `false`
    Bool(bool),
    /// JSON null: `null`
    Null,
}

impl fmt::Display for JsonValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_json_string())
    }
}

impl JsonValue {
    fn to_json_string(&self) -> String {
        match self {
            JsonValue::String(s) => format!("\"{}\"", crate::serializer::escape_json_string(s)),
            JsonValue::Number(n) => {
                if n.fract() == 0.0 && *n >= 0.0 && *n <= u64::MAX as f64 {
                    format!("{:.0}", n)
                } else {
                    n.to_string()
                }
            }
            JsonValue::Bool(b) => b.to_string(),
            JsonValue::Null => "null".to_string(),
            JsonValue::Object(map) => {
                let pairs: Vec<String> = map
                    .iter()
                    .map(|(k, v)| {
                        format!(
                            "\"{}\":{}",
                            crate::serializer::escape_json_string(k),
                            v.to_json_string()
                        )
                    })
                    .collect();
                format!("{{{}}}", pairs.join(","))
            }
            JsonValue::Array(arr) => {
                let items: Vec<String> = arr.iter().map(|v| v.to_json_string()).collect();
                format!("[{}]", items.join(","))
            }
        }
    }

    /// Returns the string value if this is a JSON string, otherwise None.
    pub fn as_string(&self) -> Option<&str> {
        if let JsonValue::String(s) = self {
            Some(s.as_str())
        } else {
            None
        }
    }

    /// Returns the number value if this is a JSON number, otherwise None.
    pub fn as_number(&self) -> Option<f64> {
        if let JsonValue::Number(n) = self {
            Some(*n)
        } else {
            None
        }
    }

    /// Returns the boolean value if this is a JSON boolean, otherwise None.
    pub fn as_bool(&self) -> Option<bool> {
        if let JsonValue::Bool(b) = self {
            Some(*b)
        } else {
            None
        }
    }

    /// Returns true if this is JSON null.
    pub fn is_null(&self) -> bool {
        matches!(self, JsonValue::Null)
    }

    /// Returns the object value if this is a JSON object, otherwise None.
    pub fn as_object(&self) -> Option<&HashMap<String, JsonValue>> {
        if let JsonValue::Object(map) = self {
            Some(map)
        } else {
            None
        }
    }

    /// Returns the mutable object value if this is a JSON object, otherwise None.
    pub fn as_object_mut(&mut self) -> Option<&mut HashMap<String, JsonValue>> {
        if let JsonValue::Object(map) = self {
            Some(map)
        } else {
            None
        }
    }

    /// Returns the array value if this is a JSON array, otherwise None.
    pub fn as_array(&self) -> Option<&Vec<JsonValue>> {
        if let JsonValue::Array(arr) = self {
            Some(arr)
        } else {
            None
        }
    }

    /// Returns the mutable array value if this is a JSON array, otherwise None.
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<JsonValue>> {
        if let JsonValue::Array(arr) = self {
            Some(arr)
        } else {
            None
        }
    }

    /// Gets a value from a JSON object by key path (dot notation).
    ///
    /// # Examples
    ///
    /// ```
    /// use json_parser::parse_json;
    ///
    /// let json_str = r#"{"user":{"name":"Alice","profile":{"age":30}}}"#;
    /// let json = parse_json(json_str).unwrap();
    ///
    /// assert_eq!(json.get_path("user.name").and_then(|v| v.as_string()), Some("Alice"));
    /// assert_eq!(json.get_path("user.profile.age").and_then(|v| v.as_number()), Some(30.0));
    /// ```
    pub fn get_path(&self, path: &str) -> Option<&JsonValue> {
        let mut current = self;
        for key in path.split('.') {
            match current.as_object()?.get(key) {
                Some(value) => current = value,
                None => return None,
            }
        }
        Some(current)
    }

    /// Gets a mutable reference to a value in a JSON object by key path.
    pub fn get_path_mut(&mut self, path: &str) -> Option<&mut JsonValue> {
        let mut current = self;
        let keys: Vec<&str> = path.split('.').collect();

        for (i, key) in keys.iter().enumerate() {
            if i == keys.len() - 1 {
                return current.as_object_mut()?.get_mut(*key);
            } else {
                current = current.as_object_mut()?.get_mut(*key)?;
            }
        }
        None
    }

    /// Sets a value in a JSON object by key path, creating intermediate objects if needed.
    ///
    /// # Examples
    ///
    /// ```
    /// use json_parser::JsonValue;
    ///
    /// let mut json = JsonValue::Object(std::collections::HashMap::new());
    /// json.set_path("user.name", JsonValue::String("Bob".to_string()));
    /// json.set_path("user.profile.age", JsonValue::Number(25.0));
    ///
    /// assert_eq!(json.get_path("user.name").and_then(|v| v.as_string()), Some("Bob"));
    /// ```
    pub fn set_path(&mut self, path: &str, value: JsonValue) {
        let keys: Vec<&str> = path.split('.').collect();
        let mut current = self;

        for (i, key) in keys.iter().enumerate() {
            if i == keys.len() - 1 {
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(key.to_string(), value);
                }
                return;
            } else {
                if !matches!(current, JsonValue::Object(_)) {
                    *current = JsonValue::Object(HashMap::new());
                }
                let obj = current.as_object_mut().unwrap();
                obj.entry(key.to_string())
                    .or_insert_with(|| JsonValue::Object(HashMap::new()));
                current = obj.get_mut(*key).unwrap();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_as_string() {
        let value = JsonValue::String("hello".to_string());
        assert_eq!(value.as_string(), Some("hello"));
        assert_eq!(JsonValue::Number(42.0).as_string(), None);
    }

    #[test]
    fn test_as_number() {
        let value = JsonValue::Number(42.0);
        assert_eq!(value.as_number(), Some(42.0));
        assert_eq!(JsonValue::String("hello".to_string()).as_number(), None);
    }

    #[test]
    fn test_as_bool() {
        let value = JsonValue::Bool(true);
        assert_eq!(value.as_bool(), Some(true));
        assert_eq!(JsonValue::Null.as_bool(), None);
    }

    #[test]
    fn test_is_null() {
        assert!(JsonValue::Null.is_null());
        assert!(!JsonValue::String("hello".to_string()).is_null());
    }

    #[test]
    fn test_as_object() {
        let mut map = HashMap::new();
        map.insert("key".to_string(), JsonValue::String("value".to_string()));
        let obj = JsonValue::Object(map);

        assert!(obj.as_object().is_some());
        assert_eq!(JsonValue::Array(vec![]).as_object(), None);
    }

    #[test]
    fn test_as_array() {
        let arr = JsonValue::Array(vec![JsonValue::Number(1.0)]);
        assert!(arr.as_array().is_some());
        assert_eq!(JsonValue::Object(HashMap::new()).as_array(), None);
    }

    #[test]
    fn test_get_path() {
        let mut map = HashMap::new();
        let mut nested = HashMap::new();
        nested.insert("age".to_string(), JsonValue::Number(30.0));
        map.insert("user".to_string(), JsonValue::Object(nested));

        let json = JsonValue::Object(map);

        assert_eq!(
            json.get_path("user.age").and_then(|v| v.as_number()),
            Some(30.0)
        );
        assert!(json.get_path("nonexistent").is_none());
    }

    #[test]
    fn test_set_path() {
        let mut json = JsonValue::Object(HashMap::new());
        json.set_path("user.name", JsonValue::String("Alice".to_string()));
        json.set_path("user.profile.age", JsonValue::Number(25.0));

        assert_eq!(
            json.get_path("user.name").and_then(|v| v.as_string()),
            Some("Alice")
        );
        assert_eq!(
            json.get_path("user.profile.age")
                .and_then(|v| v.as_number()),
            Some(25.0)
        );
    }
}
