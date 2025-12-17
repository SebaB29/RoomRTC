//! Integration tests for json_parser
//!
//! Tests the complete serialize/deserialize cycle with complex types

use json_parser::{from_str, impl_json, impl_json_enum, to_string_pretty};
use std::collections::HashMap;

#[derive(Debug, PartialEq)]
enum Role {
    Admin,
    User,
    Guest,
}

impl_json_enum! {
    Role {
        Admin,
        User,
        Guest,
    }
}

#[derive(Debug, PartialEq)]
struct Person {
    name: String,
    age: i32,
    email: Option<String>,
    role: Role,
    scores: Vec<i32>,
}

impl Default for Person {
    fn default() -> Self {
        Person {
            name: String::new(),
            age: 0,
            email: None,
            role: Role::Guest,
            scores: Vec::new(),
        }
    }
}

impl_json! {
    Person {
        name: String,
        age: i32,
        email: Option<String>,
        role: Role,
        scores: Vec<i32>,
    }
}

#[test]
fn test_struct_roundtrip() {
    let person = Person {
        name: "Alice".to_string(),
        age: 30,
        email: Some("alice@example.com".to_string()),
        role: Role::Admin,
        scores: vec![85, 92, 78],
    };

    let json = to_string_pretty(&person);
    let parsed: Person = from_str(&json).unwrap();

    assert_eq!(parsed.name, "Alice");
    assert_eq!(parsed.age, 30);
    assert_eq!(parsed.email, Some("alice@example.com".to_string()));
    assert_eq!(parsed.role, Role::Admin);
    assert_eq!(parsed.scores, vec![85, 92, 78]);
}

#[test]
fn test_struct_with_none() {
    let person = Person {
        name: "Bob".to_string(),
        age: 25,
        email: None,
        role: Role::User,
        scores: vec![],
    };

    let json = to_string_pretty(&person);
    let parsed: Person = from_str(&json).unwrap();

    assert_eq!(parsed.name, "Bob");
    assert_eq!(parsed.email, None);
    assert_eq!(parsed.scores.len(), 0);
}

#[test]
fn test_hashmap_roundtrip() {
    let mut data: HashMap<String, Person> = HashMap::new();
    data.insert(
        "user1".to_string(),
        Person {
            name: "Alice".to_string(),
            age: 30,
            email: Some("alice@test.com".to_string()),
            role: Role::Admin,
            scores: vec![100],
        },
    );
    data.insert(
        "user2".to_string(),
        Person {
            name: "Bob".to_string(),
            age: 25,
            email: None,
            role: Role::Guest,
            scores: vec![50, 60],
        },
    );

    let json = to_string_pretty(&data);
    let parsed: HashMap<String, Person> = from_str(&json).unwrap();

    assert_eq!(parsed.len(), 2);
    assert!(parsed.contains_key("user1"));
    assert!(parsed.contains_key("user2"));
    assert_eq!(parsed["user1"].name, "Alice");
    assert_eq!(parsed["user2"].name, "Bob");
}

#[test]
fn test_vec_of_structs() {
    let people = vec![
        Person {
            name: "Alice".to_string(),
            age: 30,
            email: Some("alice@test.com".to_string()),
            role: Role::Admin,
            scores: vec![95],
        },
        Person {
            name: "Bob".to_string(),
            age: 25,
            email: None,
            role: Role::User,
            scores: vec![80, 85],
        },
    ];

    let json = to_string_pretty(&people);
    let parsed: Vec<Person> = from_str(&json).unwrap();

    assert_eq!(parsed.len(), 2);
    assert_eq!(parsed[0].name, "Alice");
    assert_eq!(parsed[1].name, "Bob");
    assert_eq!(parsed[1].email, None);
}

#[test]
fn test_enum_serialization() {
    let roles = vec![Role::Admin, Role::User, Role::Guest];
    let json = to_string_pretty(&roles);
    let parsed: Vec<Role> = from_str(&json).unwrap();

    assert_eq!(parsed.len(), 3);
    assert_eq!(parsed[0], Role::Admin);
    assert_eq!(parsed[1], Role::User);
    assert_eq!(parsed[2], Role::Guest);
}

