//! Deserialization from JSON to Rust types.

use crate::error::JsonError;
use crate::value::JsonValue;
use std::collections::HashMap;

/// Trait for types that can be deserialized from JSON.
pub trait Deserialize: Sized {
    /// Deserializes a value from a JSON value.
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError>;

    /// Deserializes from a JSON string.
    fn from_str(s: &str) -> Result<Self, JsonError> {
        let value = crate::parse_json(s)?;
        Self::deserialize(&value)
    }
}

// Implement Deserialize for primitive types
impl Deserialize for String {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        value
            .as_string()
            .map(|s| s.to_string())
            .ok_or_else(|| JsonError::TypeMismatch("Expected string".to_string()))
    }
}

impl Deserialize for bool {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        value
            .as_bool()
            .ok_or_else(|| JsonError::TypeMismatch("Expected boolean".to_string()))
    }
}

impl Deserialize for f64 {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        value
            .as_number()
            .ok_or_else(|| JsonError::TypeMismatch("Expected number".to_string()))
    }
}

impl Deserialize for i32 {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        let num = value
            .as_number()
            .ok_or_else(|| JsonError::TypeMismatch("Expected number".to_string()))?;
        Ok(num as i32)
    }
}

impl Deserialize for i64 {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        let num = value
            .as_number()
            .ok_or_else(|| JsonError::TypeMismatch("Expected number".to_string()))?;
        Ok(num as i64)
    }
}

impl Deserialize for u32 {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        let num = value
            .as_number()
            .ok_or_else(|| JsonError::TypeMismatch("Expected number".to_string()))?;
        Ok(num as u32)
    }
}

impl Deserialize for u64 {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        let num = value
            .as_number()
            .ok_or_else(|| JsonError::TypeMismatch("Expected number".to_string()))?;
        Ok(num as u64)
    }
}

impl Deserialize for usize {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        let num = value
            .as_number()
            .ok_or_else(|| JsonError::TypeMismatch("Expected number".to_string()))?;
        Ok(num as usize)
    }
}

impl<T: Deserialize> Deserialize for Vec<T> {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        let arr = value
            .as_array()
            .ok_or_else(|| JsonError::TypeMismatch("Expected array".to_string()))?;

        arr.iter().map(|v| T::deserialize(v)).collect()
    }
}

impl<T: Deserialize> Deserialize for Option<T> {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        if value.is_null() {
            Ok(None)
        } else {
            Ok(Some(T::deserialize(value)?))
        }
    }
}

impl<V: Deserialize> Deserialize for HashMap<String, V> {
    fn deserialize(value: &JsonValue) -> Result<Self, JsonError> {
        let obj = value
            .as_object()
            .ok_or_else(|| JsonError::TypeMismatch("Expected object".to_string()))?;

        obj.iter()
            .map(|(k, v)| Ok((k.clone(), V::deserialize(v)?)))
            .collect()
    }
}

/// Helper function to deserialize from a JSON string
pub fn from_str<T: Deserialize>(s: &str) -> Result<T, JsonError> {
    T::from_str(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_string() {
        let json = JsonValue::String("hello".to_string());
        let result: String = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_deserialize_number() {
        let json = JsonValue::Number(42.0);
        let result: i32 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn test_deserialize_bool() {
        let json = JsonValue::Bool(true);
        let result: bool = Deserialize::deserialize(&json).unwrap();
        assert!(result);
    }

    #[test]
    fn test_deserialize_vec() {
        let json = JsonValue::Array(vec![
            JsonValue::Number(1.0),
            JsonValue::Number(2.0),
            JsonValue::Number(3.0),
        ]);
        let result: Vec<i32> = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn test_deserialize_option_some() {
        let json = JsonValue::String("hello".to_string());
        let result: Option<String> = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, Some("hello".to_string()));
    }

    #[test]
    fn test_deserialize_option_none() {
        let json = JsonValue::Null;
        let result: Option<String> = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn test_from_str() {
        let json_str = r#""hello""#;
        let result: String = from_str(json_str).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_deserialize_all_numeric_types() {
        // i32, i64, u32, u64, usize, f64
        let json = JsonValue::Number(42.0);
        let r1: i32 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r1, 42i32);
        let r2: i64 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r2, 42i64);
        let r3: u32 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r3, 42u32);
        let r4: u64 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r4, 42u64);
        let r5: usize = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r5, 42usize);
        let r6: f64 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r6, 42.0f64);
    }

    #[test]
    fn test_deserialize_negative_numbers() {
        let json = JsonValue::Number(-42.5);
        let r1: i32 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r1, -42i32);
        let r2: f64 = Deserialize::deserialize(&json).unwrap();
        assert_eq!(r2, -42.5f64);
    }

    #[test]
    fn test_deserialize_nested_vecs() {
        let json = JsonValue::Array(vec![
            JsonValue::Array(vec![JsonValue::Number(1.0), JsonValue::Number(2.0)]),
            JsonValue::Array(vec![JsonValue::Number(3.0), JsonValue::Number(4.0)]),
        ]);
        let result: Vec<Vec<i32>> = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, vec![vec![1, 2], vec![3, 4]]);
    }

    #[test]
    fn test_deserialize_empty_collections() {
        let json = JsonValue::Array(vec![]);
        let result: Vec<i32> = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, Vec::<i32>::new());

        let json_map = JsonValue::Object(HashMap::new());
        let result: HashMap<String, i32> = Deserialize::deserialize(&json_map).unwrap();
        assert_eq!(result, HashMap::new());
    }

    #[test]
    fn test_deserialize_mixed_types_in_option() {
        // Option with string
        let json = JsonValue::String("test".to_string());
        let result: Option<String> = Deserialize::deserialize(&json).unwrap();
        assert_eq!(result, Some("test".to_string()));

        // Option with number
        let json_num = JsonValue::Number(99.0);
        let result_num: Option<i32> = Deserialize::deserialize(&json_num).unwrap();
        assert_eq!(result_num, Some(99));
    }

    #[test]
    fn test_deserialize_type_mismatch_errors() {
        // String to number should fail
        let json = JsonValue::String("not a number".to_string());
        let result: Result<i32, _> = Deserialize::deserialize(&json);
        assert!(result.is_err());

        // Number to bool should fail
        let json_num = JsonValue::Number(42.0);
        let result: Result<bool, _> = Deserialize::deserialize(&json_num);
        assert!(result.is_err());

        // Object to string should fail
        let json_obj = JsonValue::Object(HashMap::new());
        let result: Result<String, _> = Deserialize::deserialize(&json_obj);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_str_with_whitespace() {
        let json_str = r#"  "hello world"  "#;
        let result: String = from_str(json_str).unwrap();
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_from_str_malformed_json() {
        let json_str = r#"{ "incomplete": "#;
        let result: Result<String, _> = from_str(json_str);
        assert!(result.is_err());
    }
}
