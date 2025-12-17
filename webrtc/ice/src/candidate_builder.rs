//! Builder pattern for ICE candidates.
//!
//! Provides a fluent API for constructing ICE candidates with validation.

use crate::{candidate::Candidate, candidate_type::CandidateType, errors::IceError};
use std::net::IpAddr;

/// Builder for constructing ICE candidates.
pub struct CandidateBuilder {
    foundation: Option<String>,
    component_id: u32,
    transport: String,
    priority: Option<u32>,
    address: Option<IpAddr>,
    port: Option<u16>,
    candidate_type: CandidateType,
    related_address: Option<IpAddr>,
    related_port: Option<u16>,
}

impl Default for CandidateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl CandidateBuilder {
    /// Creates a new candidate builder with default values.
    pub fn new() -> Self {
        Self {
            foundation: None,
            component_id: 1, // Default to RTP
            transport: "UDP".to_string(),
            priority: None,
            address: None,
            port: None,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        }
    }

    /// Sets the foundation identifier.
    ///
    /// # Arguments
    /// * `foundation` - A string representing the foundation
    pub fn foundation(mut self, foundation: impl Into<String>) -> Self {
        self.foundation = Some(foundation.into());
        self
    }

    /// Sets the component ID (1 for RTP, 2 for RTCP).
    ///
    /// # Arguments
    /// * `component_id` - The component ID
    pub fn component_id(mut self, component_id: u32) -> Self {
        self.component_id = component_id;
        self
    }