#[test]
fn test_nested_options() {
    let data: Option<Option<String>> = Some(Some("nested".to_string()));
    let json = to_string_pretty(&data);
    let parsed: Option<Option<String>> = from_str(&json).unwrap();
    assert_eq!(parsed, Some(Some("nested".to_string())));

    // Note: Some(None) serializes to null, which deserializes back to None
    // This is a limitation of JSON representation - there's no way to distinguish
    // between None and Some(None) when both serialize to null
    let data: Option<Option<String>> = Some(None);
    let json = to_string_pretty(&data);
    let parsed: Option<Option<String>> = from_str(&json).unwrap();
    // This is expected behavior - Some(None) -> "null" -> None
    assert_eq!(parsed, None);
}

#[test]
fn test_empty_collections() {
    let empty_vec: Vec<i32> = vec![];
    let json = to_string_pretty(&empty_vec);
    let parsed: Vec<i32> = from_str(&json).unwrap();
    assert_eq!(parsed.len(), 0);

    let empty_map: HashMap<String, i32> = HashMap::new();
    let json = to_string_pretty(&empty_map);
    let parsed: HashMap<String, i32> = from_str(&json).unwrap();
    assert_eq!(parsed.len(), 0);
}

#[test]
fn test_special_characters_in_strings() {
    let person = Person {
        name: "Alice \"The Great\" O'Brien".to_string(),
        age: 30,
        email: Some("alice@test.com\ncc: bob@test.com".to_string()),
        role: Role::Admin,
        scores: vec![],
    };

    let json = to_string_pretty(&person);
    let parsed: Person = from_str(&json).unwrap();

    assert_eq!(parsed.name, "Alice \"The Great\" O'Brien");
    assert!(parsed.email.unwrap().contains("\n"));
}

#[test]
fn test_numeric_types() {
    let numbers = vec![0i32, -1, 42, 100, i32::MAX, i32::MIN];
    let json = to_string_pretty(&numbers);
    let parsed: Vec<i32> = from_str(&json).unwrap();
    assert_eq!(parsed, numbers);
}

#[test]
fn test_floating_point() {
    let floats = vec![0.0, 1.5, -2.5, 1e10, 1e-10];
    let json = to_string_pretty(&floats);
    let parsed: Vec<f64> = from_str(&json).unwrap();

    for (i, &expected) in floats.iter().enumerate() {
        assert!((parsed[i] - expected).abs() < 1e-9);
    }
}

#[test]
fn test_boolean_values() {
    let bools = vec![true, false, true, true, false];
    let json = to_string_pretty(&bools);
    let parsed: Vec<bool> = from_str(&json).unwrap();
    assert_eq!(parsed, bools);
}

#[test]
fn test_invalid_json_errors() {
    assert!(from_str::<Person>("").is_err());
    assert!(from_str::<Person>("not json").is_err());
    assert!(from_str::<Person>("{").is_err());
    assert!(from_str::<Person>(r#"{"name":}"#).is_err());
    assert!(from_str::<Person>(r#"{"name":"Alice"#).is_err());
}

#[test]
fn test_type_mismatch_errors() {
    // String where number expected
    assert!(
        from_str::<Person>(
            r#"{"name":"Alice","age":"thirty","email":null,"role":"Admin","scores":[]}"#
        )
        .is_err()
    );

    // Array where object expected
    assert!(from_str::<Person>(r#"[]"#).is_err());

    // Wrong enum variant
    assert!(
        from_str::<Person>(
            r#"{"name":"Alice","age":30,"email":null,"role":"SuperAdmin","scores":[]}"#
        )
        .is_err()
    );
}

#[test]
fn test_whitespace_handling() {
    let json = r#"  {  "name"  :  "Alice"  ,  "age"  :  30  ,  "email"  :  null  ,  "role"  :  "Admin"  ,  "scores"  :  [  ]  }  "#;
    let parsed: Person = from_str(json).unwrap();
    assert_eq!(parsed.name, "Alice");
    assert_eq!(parsed.age, 30);
}

#[test]
fn test_unicode_strings() {
    let person = Person {
        name: "Alice 你好 🎉".to_string(),
        age: 30,
        email: Some("alice@テスト.com".to_string()),
        role: Role::Admin,
        scores: vec![],
    };

    let json = to_string_pretty(&person);
    let parsed: Person = from_str(&json).unwrap();
    assert_eq!(parsed.name, "Alice 你好 🎉");
    assert_eq!(parsed.email.unwrap(), "alice@テスト.com");
}
