//! Simple JSON serialization library.
//!
//! Zero-dependency JSON parsing with macro-based serialization similar to serde_json.
//!
//! # Example
//!
//! ```
//! use json_parser::{impl_json, from_str, to_string_pretty};
//!
//! struct User {
//!     name: String,
//!     age: i32,
//! }
//!
//! impl_json! {
//!     User {
//!         name: String,
//!         age: i32,
//!     }
//! }
//!
//! impl Default for User {
//!     fn default() -> Self {  
//!         User {
//!             name: String::new(),
//!             age: 0,
//!         }
//!     }
//! }
//!
//! let json = r#"{"name":"Alice","age":30}"#;
//! let user: User = from_str(json).unwrap();
//! let output = to_string_pretty(&user);
//! ```

pub mod de;
pub mod error;
pub mod macros;
mod parser;
pub mod ser;
mod serializer;
mod value;

pub use de::{Deserialize, from_str};
pub use error::{JsonError, Result};
pub use parser::parse_json;
pub use ser::{Serialize, to_string, to_string_pretty};
pub use value::JsonValue;
