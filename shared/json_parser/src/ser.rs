//! Serialization from Rust types to JSON.

use crate::value::JsonValue;
use std::collections::HashMap;

/// Trait for types that can be serialized to JSON.
pub trait Serialize {
    /// Serializes a value to a JSON value.
    fn serialize(&self) -> JsonValue;

    /// Serializes to a JSON string.
    fn to_string(&self) -> String {
        self.serialize().to_string()
    }

    /// Serializes to a pretty JSON string with indentation.
    fn to_string_pretty(&self) -> String {
        pretty_print(&self.serialize(), 0)
    }
}

// Implement Serialize for primitive types
impl Serialize for String {
    fn serialize(&self) -> JsonValue {
        JsonValue::String(self.clone())
    }
}

impl Serialize for &str {
    fn serialize(&self) -> JsonValue {
        JsonValue::String((*self).to_string())
    }
}

impl Serialize for bool {
    fn serialize(&self) -> JsonValue {
        JsonValue::Bool(*self)
    }
}

impl Serialize for f64 {
    fn serialize(&self) -> JsonValue {
        JsonValue::Number(*self)
    }
}

impl Serialize for f32 {
    fn serialize(&self) -> JsonValue {
        JsonValue::Number(*self as f64)
    }
}

impl Serialize for i32 {
    fn serialize(&self) -> JsonValue {
        JsonValue::Number(*self as f64)
    }
}

impl Serialize for i64 {
    fn serialize(&self) -> JsonValue {
        JsonValue::Number(*self as f64)
    }
}

impl Serialize for u32 {
    fn serialize(&self) -> JsonValue {
        JsonValue::Number(*self as f64)
    }
}

impl Serialize for u64 {
    fn serialize(&self) -> JsonValue {
        JsonValue::Number(*self as f64)
    }
}

impl Serialize for usize {
    fn serialize(&self) -> JsonValue {
        JsonValue::Number(*self as f64)
    }
}

impl<T: Serialize> Serialize for Vec<T> {
    fn serialize(&self) -> JsonValue {
        JsonValue::Array(self.iter().map(|v| v.serialize()).collect())
    }
}

impl<T: Serialize> Serialize for Option<T> {
    fn serialize(&self) -> JsonValue {
        match self {
            Some(v) => v.serialize(),
            None => JsonValue::Null,
        }
    }
}

impl<V: Serialize> Serialize for HashMap<String, V> {
    fn serialize(&self) -> JsonValue {
        JsonValue::Object(
            self.iter()
                .map(|(k, v)| (k.clone(), v.serialize()))
                .collect(),
        )
    }
}

impl<T: Serialize> Serialize for &T {
    fn serialize(&self) -> JsonValue {
        (*self).serialize()
    }
}

/// Helper function to serialize to a JSON string
pub fn to_string<T: Serialize>(value: &T) -> String {
    value.to_string()
}

/// Helper function to serialize to a pretty JSON string
pub fn to_string_pretty<T: Serialize>(value: &T) -> String {
    value.to_string_pretty()
}

/// Internal helper for pretty printing with indentation
fn pretty_print(value: &JsonValue, indent: usize) -> String {
    match value {
        JsonValue::Object(map) => {
            if map.is_empty() {
                return "{}".to_string();
            }
            let mut result = "{\n".to_string();
            let items: Vec<_> = map.iter().collect();
            for (i, (k, v)) in items.iter().enumerate() {
                result.push_str(&"  ".repeat(indent + 1));
                result.push_str(&format!(
                    "\"{}\": {}",
                    crate::serializer::escape_json_string(k),
                    pretty_print(v, indent + 1)
                ));
                if i < items.len() - 1 {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push_str(&"  ".repeat(indent));
            result.push('}');
            result
        }
        JsonValue::Array(arr) => {
            if arr.is_empty() {
                return "[]".to_string();
            }
            let mut result = "[\n".to_string();
            for (i, v) in arr.iter().enumerate() {
                result.push_str(&"  ".repeat(indent + 1));
                result.push_str(&pretty_print(v, indent + 1));
                if i < arr.len() - 1 {
                    result.push(',');
                }
                result.push('\n');
            }
            result.push_str(&"  ".repeat(indent));
            result.push(']');
            result
        }
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_serialize_string() {
        let s = "hello".to_string();
        let json = s.serialize();
        assert_eq!(json.as_string(), Some("hello"));
    }

    #[test]
    fn test_serialize_number() {
        let n = 42i32;
        let json = n.serialize();
        assert_eq!(json.as_number(), Some(42.0));
    }

    #[test]
    fn test_serialize_bool() {
        let b = true;
        let json = b.serialize();
        assert_eq!(json.as_bool(), Some(true));
    }

    #[test]
    fn test_serialize_vec() {
        let v = vec![1, 2, 3];
        let json = v.serialize();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_number(), Some(1.0));
    }

    #[test]
    fn test_serialize_option_some() {
        let opt = Some("hello".to_string());
        let json = opt.serialize();
        assert_eq!(json.as_string(), Some("hello"));
    }

    #[test]
    fn test_serialize_option_none() {
        let opt: Option<String> = None;
        let json = opt.serialize();
        assert!(json.is_null());
    }

    #[test]
    fn test_to_string() {
        let s = "hello".to_string();
        assert_eq!(to_string(&s), r#""hello""#);
    }

    #[test]
    fn test_to_string_pretty() {
        let mut map = HashMap::new();
        map.insert("name".to_string(), "Alice".to_string());
        map.insert("age".to_string(), "30".to_string());
        let pretty = to_string_pretty(&map);
        assert!(pretty.contains("  "));
        assert!(pretty.contains("\n"));
    }

    #[test]
    fn test_serialize_all_numeric_types() {
        assert_eq!(42i32.serialize().as_number(), Some(42.0));
        assert_eq!(42i64.serialize().as_number(), Some(42.0));
        assert_eq!(42u32.serialize().as_number(), Some(42.0));
        assert_eq!(42u64.serialize().as_number(), Some(42.0));
        assert_eq!(42usize.serialize().as_number(), Some(42.0));
        // f32 has precision loss when converted to f64
        let f32_val = PI as f32;
        let f32_val = f32_val.serialize().as_number().unwrap();
        assert!((f32_val - PI).abs() < 0.01);
        assert_eq!(PI.serialize().as_number(), Some(PI));
    }

    #[test]
    fn test_serialize_nested_vecs() {
        let nested = vec![vec![1, 2], vec![3, 4, 5]];
        let json = nested.serialize();
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0].as_array().unwrap().len(), 2);
        assert_eq!(arr[1].as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_serialize_hashmap_complex() {
        let mut map = HashMap::new();
        map.insert("numbers".to_string(), vec![1, 2, 3]);
        map.insert("empty".to_string(), vec![]);

        let json = map.serialize();
        let obj = json.as_object().unwrap();
        assert_eq!(obj.len(), 2);
        assert!(obj.contains_key("numbers"));
        assert!(obj.contains_key("empty"));
    }

    #[test]
    fn test_pretty_print_nested() {
        let mut inner = HashMap::new();
        inner.insert("nested".to_string(), "value".to_string());

        let mut outer = HashMap::new();
        outer.insert("inner".to_string(), inner);

        let pretty = to_string_pretty(&outer);
        assert!(pretty.contains("    ")); // Double indentation
        assert!(pretty.lines().count() > 3);
    }
}
