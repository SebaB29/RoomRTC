//! SDP attribute representation.
//!
//! Attributes in SDP provide additional information about the session
//! or media descriptions.

/// Represents an attribute (a=) in an SDP message as defined in RFC 4566.
///
/// # Format
/// - `a=<attribute-name>` for flag attributes (no value)
/// - `a=<attribute-name>:<attribute-value>` for value attributes
#[derive(Debug, Clone)]
pub struct Attribute {
    pub name: String,
    pub value: Option<String>,
}

impl Attribute {
    /// Parses an attribute string into an `Attribute` struct.
    ///
    /// # Arguments
    /// * `value` - The attribute string to parse, without the leading "a="
    ///
    /// # Returns
    /// * `Ok(Attribute)` - Successfully parsed attribute
    /// * `Err(SdpError)` - If the attribute format is invalid
    pub fn parse(value: &str) -> Result<Self, crate::errors::SdpError> {
        let parts: Vec<&str> = value.splitn(2, ':').collect();
        match parts.len() {
            1 => Ok(Attribute {
                name: parts[0].to_string(),
                value: None,
            }),
            2 => Ok(Attribute {
                name: parts[0].to_string(),
                value: Some(parts[1].to_string()),
            }),
            _ => Err(crate::errors::SdpError::InvalidAttributeFormat),
        }
    }
}

/// Implements the Display trait to format attributes according to RFC 4566.
///
/// The format follows these rules:
/// - Flag attributes: `a=<name>`
/// - Value attributes: `a=<name>:<value>`
impl std::fmt::Display for Attribute {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.value {
            Some(val) => writeln!(f, "a={}:{}", self.name, val),
            None => writeln!(f, "a={}", self.name),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_attribute_parse_flag() {
        let attr = Attribute::parse("recvonly").unwrap();
        assert_eq!(attr.name, "recvonly");
        assert_eq!(attr.value, None);
    }

    #[test]
    fn test_attribute_parse_with_value() {
        let attr = Attribute::parse("rtpmap:96 VP8/90000").unwrap();
        assert_eq!(attr.name, "rtpmap");
        assert_eq!(attr.value, Some("96 VP8/90000".to_string()));
    }

    #[test]
    fn test_attribute_parse_with_colon_in_value() {
        let attr = Attribute::parse("fingerprint:sha-256 AA:BB:CC").unwrap();
        assert_eq!(attr.name, "fingerprint");
        assert_eq!(attr.value, Some("sha-256 AA:BB:CC".to_string()));
    }

    #[test]
    fn test_attribute_display_flag() {
        let attr = Attribute {
            name: "sendrecv".to_string(),
            value: None,
        };
        assert_eq!(format!("{}", attr), "a=sendrecv\n");
    }

    #[test]
    fn test_attribute_display_with_value() {
        let attr = Attribute {
            name: "rtpmap".to_string(),
            value: Some("96 VP8/90000".to_string()),
        };
        assert_eq!(format!("{}", attr), "a=rtpmap:96 VP8/90000\n");
    }
}
