# Shared Libraries

This directory contains reusable libraries that provide core functionality for the project. All libraries are implemented using only Rust standard library (zero external dependencies) for maximum portability and minimal overhead.

## 📚 Available Libraries

1. **[logger](logger/)** - Thread-safe asynchronous logging system
2. **[json_parser](json_parser/)** - RFC 8259 compliant JSON parser
3. **[config_loader](config_loader/)** - Generic configuration file loader

## Libraries Overview

### 📦 logger
**Purpose**: Flexible logging system with multiple output targets and log levels.

**Status**: ✅ Complete and Production Ready

**Key Features**:
- Multiple log levels (DEBUG, INFO, WARN, ERROR)
- Multiple output targets (stdout, stderr, file)
- Thread-safe logging
- Timestamp and level formatting
- Simple API

**Documentation**: See [logger/README.md](logger/README.md)

**Quick Example**:
```rust
use logger::{Logger, LogLevel, LogWriter};

let logger = Logger::new(LogLevel::INFO, LogWriter::Stdout);
logger.info("Application started");
logger.error("Something went wrong");
```

---

### 📦 json_parser
**Purpose**: Complete JSON parser and serializer following RFC 8259.

**Status**: ✅ Complete and Production Ready

**Key Features**:
- Full JSON support (objects, arrays, strings, numbers, booleans, null)
- Unicode escape sequences
- Comprehensive error reporting with positions
- Convenience macros for JSON construction
- Type-safe value access

**Documentation**: See [json_parser/README.md](json_parser/README.md)

**Quick Example**:
```rust
use json_parser::{parse_json, JsonValue, json_object};

// Parse JSON
let value = parse_json(r#"{"name": "Alice", "age": 30}"#)?;

// Create JSON
let person = json_object! {
    "name" => JsonValue::String("Bob".to_string()),
    "age" => JsonValue::Number(25.0)
};
```

---

### 📦 config_loader
**Purpose**: Generic configuration file loader with automatic discovery.

**Status**: ✅ Complete and Production Ready

**Philosophy**: 
- **Generic**: Only loads files, doesn't impose structure
- **Flexible**: Consumer defines parsing and validation
- **Simple**: Automatic search in `./config/`, `./`, or `CONFIG_PATH`
- **Type-safe by consumer**: You define your own structs

**Key Features**:
- Automatic file discovery in common locations
- Zero external dependencies (only std + json_parser)
- Consumer controls struct definition and parsing
- Works with any format (JSON, TOML-like, custom)

**Documentation**: See [config_loader/README.md](config_loader/README.md)

**Quick Example**:
```rust
use config_loader::find_and_load;
use json_parser::{parse_json, JsonValue};

// 1. Define YOUR struct
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
}

impl ServerConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        // 2. Load file (automatic discovery)
        let content = find_and_load("server_config.json")?;
        
        // 3. Parse with json_parser
        let json = parse_json(&content)?;
        
        // 4. YOUR logic to map to struct
        Self::from_json(json)
    }
}
```

---

## Architecture

All shared libraries follow a consistent modular architecture:

```
library_name/
├── Cargo.toml          # Package configuration
├── README.md           # Complete documentation
└── src/
    ├── lib.rs          # Public API and exports
    ├── error.rs        # Error types
    ├── [module].rs     # Focused functionality modules
    └── tests/          # Unit tests (inline)
```

### Design Principles

1. **Zero Dependencies**: Only use Rust standard library
2. **Modular Design**: Each module has a single, clear responsibility
3. **Type Safety**: Leverage Rust's type system for compile-time guarantees
4. **Error Handling**: Comprehensive error types with meaningful messages
5. **Documentation**: Complete README with examples and API reference
6. **Testing**: Comprehensive unit tests for all functionality

## Usage in Your Project

Add these libraries to your `Cargo.toml`:

```toml
[dependencies]
logger = { path = "../shared/logger" }
json_parser = { path = "../shared/json_parser" }
http_lib = { path = "../shared/http_lib" }
config_loader = { path = "../shared/config_loader" }
```

## Testing

Run tests for all shared libraries:

```bash
# Test all libraries
cargo test -p logger -p json_parser -p config_loader

# Test a specific library
cargo test -p json_parser

# Run with output
cargo test -p logger -- --nocapture
```

## Development Guidelines

When modifying or creating shared libraries:

1. **Keep it simple**: Don't add unnecessary complexity
2. **Document everything**: Public APIs must have documentation
3. **Test thoroughly**: Aim for high test coverage
4. **Follow patterns**: Use the established architecture from existing libraries
5. **Minimal dependencies**: Prefer std library when possible
6. **Maintain backwards compatibility**: Don't break existing APIs

## Integration Example

### Application with Logging and Configuration

```rust
use logger::{Logger, LogLevel};
use config_loader::find_and_load;
use json_parser::parse_json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Setup logging
    let logger = Logger::new("app.log".into(), LogLevel::Info)?;
    
    // Load configuration
    let config_content = find_and_load("server_config.json")?;
    let config = parse_json(&config_content)?;
    
    logger.info("Application started");
    logger.info(&format!("Configuration loaded: {:?}", config));
    
    // Your application logic here...
    
    Ok(())
}
```

---
            
            return HttpResponse::ok().json(&response_data.to_string());
        }
    }
    
    HttpResponse::bad_request("Invalid request")
}
```

## Performance Considerations

- **logger**: Minimal overhead, synchronous writes
- **json_parser**: Single-pass parsing, O(n) complexity
- **http_lib**: Efficient streaming parsing, minimal allocations
- **config_manager**: HashMap-based storage, O(1) average lookup

## Future Enhancements

Potential improvements for shared libraries:

### logger
- [ ] Async logging support
- [ ] Log rotation
- [ ] Structured logging (JSON output)
- [ ] Log filtering by module

## Future Enhancements

### logger
- [ ] Log rotation by size
- [ ] Compression of old logs
- [ ] Remote logging support

### json_parser
- [ ] Streaming parser for large files
- [ ] Custom serialization traits
- [ ] Pretty-printing with indentation

### config_loader
- [ ] Environment variable substitution
- [ ] Configuration file watching
- [ ] Validation hooks

## Contributing

When contributing to shared libraries:

1. Follow the existing code style
2. Add comprehensive tests for new features
3. Update the README with examples
4. Keep the API backwards compatible
5. Document all public items

---

**Part of the RoomRTC project | Rusty Coders | Taller de Programación I - FIUBA - 2025**
