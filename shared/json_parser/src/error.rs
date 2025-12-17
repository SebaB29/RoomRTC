//! Error types for JSON parsing operations.

/// Result type for JSON parsing operations.
pub type Result<T> = std::result::Result<T, JsonError>;

/// Error type for JSON parsing failures.
#[derive(Debug, Clone, PartialEq)]
pub enum JsonError {
    /// Input string is empty
    EmptyInput,
    /// Unexpected character at given position
    UnexpectedCharacter(char, usize),
    /// Unexpected end of input
    UnexpectedEndOfInput,
    /// Invalid number format
    InvalidNumber(String),
    /// Invalid escape sequence in string
    InvalidEscapeSequence(String),
    /// Unterminated string
    UnterminatedString,
    /// Expected comma or closing bracket
    ExpectedCommaOrClosingBracket,
    /// Expected colon after object key
    ExpectedColon,
    /// Duplicate object key
    DuplicateKey(String),
    /// Invalid Unicode escape sequence
    InvalidUnicodeEscape(String),
    /// Trailing characters after JSON value
    TrailingCharacters,
    /// Type mismatch during deserialization
    TypeMismatch(String),
    /// Missing required field during deserialization
    MissingField(String),
}

impl std::fmt::Display for JsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JsonError::EmptyInput => write!(f, "Empty JSON input"),
            JsonError::UnexpectedCharacter(c, pos) => {
                write!(f, "Unexpected character '{}' at position {}", c, pos)
            }
            JsonError::UnexpectedEndOfInput => write!(f, "Unexpected end of input"),
            JsonError::InvalidNumber(s) => write!(f, "Invalid number: {}", s),
            JsonError::InvalidEscapeSequence(s) => write!(f, "Invalid escape sequence: {}", s),
            JsonError::UnterminatedString => write!(f, "Unterminated string"),
            JsonError::ExpectedCommaOrClosingBracket => {
                write!(f, "Expected comma or closing bracket")
            }
            JsonError::ExpectedColon => write!(f, "Expected colon after object key"),
            JsonError::DuplicateKey(key) => write!(f, "Duplicate object key: {}", key),
            JsonError::InvalidUnicodeEscape(s) => {
                write!(f, "Invalid Unicode escape sequence: {}", s)
            }
            JsonError::TrailingCharacters => write!(f, "Trailing characters after JSON value"),
            JsonError::TypeMismatch(s) => write!(f, "Type mismatch: {}", s),
            JsonError::MissingField(s) => write!(f, "Missing required field: {}", s),
        }
    }
}

impl std::error::Error for JsonError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_error_display() {
        let err = JsonError::EmptyInput;
        assert_eq!(err.to_string(), "Empty JSON input");

        let err = JsonError::UnexpectedCharacter('x', 5);
        assert_eq!(err.to_string(), "Unexpected character 'x' at position 5");

        let err = JsonError::DuplicateKey("name".to_string());
        assert_eq!(err.to_string(), "Duplicate object key: name");
    }
}