    /// Sets the transport protocol.
    ///
    /// # Arguments
    /// * `transport` - The transport protocol (e.g., "UDP", "TCP")
    pub fn transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = transport.into();
        self
    }

    /// Sets the priority value.
    ///
    /// # Arguments
    /// * `priority` - The priority value
    pub fn priority(mut self, priority: u32) -> Self {
        self.priority = Some(priority);
        self
    }

    /// Sets the IP address.
    ///
    /// # Arguments
    /// * `address` - The IP address
    pub fn address(mut self, address: IpAddr) -> Self {
        self.address = Some(address);
        self
    }

    /// Sets the port number.
    ///
    /// # Arguments
    /// * `port` - The port number
    pub fn port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Sets the candidate type.
    ///
    /// # Arguments
    /// * `candidate_type` - The type of candidate (Host, Srflx, Prflx, Relay)
    pub fn candidate_type(mut self, candidate_type: CandidateType) -> Self {
        self.candidate_type = candidate_type;
        self
    }

    /// Sets the related address (for non-host candidates).
    ///
    /// # Arguments
    /// * `related_address` - The related IP address
    pub fn related_address(mut self, related_address: IpAddr) -> Self {
        self.related_address = Some(related_address);
        self
    }

    /// Sets the related port (for non-host candidates).
    ///
    /// # Arguments
    /// * `related_port` - The related port number
    pub fn related_port(mut self, related_port: u16) -> Self {
        self.related_port = Some(related_port);
        self
    }

    /// Builds and validates the candidate.
    ///
    /// # Returns
    /// * `Ok(Candidate)` - If the candidate is valid
    /// * `Err(IceError)` - If required fields are missing or validation fails
    pub fn build(self) -> Result<Candidate, IceError> {
        let foundation = self
            .foundation
            .ok_or(IceError::MissingRequiredField("foundation"))?;

        let address = self
            .address
            .ok_or(IceError::MissingRequiredField("address"))?;

        let port = self.port.ok_or(IceError::MissingRequiredField("port"))?;

        // Calculate priority if not provided
        let priority = self.priority.unwrap_or_else(|| {
            let type_pref = match self.candidate_type {
                CandidateType::Host => 126,
                CandidateType::Prflx => 110,
                CandidateType::Srflx => 100,
                CandidateType::Relay => 0,
            };
            Candidate::calculate_priority(type_pref, 65535, self.component_id)
        });

        let candidate = Candidate {
            foundation,
            component_id: self.component_id,
            transport: self.transport,
            priority,
            address,
            port,
            candidate_type: self.candidate_type,
            related_address: self.related_address,
            related_port: self.related_port,
        };

        candidate.validate()?;
        Ok(candidate)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_new_builder_has_default_values() {
        let builder = CandidateBuilder::new();

        assert!(builder.foundation.is_none());
        assert_eq!(builder.component_id, 1);
        assert_eq!(builder.transport, "UDP");
        assert!(builder.priority.is_none());
        assert!(builder.address.is_none());
        assert!(builder.port.is_none());
        assert!(matches!(builder.candidate_type, CandidateType::Host));
        assert!(builder.related_address.is_none());
        assert!(builder.related_port.is_none());
    }

    #[test]
    fn test_build_success_with_all_required_fields() {
        let candidate = CandidateBuilder::new()
            .foundation("1")
            .address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .port(8080)
            .build();

        assert!(candidate.is_ok());
        let candidate = candidate.unwrap();
        assert_eq!(candidate.foundation, "1");
        assert_eq!(candidate.port, 8080);
    }

    #[test]
    fn test_build_fails_without_foundation() {
        let result = CandidateBuilder::new()
            .address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .port(8080)
            .build();

        assert!(result.is_err());
        match result {
            Err(IceError::MissingRequiredField("foundation")) => (),
            _ => panic!("Expected MissingRequiredField error for foundation"),
        }
    }

    #[test]
    fn test_build_fails_without_address() {
        let result = CandidateBuilder::new().foundation("1").port(8080).build();

        assert!(result.is_err());
        match result {
            Err(IceError::MissingRequiredField("address")) => (),
            _ => panic!("Expected MissingRequiredField error for address"),
        }
    }

    #[test]
    fn test_build_fails_without_port() {
        let result = CandidateBuilder::new()
            .foundation("1")
            .address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .build();

        assert!(result.is_err());
        match result {
            Err(IceError::MissingRequiredField("port")) => (),
            _ => panic!("Expected MissingRequiredField error for port"),
        }
    }

    #[test]
    fn test_foundation_setter() {
        let builder = CandidateBuilder::new().foundation("test_foundation");
        assert_eq!(builder.foundation, Some("test_foundation".to_string()));
    }

    #[test]
    fn test_foundation_accepts_string() {
        let builder = CandidateBuilder::new().foundation(String::from("dynamic"));
        assert_eq!(builder.foundation, Some("dynamic".to_string()));
    }

    #[test]
    fn test_component_id_setter() {
        let builder = CandidateBuilder::new().component_id(2);
        assert_eq!(builder.component_id, 2);
    }

    #[test]
    fn test_transport_setter() {
        let builder = CandidateBuilder::new().transport("TCP");
        assert_eq!(builder.transport, "TCP");
    }

    #[test]
    fn test_priority_setter() {
        let builder = CandidateBuilder::new().priority(12345);
        assert_eq!(builder.priority, Some(12345));
    }

    #[test]
    fn test_address_setter_ipv4() {
        let ip = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
        let builder = CandidateBuilder::new().address(ip);
        assert_eq!(builder.address, Some(ip));
    }

    #[test]
    fn test_address_setter_ipv6() {
        let ip = IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1));
        let builder = CandidateBuilder::new().address(ip);
        assert_eq!(builder.address, Some(ip));
    }

    #[test]
    fn test_port_setter() {
        let builder = CandidateBuilder::new().port(9000);
        assert_eq!(builder.port, Some(9000));
    }

    #[test]
    fn test_candidate_type_setter() {
        let builder = CandidateBuilder::new().candidate_type(CandidateType::Srflx);
        assert!(matches!(builder.candidate_type, CandidateType::Srflx));
    }

    #[test]
    fn test_related_address_setter() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let builder = CandidateBuilder::new().related_address(ip);
        assert_eq!(builder.related_address, Some(ip));
    }

    #[test]
    fn test_related_port_setter() {
        let builder = CandidateBuilder::new().related_port(5000);
        assert_eq!(builder.related_port, Some(5000));
    }

    #[test]
    fn test_priority_calculation_for_host_candidate() {
        let candidate = CandidateBuilder::new()
            .foundation("1")
            .address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .port(8080)
            .candidate_type(CandidateType::Host)
            .component_id(1)
            .build()
            .unwrap();

        let expected_priority = Candidate::calculate_priority(126, 65535, 1);
        assert_eq!(candidate.priority, expected_priority);
    }

    #[test]
    fn test_priority_calculation_for_srflx_candidate() {
        let candidate = CandidateBuilder::new()
            .foundation("2")
            .address(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4)))
            .port(8080)
            .candidate_type(CandidateType::Srflx)
            .build()
            .unwrap();

        let expected_priority = Candidate::calculate_priority(100, 65535, 1);
        assert_eq!(candidate.priority, expected_priority);
    }

    #[test]
    fn test_priority_calculation_for_prflx_candidate() {
        let candidate = CandidateBuilder::new()
            .foundation("3")
            .address(IpAddr::V4(Ipv4Addr::new(5, 6, 7, 8)))
            .port(8080)
            .candidate_type(CandidateType::Prflx)
            .build()
            .unwrap();

        let expected_priority = Candidate::calculate_priority(110, 65535, 1);
        assert_eq!(candidate.priority, expected_priority);
    }

    #[test]
    fn test_priority_calculation_for_relay_candidate() {
        let candidate = CandidateBuilder::new()
            .foundation("4")
            .address(IpAddr::V4(Ipv4Addr::new(9, 10, 11, 12)))
            .port(8080)
            .candidate_type(CandidateType::Relay)
            .build()
            .unwrap();

        let expected_priority = Candidate::calculate_priority(0, 65535, 1);
        assert_eq!(candidate.priority, expected_priority);
    }

    #[test]
    fn test_manual_priority_overrides_calculation() {
        let custom_priority = 99999;
        let candidate = CandidateBuilder::new()
            .foundation("1")
            .address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .port(8080)
            .priority(custom_priority)
            .build()
            .unwrap();

        assert_eq!(candidate.priority, custom_priority);
    }

    #[test]
    fn test_builder_method_chaining() {
        let candidate = CandidateBuilder::new()
            .foundation("chain_test")
            .component_id(2)
            .transport("TCP")
            .priority(50000)
            .address(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)))
            .port(3478)
            .candidate_type(CandidateType::Relay)
            .related_address(IpAddr::V4(Ipv4Addr::new(192, 168, 0, 1)))
            .related_port(8080)
            .build();

        assert!(candidate.is_ok());
        let candidate = candidate.unwrap();
        assert_eq!(candidate.foundation, "chain_test");
        assert_eq!(candidate.component_id, 2);
        assert_eq!(candidate.transport, "TCP");
        assert_eq!(candidate.priority, 50000);
        assert_eq!(candidate.port, 3478);
        assert!(matches!(candidate.candidate_type, CandidateType::Relay));
        assert!(candidate.related_address.is_some());
        assert_eq!(candidate.related_port, Some(8080));
    }

    #[test]
    fn test_component_id_affects_priority_calculation() {
        let candidate1 = CandidateBuilder::new()
            .foundation("1")
            .address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .port(8080)
            .component_id(1)
            .build()
            .unwrap();

        let candidate2 = CandidateBuilder::new()
            .foundation("2")
            .address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)))
            .port(8080)
            .component_id(2)
            .build()
            .unwrap();

        assert_ne!(candidate1.priority, candidate2.priority);
    }

    #[test]
    fn test_build_with_minimal_host_candidate() {
        let candidate = CandidateBuilder::new()
            .foundation("minimal")
            .address(IpAddr::V4(Ipv4Addr::LOCALHOST))
            .port(1234)
            .build();

        assert!(candidate.is_ok());
    }

    #[test]
    fn test_build_with_complete_relay_candidate() {
        let candidate = CandidateBuilder::new()
            .foundation("relay_complete")
            .component_id(1)
            .transport("UDP")
            .priority(16777215)
            .address(IpAddr::V4(Ipv4Addr::new(203, 0, 113, 1)))
            .port(54321)
            .candidate_type(CandidateType::Relay)
            .related_address(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)))
            .related_port(12345)
            .build();

        assert!(candidate.is_ok());
        let candidate = candidate.unwrap();
        assert!(matches!(candidate.candidate_type, CandidateType::Relay));
        assert_eq!(
            candidate.related_address,
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)))
        );
        assert_eq!(candidate.related_port, Some(12345));
    }
}
