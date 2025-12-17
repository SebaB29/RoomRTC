//! SDP origin field representation.
//!
//! The origin field provides unique identification for an SDP session
//! and information about the session creator.

/// Represents an Origin field in an SDP message according to RFC 4566.
///
/// The Origin field ('o=') provides unique identification for a session. Its format is:
/// `o=<username> <sess-id> <sess-version> <nettype> <addrtype> <unicast-address>`
///
/// According to RFC 4566:
/// - Username: The originator of the session (e.g., "-" for anonymous)
/// - Session ID: A unique numeric identifier for the session
/// - Session Version: A version number that increases when the session data is modified
/// - Network Type: The type of network (typically "IN" for Internet)
/// - Address Type: The type of the address (e.g., "IP4" or "IP6")
/// - Unicast Address: The originator's address
#[derive(Debug, Clone)]
pub struct Origin {
    pub username: String,
    pub session_id: u64,
    pub session_version: u64,
    pub network_type: String,
    pub address_type: String,
    pub unicast_address: String,
}

impl Origin {
    /// Parses an Origin field from an SDP string.
    ///
    /// The expected format is:
    /// `<username> <sess-id> <sess-version> <nettype> <addrtype> <unicast-address>`
    ///
    /// # Arguments
    /// * `value` - The string containing the origin field value to parse
    ///
    /// # Returns
    /// * `Ok(Origin)` - If parsing is successful
    /// * `Err(SdpError)` - If the string format is invalid:
    ///   - `InvalidOriginFormat` - If the string doesn't contain exactly 6 parts
    ///   - `InvalidSessionId` - If session ID or version cannot be parsed as u64
    pub fn parse(value: &str) -> Result<Self, crate::errors::SdpError> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() != 6 {
            return Err(crate::errors::SdpError::InvalidOriginFormat);
        }

        let session_id = parts[1]
            .parse()
            .map_err(|_| crate::errors::SdpError::InvalidSessionId)?;

        let session_version = parts[2]
            .parse()
            .map_err(|_| crate::errors::SdpError::InvalidSessionId)?;

        Ok(Origin {
            username: parts[0].to_string(),
            session_id,
            session_version,
            network_type: parts[3].to_string(),
            address_type: parts[4].to_string(),
            unicast_address: parts[5].to_string(),
        })
    }

    /// Validates the Origin field according to RFC 4566 specifications.
    ///
    /// This method performs the following checks:
    /// - Network type must be "IN" (Internet)
    /// - Address type must be either "IP4" or "IP6"
    /// - Session ID must be non-zero
    ///
    /// # Returns
    /// * `Ok(())` - If all validations pass
    /// * `Err(SdpError)` - If any validation fails, with specific error variants:
    ///   - `InvalidNetworkType` - If network type is not "IN"
    ///   - `InvalidAddressType` - If address type is neither "IP4" nor "IP6"
    ///   - `InvalidSessionId` - If session ID is zero
    pub fn validate(&self) -> Result<(), crate::errors::SdpError> {
        // Network type must be IN
        if self.network_type != "IN" {
            return Err(crate::errors::SdpError::InvalidNetworkType);
        }

        // Address type must be IP4 or IP6
        if self.address_type != "IP4" && self.address_type != "IP6" {
            return Err(crate::errors::SdpError::InvalidAddressType);
        }

        // Session ID and version must be positive
        if self.session_id == 0 {
            return Err(crate::errors::SdpError::InvalidSessionId);
        }

        Ok(())
    }
}

/// Provides a default implementation for [`Origin`].
impl Default for Origin {
    fn default() -> Self {
        Self {
            username: String::from("-"),
            session_id: 0,
            session_version: 0,
            network_type: String::from("IN"),
            address_type: String::from("IP4"),
            unicast_address: String::from("0.0.0.0"),
        }
    }
}

/// Implements the Display trait to format an Origin field according to RFC 4566.
///
/// The origin field is formatted as:
/// `o=<username> <sess-id> <sess-version> <nettype> <addrtype> <unicast-address>`
impl std::fmt::Display for Origin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "o={} {} {} {} {} {}",
            self.username,
            self.session_id,
            self.session_version,
            self.network_type,
            self.address_type,
            self.unicast_address
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_parse_valid() {
        let origin = Origin::parse("- 123456 789012 IN IP4 192.168.1.1").unwrap();
        assert_eq!(origin.username, "-");
        assert_eq!(origin.session_id, 123456);
        assert_eq!(origin.session_version, 789012);
        assert_eq!(origin.network_type, "IN");
        assert_eq!(origin.address_type, "IP4");
        assert_eq!(origin.unicast_address, "192.168.1.1");
    }

    #[test]
    fn test_origin_parse_ipv6() {
        let origin = Origin::parse("user 1 2 IN IP6 ::1").unwrap();
        assert_eq!(origin.username, "user");
        assert_eq!(origin.session_id, 1);
        assert_eq!(origin.session_version, 2);
        assert_eq!(origin.network_type, "IN");
        assert_eq!(origin.address_type, "IP6");
        assert_eq!(origin.unicast_address, "::1");
    }

    #[test]
    fn test_origin_parse_invalid_format() {
        assert!(Origin::parse("- 123456 789012 IN IP4").is_err());
        assert!(Origin::parse("- 123456 789012").is_err());
        assert!(Origin::parse("").is_err());
    }

    #[test]
    fn test_origin_parse_invalid_session_id() {
        assert!(Origin::parse("- abc 789012 IN IP4 192.168.1.1").is_err());
        assert!(Origin::parse("- 123456 xyz IN IP4 192.168.1.1").is_err());
    }

    #[test]
    fn test_origin_validate_valid() {
        let origin = Origin {
            username: "-".to_string(),
            session_id: 123456,
            session_version: 1,
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            unicast_address: "192.168.1.1".to_string(),
        };
        assert!(origin.validate().is_ok());
    }

    #[test]
    fn test_origin_validate_invalid_network_type() {
        let origin = Origin {
            username: "-".to_string(),
            session_id: 123456,
            session_version: 1,
            network_type: "XX".to_string(),
            address_type: "IP4".to_string(),
            unicast_address: "192.168.1.1".to_string(),
        };
        assert!(origin.validate().is_err());
    }

    #[test]
    fn test_origin_validate_invalid_address_type() {
        let origin = Origin {
            username: "-".to_string(),
            session_id: 123456,
            session_version: 1,
            network_type: "IN".to_string(),
            address_type: "IP5".to_string(),
            unicast_address: "192.168.1.1".to_string(),
        };
        assert!(origin.validate().is_err());
    }

    #[test]
    fn test_origin_validate_zero_session_id() {
        let origin = Origin {
            username: "-".to_string(),
            session_id: 0,
            session_version: 1,
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            unicast_address: "192.168.1.1".to_string(),
        };
        assert!(origin.validate().is_err());
    }

    #[test]
    fn test_origin_default() {
        let origin = Origin::default();
        assert_eq!(origin.username, "-");
        assert_eq!(origin.session_id, 0);
        assert_eq!(origin.session_version, 0);
        assert_eq!(origin.network_type, "IN");
        assert_eq!(origin.address_type, "IP4");
        assert_eq!(origin.unicast_address, "0.0.0.0");
    }

    #[test]
    fn test_origin_display() {
        let origin = Origin {
            username: "-".to_string(),
            session_id: 123456,
            session_version: 789012,
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            unicast_address: "192.168.1.1".to_string(),
        };
        let display = format!("{}", origin);
        assert_eq!(display, "o=- 123456 789012 IN IP4 192.168.1.1\n");
    }
}
