//! ICE candidate representation and parsing.
//!
//! This module provides the core `Candidate` type for representing
//! ICE candidates according to RFC 5245, including parsing from and
//! formatting to SDP format.

use crate::{candidate_type::CandidateType, errors::IceError};
use std::net::IpAddr;

/// Represents an ICE candidate according to RFC 5245.
///
/// ICE candidates are encoded as SDP attributes in the format:
/// ```text
/// a=candidate:<foundation> <component-id> <transport> <priority> <connection-address> <port> typ <cand-type> [raddr <rel-addr>] [rport <rel-port>]
/// ```
///
/// # Fields
/// * `foundation` - Identifier for candidates from the same interface
/// * `component_id` - Component ID (1 for RTP, 2 for RTCP)
/// * `transport` - Transport protocol ("UDP" or "TCP")
/// * `priority` - Priority value for candidate selection
/// * `address` - IP address of the candidate
/// * `port` - Port number
/// * `candidate_type` - Type of candidate (host, srflx, relay, prflx)
/// * `related_address` - Related address for non-host candidates
/// * `related_port` - Related port for non-host candidates
#[derive(Debug, Clone)]
pub struct Candidate {
    pub foundation: String,
    pub component_id: u32,
    pub transport: String,
    pub priority: u32,
    pub address: IpAddr,
    pub port: u16,
    pub candidate_type: CandidateType,
    pub related_address: Option<IpAddr>,
    pub related_port: Option<u16>,
}

impl Candidate {
    /// Parses an ICE candidate from an SDP attribute value.
    ///
    /// Expected format:
    /// `<foundation> <component-id> <transport> <priority> <connection-address> <port> typ <cand-type> [raddr <rel-addr>] [rport <rel-port>]`
    ///
    /// # Arguments
    /// * `value` - The candidate string to parse (without "a=candidate:" prefix)
    ///
    /// # Returns
    /// * `Ok(Candidate)` - Successfully parsed candidate
    /// * `Err(IceError)` - If the format is invalid
    pub fn parse(value: &str) -> Result<Self, IceError> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() < 8 {
            return Err(IceError::InvalidCandidateFormat);
        }

        // Parse basic fields
        let foundation = parts[0].to_string();

        let component_id: u32 = parts[1].parse().map_err(|_| IceError::InvalidComponentId)?;

        if component_id != 1 && component_id != 2 {
            return Err(IceError::InvalidComponentId);
        }

        let transport = parts[2].to_string();
        if transport != "UDP" && transport != "TCP" {
            return Err(IceError::InvalidTransportProtocol);
        }

        let priority: u32 = parts[3].parse().map_err(|_| IceError::InvalidPriority)?;

        let address: IpAddr = parts[4].parse().map_err(|_| IceError::InvalidIpAddress)?;

        let port: u16 = parts[5].parse().map_err(|_| IceError::InvalidPort)?;

        // parts[6] should be "typ"
        if parts[6] != "typ" {
            return Err(IceError::InvalidCandidateFormat);
        }

        let candidate_type = CandidateType::parse(parts[7])?;

        // Parse optional related address and port
        let mut related_address = None;
        let mut related_port = None;

        let mut i = 8;
        while i < parts.len() {
            match parts[i] {
                "raddr" if i + 1 < parts.len() => {
                    related_address = Some(
                        parts[i + 1]
                            .parse()
                            .map_err(|_| IceError::InvalidIpAddress)?,
                    );
                    i += 2;
                }
                "rport" if i + 1 < parts.len() => {
                    related_port = Some(parts[i + 1].parse().map_err(|_| IceError::InvalidPort)?);
                    i += 2;
                }
                _ => i += 1,
            }
        }

