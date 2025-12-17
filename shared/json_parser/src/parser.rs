//! JSON parsing implementation.

use crate::error::JsonError;
use crate::value::JsonValue;
use std::collections::HashMap;

/// Parses a JSON string into a JsonValue.
///
/// This function implements a complete JSON parser that handles:
/// - Objects with string keys
/// - Arrays
/// - Strings (with escape sequences)
/// - Numbers (integers and floats)
/// - Booleans
/// - Null values
///
/// # Errors
///
/// Returns a `JsonError` if the input is not valid JSON.
///
/// # Examples
///
/// ```
/// use json_parser::{JsonValue, parse_json};
///
/// let result = parse_json(r#"{"key": "value", "number": 42}"#);
/// assert!(result.is_ok());
///
/// let json = result.unwrap();
/// assert_eq!(json.get_path("key").and_then(|v| v.as_string()), Some("value"));
/// assert_eq!(json.get_path("number").and_then(|v| v.as_number()), Some(42.0));
/// ```
pub fn parse_json(input: &str) -> Result<JsonValue, JsonError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(JsonError::EmptyInput);
    }

    let mut parser = JsonParser::new(trimmed);
    let result = parser.parse_value()?;

    // Check for trailing characters
    parser.skip_whitespace();
    if !parser.is_at_end() {
        return Err(JsonError::TrailingCharacters);
    }

    Ok(result)
}

/// Internal JSON parser implementation
struct JsonParser<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> JsonParser<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn parse_value(&mut self) -> Result<JsonValue, JsonError> {
        self.skip_whitespace();

        if self.is_at_end() {
            return Err(JsonError::UnexpectedEndOfInput);
        }

