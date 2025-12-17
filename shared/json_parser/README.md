# JSON Parser - Zero-Dependency JSON Serialization

Simple JSON serialization/deserialization library with syntax similar to serde_json, but with zero external dependencies.

## Overview

Lightweight JSON parser for projects that cannot or prefer not to use external dependencies. Provides serialize/deserialize functionality with a simple macro-based API.

## Features

- ✅ Zero external dependencies (only Rust std)
- ✅ Simple macro-based serialization
- ✅ API similar to serde_json
- ✅ Support for all JSON types
- ✅ Nested structs and collections
- ✅ Type-safe enum serialization

## Quick Start

```rust
use json_parser::{impl_json, from_str, to_string_pretty};

// Define your struct
struct User {
    name: String,
    age: i32,
    active: bool,
}

// Add serialization with one macro
impl_json! {
    User {
        name: String,
        age: i32,
        active: bool,
    }
}

// Use it!
fn main() {
    // Deserialize from JSON
    let json = r#"{"name":"Alice","age":30,"active":true}"#;
    let user: User = from_str(json).unwrap();
    
    // Serialize to JSON
    let json_out = to_string_pretty(&user);
    println!("{}", json_out);
}
```

## API

```rust
// Deserialization
from_str::<T>(json: &str) -> Result<T, JsonError>

// Serialization
to_string_pretty<T>(value: &T) -> String

// Macros
impl_json! { StructName { field: Type, ... } }
impl_json_enum! { EnumName { Variant1, Variant2, ... } }
```

## Supported Types

| Type | Serialize | Deserialize |
|------|-----------|-------------|
| `String`, `&str` | ✅ | ✅ |
| `bool` | ✅ | ✅ |
| `i32`, `i64`, `u32`, `u64`, `usize` | ✅ | ✅ |
| `f32`, `f64` | ✅ | ✅ |
| `Vec<T>` | ✅ | ✅ |
| `Option<T>` | ✅ | ✅ |
| `HashMap<String, V>` | ✅ | ✅ |

## Examples

### Basic Struct

```rust
use json_parser::{impl_json, from_str, to_string_pretty};

struct Person {
    name: String,
    age: i32,
}

impl_json! {
    Person {
        name: String,
        age: i32,
    }
}

let json = r#"{"name":"Bob","age":25}"#;
let person: Person = from_str(json).unwrap();
println!("{}", to_string_pretty(&person));
```

### Enum Support

```rust
use json_parser::{impl_json_enum, from_str, to_string_pretty};

enum Status {
    Active,
    Inactive,
    Pending,
}

impl_json_enum! {
    Status {
        Active,
        Inactive,
        Pending,
    }
}

let status = Status::Active;
let json = to_string_pretty(&status); // "Active"
let parsed: Status = from_str(r#""Pending""#).unwrap();
```

### Nested Structures

```rust
use json_parser::{impl_json, impl_json_enum, from_str, to_string_pretty};

enum Role {
    Owner,
    Guest,
}

impl_json_enum! {
    Role {
        Owner,
        Guest,
    }
}

struct Participant {
    name: String,
    role: Role,
}

impl_json! {
    Participant {
        name: String,
        role: Role,
    }
}

struct Room {
    id: String,
    participants: Vec<Participant>,
}

impl_json! {
    Room {
        id: String,
        participants: Vec<Participant>,
    }
}

let json = r#"{
    "id": "ROOM123",
    "participants": [
        {"name": "Alice", "role": "Owner"},
        {"name": "Bob", "role": "Guest"}
    ]
}"#;

let room: Room = from_str(json).unwrap();
println!("{}", to_string_pretty(&room));
```

### Collections

```rust
use json_parser::{from_str, to_string_pretty};
use std::collections::HashMap;

// HashMap
let json = r#"{"key1":"value1","key2":"value2"}"#;
let map: HashMap<String, String> = from_str(json).unwrap();

// Vec
let json = r#"[1,2,3,4,5]"#;
let numbers: Vec<i32> = from_str(json).unwrap();

// Option
let some: Option<String> = from_str(r#""hello""#).unwrap();
let none: Option<String> = from_str("null").unwrap();
```

## Best Practices

**✅ DO:**
- Use meaningful field names that match your JSON structure
- Handle deserialization errors appropriately
- Use `Option<T>` for optional fields

**❌ DON'T:**
- Use this for extremely large JSON files (performance)
- Expect advanced features like field renaming or skipping
- Use with complex enum variants (only simple enums supported)

## Limitations

- Enums only support unit variants (no data)
- No custom serialization logic
- Field names must match JSON keys exactly
- Numbers are represented as `f64` (no arbitrary precision)

For complex use cases, consider using serde_json instead.

## Testing

```bash
# Run all tests
cargo test -p json_parser

# Run with output
cargo test -p json_parser -- --nocapture
```

## References

- [JSON Specification (RFC 8259)](https://tools.ietf.org/html/rfc8259)
- [serde_json Documentation](https://docs.rs/serde_json/) (for comparison)