        Ok(Candidate {
            foundation,
            component_id,
            transport,
            priority,
            address,
            port,
            candidate_type,
            related_address,
            related_port,
        })
    }

    /// Validates the candidate according to RFC 5245.
    ///
    /// # Returns
    /// * `Ok(())` - If the candidate is valid
    /// * `Err(IceError)` - If validation fails
    pub fn validate(&self) -> Result<(), IceError> {
        if self.foundation.is_empty() {
            return Err(IceError::InvalidFoundation);
        }

        if self.component_id != 1 && self.component_id != 2 {
            return Err(IceError::InvalidComponentId);
        }

        if self.transport != "UDP" && self.transport != "TCP" {
            return Err(IceError::InvalidTransportProtocol);
        }

        Ok(())
    }

    /// Calculates the priority for a candidate according to RFC 5245.
    ///
    /// Priority = (2^24)*(type preference) + (2^8)*(local preference) + (256 - component ID)
    ///
    /// # Arguments
    /// * `type_pref` - Type preference (0-126)
    /// * `local_pref` - Local preference (0-65535)
    ///
    /// # Returns
    /// The calculated priority value
    pub fn calculate_priority(type_pref: u32, local_pref: u32, component_id: u32) -> u32 {
        (type_pref << 24) + (local_pref << 8) + (256 - component_id)
    }

    /// Returns the default type preference for this candidate type.
    ///
    /// According to RFC 5245:
    /// - Host: 126
    /// - Peer Reflexive: 110
    /// - Server Reflexive: 100
    /// - Relay: 0
    pub fn default_type_preference(&self) -> u32 {
        match self.candidate_type {
            CandidateType::Host => 126,
            CandidateType::Prflx => 110,
            CandidateType::Srflx => 100,
            CandidateType::Relay => 0,
        }
    }
}

