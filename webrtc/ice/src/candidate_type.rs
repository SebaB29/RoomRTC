//! ICE candidate types.
//!
//! Defines the different types of ICE candidates as specified in RFC 5245.

/// Represents the type of ICE candidate according to RFC 5245.
///
/// # Candidate Types
/// - **Host**: A candidate obtained by binding to a specific port on a local interface
/// - **Srflx**: Server Reflexive - obtained from a STUN server (external IP address)
/// - **Relay**: Relayed candidate obtained from a TURN server
/// - **Prflx**: Peer Reflexive - discovered during connectivity checks
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CandidateType {
    /// Host candidate - local interface address
    Host,
    /// Server Reflexive candidate - NAT-mapped address from STUN
    Srflx,
    /// Relay candidate - address from TURN server
    Relay,
    /// Peer Reflexive candidate - discovered during checks
    Prflx,
}

impl CandidateType {
    /// Parses a candidate type from a string.
    ///
    /// # Arguments
    /// * `s` - The candidate type string ("host", "srflx", "relay", "prflx")
    ///
    /// # Returns
    /// * `Ok(CandidateType)` - If parsing is successful
    /// * `Err(IceError)` - If the type is invalid
    pub fn parse(s: &str) -> Result<Self, crate::errors::IceError> {
        match s {
            "host" => Ok(CandidateType::Host),
            "srflx" => Ok(CandidateType::Srflx),
            "relay" => Ok(CandidateType::Relay),
            "prflx" => Ok(CandidateType::Prflx),
            _ => Err(crate::errors::IceError::InvalidCandidateType(s.to_string())),
        }
    }

    /// Returns the string representation of the candidate type.
    pub fn as_str(&self) -> &'static str {
        match self {
            CandidateType::Host => "host",
            CandidateType::Srflx => "srflx",
            CandidateType::Relay => "relay",
            CandidateType::Prflx => "prflx",
        }
    }
}

/// Provides a default candidate type of Host.
impl Default for CandidateType {
    fn default() -> Self {
        CandidateType::Host
    }
}

/// Implements the `Display` trait for [`CandidateType`], allowing it to be
/// formatted as a string.
impl std::fmt::Display for CandidateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_host() {
        let result = CandidateType::parse("host");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CandidateType::Host);
    }

    #[test]
    fn test_parse_srflx() {
        let result = CandidateType::parse("srflx");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CandidateType::Srflx);
    }

    #[test]
    fn test_parse_relay() {
        let result = CandidateType::parse("relay");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CandidateType::Relay);
    }

    #[test]
    fn test_parse_prflx() {
        let result = CandidateType::parse("prflx");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), CandidateType::Prflx);
    }

    #[test]
    fn test_parse_invalid_type() {
        let result = CandidateType::parse("invalid");
        assert!(result.is_err());
        match result {
            Err(crate::errors::IceError::InvalidCandidateType(s)) => {
                assert_eq!(s, "invalid");
            }
            _ => panic!("Expected InvalidCandidateType error"),
        }
    }

    #[test]
    fn test_parse_empty_string() {
        let result = CandidateType::parse("");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_case_sensitive() {
        assert!(CandidateType::parse("HOST").is_err());
        assert!(CandidateType::parse("Host").is_err());
        assert!(CandidateType::parse("SRFLX").is_err());
    }

    #[test]
    fn test_parse_with_whitespace() {
        assert!(CandidateType::parse(" host").is_err());
        assert!(CandidateType::parse("host ").is_err());
        assert!(CandidateType::parse(" host ").is_err());
    }

    #[test]
    fn test_as_str_host() {
        assert_eq!(CandidateType::Host.as_str(), "host");
    }

    #[test]
    fn test_as_str_srflx() {
        assert_eq!(CandidateType::Srflx.as_str(), "srflx");
    }

    #[test]
    fn test_as_str_relay() {
        assert_eq!(CandidateType::Relay.as_str(), "relay");
    }

    #[test]
    fn test_as_str_prflx() {
        assert_eq!(CandidateType::Prflx.as_str(), "prflx");
    }

    #[test]
    fn test_parse_roundtrip() {
        let types = vec![
            CandidateType::Host,
            CandidateType::Srflx,
            CandidateType::Relay,
            CandidateType::Prflx,
        ];

        for candidate_type in types {
            let str_repr = candidate_type.as_str();
            let parsed = CandidateType::parse(str_repr).unwrap();
            assert_eq!(parsed, candidate_type);
        }
    }

    #[test]
    fn test_display_trait_host() {
        let host = CandidateType::Host;
        assert_eq!(format!("{}", host), "host");
    }

    #[test]
    fn test_display_trait_srflx() {
        let srflx = CandidateType::Srflx;
        assert_eq!(format!("{}", srflx), "srflx");
    }

    #[test]
    fn test_display_trait_relay() {
        let relay = CandidateType::Relay;
        assert_eq!(format!("{}", relay), "relay");
    }

    #[test]
    fn test_display_trait_prflx() {
        let prflx = CandidateType::Prflx;
        assert_eq!(format!("{}", prflx), "prflx");
    }

    #[test]
    fn test_display_matches_as_str() {
        let types = vec![
            CandidateType::Host,
            CandidateType::Srflx,
            CandidateType::Relay,
            CandidateType::Prflx,
        ];

        for candidate_type in types {
            assert_eq!(format!("{}", candidate_type), candidate_type.as_str());
        }
    }
}
