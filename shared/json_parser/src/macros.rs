//! Macros for JSON serialization.

/// Macro to implement both Serialize and Deserialize for a struct.
///
/// # Examples
///
/// ```ignore
/// #[derive(Debug, Clone)]
/// struct Config {
///     host: String,
///     port: u32,
/// }
///
/// impl Default for Config {
///     fn default() -> Self {
///         Self {
///             host: "127.0.0.1".to_string(),
///             port: 8080,
///         }
///     }
/// }
///
/// impl_json! {
///     Config {
///         host: String,
///         port: u32,
///     }
/// }
///
/// // Can deserialize with missing fields - uses defaults
/// let json = r#"{"port": 3000}"#;
/// let config: Config = from_str(json).unwrap();
/// // config.host will be "127.0.0.1" (default)
/// // config.port will be 3000 (from json)
/// ```
#[macro_export]
macro_rules! impl_json {
    ($struct_name:ident { $($field:ident: $field_ty:ty),* $(,)? }) => {
        // Implement Serialize
        impl $crate::Serialize for $struct_name {
            fn serialize(&self) -> $crate::JsonValue {
                let mut map = std::collections::HashMap::new();
                $(
                    map.insert(
                        stringify!($field).to_string(),
                        $crate::Serialize::serialize(&self.$field)
                    );
                )*
                $crate::JsonValue::Object(map)
            }
        }

        // Implement Deserialize with default values
        impl $crate::Deserialize for $struct_name {
            fn deserialize(value: &$crate::JsonValue) -> Result<Self, $crate::JsonError> {
                let obj = value
                    .as_object()
                    .ok_or_else(|| $crate::JsonError::TypeMismatch(
                        format!("Expected object for {}", stringify!($struct_name))
                    ))?;

                // Start with default values
                let mut result = Self::default();

                // Update fields that are present in JSON
                $(
                    if let Some(field_value) = obj.get(stringify!($field)) {
                        result.$field = <$field_ty as $crate::Deserialize>::deserialize(field_value)?;
                    }
                )*

                Ok(result)
            }
        }
    };
}

/// Macro to implement both Serialize and Deserialize for an enum with unit variants.
///
/// This macro handles simple enums (without associated data), serializing them
/// as JSON strings.
///
/// # Examples
///
/// ```ignore
/// use json_parser::impl_json_enum;
///
/// enum Role {
///     Owner,
///     Guest,
/// }
///
/// impl_json_enum! {
///     Role {
///         Owner,
///         Guest,
///     }
/// }
///
/// // Serializes to: "Owner" or "Guest"
/// let role = Role::Owner;
/// let json = json_parser::to_string(&role);
/// ```
#[macro_export]
macro_rules! impl_json_enum {
    ($enum_name:ident { $($variant:ident),* $(,)? }) => {
        // Implement Serialize
        impl $crate::Serialize for $enum_name {
            fn serialize(&self) -> $crate::JsonValue {
                match self {
                    $(
                        $enum_name::$variant => {
                            $crate::JsonValue::String(stringify!($variant).to_string())
                        }
                    )*
                }
            }
        }

        // Implement Deserialize
        impl $crate::Deserialize for $enum_name {
            fn deserialize(value: &$crate::JsonValue) -> Result<Self, $crate::JsonError> {
                let s = value
                    .as_string()
                    .ok_or_else(|| $crate::JsonError::TypeMismatch(
                        format!("Expected string for enum {}", stringify!($enum_name))
                    ))?;

                match s {
                    $(
                        stringify!($variant) => Ok($enum_name::$variant),
                    )*
                    _ => Err($crate::JsonError::TypeMismatch(
                        format!("Unknown variant '{}' for enum {}", s, stringify!($enum_name))
                    )),
                }
            }
        }
    };
}