/// Implements the Display trait to format candidates as SDP attributes.
///
/// Format:
/// `a=candidate:<foundation> <component-id> <transport> <priority> <address> <port> typ <type> [raddr <rel-addr>] [rport <rel-port>]`
impl std::fmt::Display for Candidate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "candidate:{} {} {} {} {} {} typ {}",
            self.foundation,
            self.component_id,
            self.transport,
            self.priority,
            self.address,
            self.port,
            self.candidate_type
        )?;

        if let Some(raddr) = self.related_address {
            write!(f, " raddr {}", raddr)?;
        }

        if let Some(rport) = self.related_port {
            write!(f, " rport {}", rport)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_parse_valid_host_candidate() {
        let input = "1 1 UDP 2130706431 192.168.1.1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.foundation, "1");
        assert_eq!(candidate.component_id, 1);
        assert_eq!(candidate.transport, "UDP");
        assert_eq!(candidate.priority, 2130706431);
        assert_eq!(candidate.address, IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)));
        assert_eq!(candidate.port, 8080);
        assert_eq!(candidate.candidate_type, CandidateType::Host);
        assert!(candidate.related_address.is_none());
        assert!(candidate.related_port.is_none());
    }

    #[test]
    fn test_parse_valid_srflx_candidate_with_related() {
        let input = "2 1 UDP 1694498815 203.0.113.1 54321 typ srflx raddr 192.168.1.1 rport 8080";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.foundation, "2");
        assert_eq!(candidate.candidate_type, CandidateType::Srflx);
        assert_eq!(
            candidate.related_address,
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
        );
        assert_eq!(candidate.related_port, Some(8080));
    }

    #[test]
    fn test_parse_valid_relay_candidate() {
        let input = "3 1 UDP 16777215 198.51.100.1 3478 typ relay raddr 203.0.113.1 rport 54321";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.candidate_type, CandidateType::Relay);
    }

    #[test]
    fn test_parse_valid_prflx_candidate() {
        let input = "4 1 UDP 1845501695 198.51.100.2 9000 typ prflx";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.candidate_type, CandidateType::Prflx);
    }

    #[test]
    fn test_parse_component_id_2() {
        let input = "1 2 UDP 2130706431 192.168.1.1 8081 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.component_id, 2);
    }

    #[test]
    fn test_parse_tcp_transport() {
        let input = "1 1 TCP 2130706431 192.168.1.1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(candidate.transport, "TCP");
    }

    #[test]
    fn test_parse_ipv6_address() {
        let input = "1 1 UDP 2130706431 2001:db8::1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(
            candidate.address,
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1))
        );
    }

    #[test]
    fn test_parse_insufficient_fields() {
        let input = "1 1 UDP 2130706431 192.168.1.1";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IceError::InvalidCandidateFormat
        ));
    }

    #[test]
    fn test_parse_invalid_component_id() {
        let input = "1 3 UDP 2130706431 192.168.1.1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidComponentId));
    }

    #[test]
    fn test_parse_invalid_component_id_zero() {
        let input = "1 0 UDP 2130706431 192.168.1.1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidComponentId));
    }

    #[test]
    fn test_parse_non_numeric_component_id() {
        let input = "1 abc UDP 2130706431 192.168.1.1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidComponentId));
    }

    #[test]
    fn test_parse_invalid_transport() {
        let input = "1 1 SCTP 2130706431 192.168.1.1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IceError::InvalidTransportProtocol
        ));
    }

    #[test]
    fn test_parse_invalid_priority() {
        let input = "1 1 UDP abc 192.168.1.1 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidPriority));
    }

    #[test]
    fn test_parse_invalid_ip_address() {
        let input = "1 1 UDP 2130706431 999.999.999.999 8080 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidIpAddress));
    }

    #[test]
    fn test_parse_invalid_port() {
        let input = "1 1 UDP 2130706431 192.168.1.1 99999 typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidPort));
    }

    #[test]
    fn test_parse_non_numeric_port() {
        let input = "1 1 UDP 2130706431 192.168.1.1 abc typ host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidPort));
    }

    #[test]
    fn test_parse_missing_typ_keyword() {
        let input = "1 1 UDP 2130706431 192.168.1.1 8080 host";
        let result = Candidate::parse(input);

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IceError::InvalidCandidateFormat
        ));
    }

    #[test]
    fn test_parse_invalid_candidate_type() {
        let input = "1 1 UDP 2130706431 192.168.1.1 8080 typ invalid";
        let result = Candidate::parse(input);

        assert!(result.is_err());
    }

    #[test]
    fn test_parse_only_raddr_without_rport() {
        let input = "1 1 UDP 2130706431 192.168.1.1 8080 typ srflx raddr 10.0.0.1";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert_eq!(
            candidate.related_address,
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)))
        );
        assert!(candidate.related_port.is_none());
    }

    #[test]
    fn test_parse_only_rport_without_raddr() {
        let input = "1 1 UDP 2130706431 192.168.1.1 8080 typ srflx rport 5000";
        let result = Candidate::parse(input);

        assert!(result.is_ok());
        let candidate = result.unwrap();
        assert!(candidate.related_address.is_none());
        assert_eq!(candidate.related_port, Some(5000));
    }

    #[test]
    fn test_parse_raddr_and_rport_in_any_order() {
        let input1 = "1 1 UDP 2130706431 192.168.1.1 8080 typ srflx rport 5000 raddr 10.0.0.1";
        let result1 = Candidate::parse(input1);

        assert!(result1.is_ok());
        let candidate = result1.unwrap();
        assert_eq!(
            candidate.related_address,
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1)))
        );
        assert_eq!(candidate.related_port, Some(5000));
    }

    #[test]
    fn test_validate_valid_candidate() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        assert!(candidate.validate().is_ok());
    }

    #[test]
    fn test_validate_empty_foundation() {
        let candidate = Candidate {
            foundation: "".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        let result = candidate.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidFoundation));
    }

    #[test]
    fn test_validate_invalid_component_id() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 5,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        let result = candidate.validate();
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::InvalidComponentId));
    }

    #[test]
    fn test_validate_invalid_transport() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "SCTP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        let result = candidate.validate();
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            IceError::InvalidTransportProtocol
        ));
    }

    #[test]
    fn test_calculate_priority_component_1() {
        let priority = Candidate::calculate_priority(126, 65535, 1);
        // (126 << 24) + (65535 << 8) + (256 - 1)
        let expected = (126 << 24) + (65535 << 8) + 255;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_calculate_priority_component_2() {
        let priority = Candidate::calculate_priority(126, 65535, 2);
        // (126 << 24) + (65535 << 8) + (256 - 2)
        let expected = (126 << 24) + (65535 << 8) + 254;
        assert_eq!(priority, expected);
    }

    #[test]
    fn test_calculate_priority_different_type_prefs() {
        let host_priority = Candidate::calculate_priority(126, 65535, 1);
        let srflx_priority = Candidate::calculate_priority(100, 65535, 1);
        let relay_priority = Candidate::calculate_priority(0, 65535, 1);

        assert!(host_priority > srflx_priority);
        assert!(srflx_priority > relay_priority);
    }

    #[test]
    fn test_calculate_priority_zero_values() {
        let priority = Candidate::calculate_priority(0, 0, 1);
        assert_eq!(priority, 255);
    }

    #[test]
    fn test_default_type_preference_host() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 0,
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        assert_eq!(candidate.default_type_preference(), 126);
    }

    #[test]
    fn test_default_type_preference_prflx() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 0,
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8080,
            candidate_type: CandidateType::Prflx,
            related_address: None,
            related_port: None,
        };

        assert_eq!(candidate.default_type_preference(), 110);
    }

    #[test]
    fn test_default_type_preference_srflx() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 0,
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8080,
            candidate_type: CandidateType::Srflx,
            related_address: None,
            related_port: None,
        };

        assert_eq!(candidate.default_type_preference(), 100);
    }

    #[test]
    fn test_default_type_preference_relay() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 0,
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port: 8080,
            candidate_type: CandidateType::Relay,
            related_address: None,
            related_port: None,
        };

        assert_eq!(candidate.default_type_preference(), 0);
    }

    #[test]
    fn test_display_host_candidate() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        let output = format!("{}", candidate);
        assert_eq!(
            output,
            "candidate:1 1 UDP 2130706431 192.168.1.1 8080 typ host"
        );
    }

    #[test]
    fn test_display_srflx_candidate_with_related() {
        let candidate = Candidate {
            foundation: "2".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 1694498815,
            address: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
            port: 54321,
            candidate_type: CandidateType::Srflx,
            related_address: Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            related_port: Some(8080),
        };

        let output = format!("{}", candidate);
        assert_eq!(
            output,
            "candidate:2 1 UDP 1694498815 203.0.113.1 54321 typ srflx raddr 192.168.1.1 rport 8080"
        );
    }

    #[test]
    fn test_display_only_raddr() {
        let candidate = Candidate {
            foundation: "2".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 1694498815,
            address: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
            port: 54321,
            candidate_type: CandidateType::Srflx,
            related_address: Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            related_port: None,
        };

        let output = format!("{}", candidate);
        assert_eq!(
            output,
            "candidate:2 1 UDP 1694498815 203.0.113.1 54321 typ srflx raddr 192.168.1.1"
        );
    }

    #[test]
    fn test_display_only_rport() {
        let candidate = Candidate {
            foundation: "2".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 1694498815,
            address: IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)),
            port: 54321,
            candidate_type: CandidateType::Srflx,
            related_address: None,
            related_port: Some(8080),
        };

        let output = format!("{}", candidate);
        assert_eq!(
            output,
            "candidate:2 1 UDP 1694498815 203.0.113.1 54321 typ srflx rport 8080"
        );
    }

    #[test]
    fn test_display_ipv6() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        let output = format!("{}", candidate);
        assert!(output.contains("2001:db8::1"));
    }

    #[test]
    fn test_parse_display_roundtrip() {
        let original = "1 1 UDP 2130706431 192.168.1.1 8080 typ host";
        let candidate = Candidate::parse(original).unwrap();
        let displayed = format!("{}", candidate);

        // Parse the displayed string again
        let reparsed = Candidate::parse(&displayed.replace("candidate:", "")).unwrap();

        assert_eq!(candidate.foundation, reparsed.foundation);
        assert_eq!(candidate.component_id, reparsed.component_id);
        assert_eq!(candidate.transport, reparsed.transport);
        assert_eq!(candidate.priority, reparsed.priority);
        assert_eq!(candidate.address, reparsed.address);
        assert_eq!(candidate.port, reparsed.port);
    }

    #[test]
    fn test_clone_trait() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        let cloned = candidate.clone();
        assert_eq!(candidate.foundation, cloned.foundation);
        assert_eq!(candidate.priority, cloned.priority);
    }

    #[test]
    fn test_debug_trait() {
        let candidate = Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port: 8080,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        };

        let debug_output = format!("{:?}", candidate);
        assert!(debug_output.contains("Candidate"));
        assert!(debug_output.contains("foundation"));
    }
}
