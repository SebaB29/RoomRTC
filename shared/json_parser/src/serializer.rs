//! JSON serialization utilities.

/// Escapes special characters in a string for JSON serialization.
pub fn escape_json_string(s: &str) -> String {
    let mut result = String::new();
    for ch in s.chars() {
        match ch {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\x08' => result.push_str("\\b"),
            '\x0C' => result.push_str("\\f"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            ch if ch.is_control() => {
                // Escape control characters as \uXXXX
                result.push_str(&format!("\\u{:04X}", ch as u32));
            }
            ch => result.push(ch),
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_json_string() {
        assert_eq!(escape_json_string("hello"), "hello");
        assert_eq!(escape_json_string("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json_string("hello\\world"), "hello\\\\world");
        assert_eq!(escape_json_string("hello\nworld"), "hello\\nworld");
        assert_eq!(escape_json_string("hello\rworld"), "hello\\rworld");
        assert_eq!(escape_json_string("hello\tworld"), "hello\\tworld");
    }

    #[test]
    fn test_escape_control_characters() {
        assert_eq!(escape_json_string("hello\x08world"), "hello\\bworld");
        assert_eq!(escape_json_string("hello\x0cworld"), "hello\\fworld");
    }
}