        let ch = self.current_char();
        match ch {
            '{' => self.parse_object(),
            '[' => self.parse_array(),
            '"' => self.parse_string(),
            't' => self.parse_true(),
            'f' => self.parse_false(),
            'n' => self.parse_null(),
            '0'..='9' | '-' => self.parse_number(),
            _ => Err(JsonError::UnexpectedCharacter(ch, self.position)),
        }
    }

    fn parse_object(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_char('{')?;
        self.skip_whitespace();

        if self.is_at_end() {
            return Err(JsonError::UnexpectedEndOfInput);
        }

        let mut map = HashMap::new();

        if self.current_char() == '}' {
            self.advance();
            return Ok(JsonValue::Object(map));
        }

        loop {
            // Parse key
            let key = self.parse_string()?;
            let key_str = if let JsonValue::String(s) = key {
                s
            } else {
                unreachable!()
            };

            self.skip_whitespace();
            self.expect_char(':')?;
            self.skip_whitespace();

            // Parse value
            let value = self.parse_value()?;

            // Check for duplicate keys
            if map.contains_key(&key_str) {
                return Err(JsonError::DuplicateKey(key_str));
            }

            map.insert(key_str, value);

            self.skip_whitespace();
            if self.is_at_end() {
                return Err(JsonError::UnexpectedEndOfInput);
            }
            if self.current_char() == '}' {
                self.advance();
                break;
            } else {
                self.expect_char(',')?;
                self.skip_whitespace();
            }
        }

        Ok(JsonValue::Object(map))
    }

    fn parse_array(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_char('[')?;
        self.skip_whitespace();

        if self.is_at_end() {
            return Err(JsonError::UnexpectedEndOfInput);
        }

        let mut array = Vec::new();

        if self.current_char() == ']' {
            self.advance();
            return Ok(JsonValue::Array(array));
        }

        loop {
            let value = self.parse_value()?;
            array.push(value);

            self.skip_whitespace();
            if self.is_at_end() {
                return Err(JsonError::UnexpectedEndOfInput);
            }
            if self.current_char() == ']' {
                self.advance();
                break;
            } else {
                self.expect_char(',')?;
                self.skip_whitespace();
            }
        }

        Ok(JsonValue::Array(array))
    }

    fn parse_string(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_char('"')?;

        let mut result = String::new();
        let mut escaped = false;

        while !self.is_at_end() {
            let ch = self.current_char();
            self.advance();

            if escaped {
                self.process_escape_sequence(ch, &mut result)?;
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                return Ok(JsonValue::String(result));
            } else {
                result.push(ch);
            }
        }

        Err(JsonError::UnterminatedString)
    }

    fn process_escape_sequence(&mut self, ch: char, result: &mut String) -> Result<(), JsonError> {
        match ch {
            '"' => result.push('"'),
            '\\' => result.push('\\'),
            '/' => result.push('/'),
            'b' => result.push('\x08'),
            'f' => result.push('\x0C'),
            'n' => result.push('\n'),
            'r' => result.push('\r'),
            't' => result.push('\t'),
            'u' => {
                let code = self.parse_unicode_escape()?;
                if let Some(ch) = char::from_u32(code) {
                    result.push(ch);
                } else {
                    return Err(JsonError::InvalidUnicodeEscape(format!("\\u{:04X}", code)));
                }
            }
            _ => return Err(JsonError::InvalidEscapeSequence(format!("\\{}", ch))),
        }
        Ok(())
    }

    fn parse_unicode_escape(&mut self) -> Result<u32, JsonError> {
        let mut code = 0u32;
        for _ in 0..4 {
            if self.is_at_end() {
                return Err(JsonError::UnexpectedEndOfInput);
            }
            let ch = self.current_char();
            self.advance();

            let digit = match ch {
                '0'..='9' => ch as u32 - '0' as u32,
                'a'..='f' => ch as u32 - 'a' as u32 + 10,
                'A'..='F' => ch as u32 - 'A' as u32 + 10,
                _ => {
                    return Err(JsonError::InvalidUnicodeEscape(format!(
                        "Invalid hex digit: {}",
                        ch
                    )));
                }
            };
            code = code * 16 + digit;
        }
        Ok(code)
    }

    fn parse_number(&mut self) -> Result<JsonValue, JsonError> {
        let start = self.position;

        // Optional minus sign
        if self.current_char() == '-' {
            self.advance();
        }

        // Integer part
        if self.current_char() == '0' {
            self.advance();
        } else if self.current_char().is_ascii_digit() {
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                self.advance();
            }
        } else {
            return Err(JsonError::InvalidNumber("Expected digit".to_string()));
        }

        // Fractional part
        if !self.is_at_end() && self.current_char() == '.' {
            self.advance();
            if self.is_at_end() || !self.current_char().is_ascii_digit() {
                return Err(JsonError::InvalidNumber(
                    "Expected digit after decimal point".to_string(),
                ));
            }
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                self.advance();
            }
        }

        // Exponent part
        if !self.is_at_end() && (self.current_char() == 'e' || self.current_char() == 'E') {
            self.advance();
            if !self.is_at_end() && (self.current_char() == '+' || self.current_char() == '-') {
                self.advance();
            }
            if self.is_at_end() || !self.current_char().is_ascii_digit() {
                return Err(JsonError::InvalidNumber(
                    "Expected digit in exponent".to_string(),
                ));
            }
            while !self.is_at_end() && self.current_char().is_ascii_digit() {
                self.advance();
            }
        }

        let number_str = &self.input[start..self.position];
        match number_str.parse::<f64>() {
            Ok(num) => Ok(JsonValue::Number(num)),
            Err(_) => Err(JsonError::InvalidNumber(number_str.to_string())),
        }
    }

    fn parse_true(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_string("true")?;
        Ok(JsonValue::Bool(true))
    }

    fn parse_false(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_string("false")?;
        Ok(JsonValue::Bool(false))
    }

    fn parse_null(&mut self) -> Result<JsonValue, JsonError> {
        self.expect_string("null")?;
        Ok(JsonValue::Null)
    }

    fn expect_char(&mut self, expected: char) -> Result<(), JsonError> {
        if self.is_at_end() {
            return Err(JsonError::UnexpectedEndOfInput);
        }
        let ch = self.current_char();
        if ch == expected {
            self.advance();
            Ok(())
        } else {
            Err(JsonError::UnexpectedCharacter(ch, self.position))
        }
    }

    fn expect_string(&mut self, expected: &str) -> Result<(), JsonError> {
        for ch in expected.chars() {
            self.expect_char(ch)?;
        }
        Ok(())
    }

    fn current_char(&self) -> char {
        self.input[self.position..].chars().next().unwrap()
    }

    fn advance(&mut self) {
        if !self.is_at_end() {
            let ch = self.current_char();
            self.position += ch.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while !self.is_at_end() && self.current_char().is_whitespace() {
            self.advance();
        }
    }

    fn is_at_end(&self) -> bool {
        self.position >= self.input.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_parse_simple_object() {
        let json = r#"{"key": "value"}"#;
        let result = parse_json(json).unwrap();
        assert_eq!(
            result.get_path("key").and_then(|v| v.as_string()),
            Some("value")
        );
    }

    #[test]
    fn test_parse_nested_object() {
        let json = r#"{"user": {"name": "Alice", "age": 30}}"#;
        let result = parse_json(json).unwrap();
        assert_eq!(
            result.get_path("user.name").and_then(|v| v.as_string()),
            Some("Alice")
        );
        assert_eq!(
            result.get_path("user.age").and_then(|v| v.as_number()),
            Some(30.0)
        );
    }

    #[test]
    fn test_parse_array() {
        let json = r#"["a", "b", "c"]"#;
        let result = parse_json(json).unwrap();
        let arr = result.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0].as_string(), Some("a"));
    }

    #[test]
    fn test_parse_number() {
        let json = r#"42"#;
        let result = parse_json(json).unwrap();
        assert_eq!(result.as_number(), Some(42.0));
    }

    #[test]
    fn test_parse_float() {
        let json = r#"3.14"#;
        let result = parse_json(json).unwrap();
        assert_eq!(result.as_number(), Some(PI));
    }

    #[test]
    fn test_parse_boolean() {
        let json = r#"true"#;
        let result = parse_json(json).unwrap();
        assert_eq!(result.as_bool(), Some(true));
    }

    #[test]
    fn test_parse_null() {
        let json = r#"null"#;
        let result = parse_json(json).unwrap();
        assert!(result.is_null());
    }

    #[test]
    fn test_parse_string_with_escapes() {
        let json = r#""Hello\nWorld""#;
        let result = parse_json(json).unwrap();
        assert_eq!(result.as_string(), Some("Hello\nWorld"));
    }

    #[test]
    fn test_invalid_json() {
        assert!(parse_json("").is_err());
        assert!(parse_json("{").is_err());
        assert!(parse_json(r#""unterminated"#).is_err());
        assert!(parse_json("invalid").is_err());
    }

    #[test]
    fn test_trailing_characters() {
        assert!(parse_json(r#"{"key": "value"} extra"#).is_err());
    }

    #[test]
    fn test_duplicate_keys() {
        assert!(parse_json(r#"{"key": "value1", "key": "value2"}"#).is_err());
    }
}
