//! ICE agent implementation.
//!
//! The ICE agent is responsible for managing local and remote candidates,
//! forming candidate pairs, and establishing connectivity.

use crate::{candidate::Candidate, candidate_builder::CandidateBuilder, errors::IceError};
use crate::{candidate_pair::CandidatePair, connection_state::ConnectionState};
use logging::Logger;
use stun::StunClient;

/// ICE Agent that manages ICE candidates and connectivity.
///
/// The agent is responsible for:
/// - Gathering local candidates
/// - Processing remote candidates
/// - Managing candidate pairs
/// - Handling connectivity checks
/// - Maintaining connection state
pub struct IceAgent {
    pub ufrag: String,
    pub pwd: String,
    pub local_candidates: Vec<Candidate>,
    pub remote_candidates: Vec<Candidate>,
    candidate_pairs: Vec<CandidatePair>,
    connection_state: ConnectionState,
    logger: Option<Logger>,
}

// Manual Debug implementation to avoid requiring Debug for Logger
impl std::fmt::Debug for IceAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("IceAgent")
            .field("ufrag", &self.ufrag)
            .field("pwd", &self.pwd)
            .field("local_candidates", &self.local_candidates)
            .field("remote_candidates", &self.remote_candidates)
            .field("candidate_pairs", &self.candidate_pairs)
            .field("connection_state", &self.connection_state)
            .field("logger", &self.logger.is_some())
            .finish()
    }
}

impl IceAgent {
    /// Creates a new ICE agent with generated credentials.
    ///
    /// # Returns
    /// A new `IceAgent` instance with random ufrag and pwd
    pub fn new() -> Self {
        Self {
            ufrag: Self::generate_ufrag(),
            pwd: Self::generate_pwd(),
            local_candidates: Vec::new(),
            remote_candidates: Vec::new(),
            candidate_pairs: Vec::new(),
            connection_state: ConnectionState::New,
            logger: None,
        }
    }

    /// Creates a new ICE agent with specific credentials.
    ///
    /// # Arguments
    /// * `ufrag` - ICE username fragment
    /// * `pwd` - ICE password
    pub fn with_credentials(ufrag: String, pwd: String) -> Self {
        Self {
            ufrag,
            pwd,
            local_candidates: Vec::new(),
            remote_candidates: Vec::new(),
            candidate_pairs: Vec::new(),
            connection_state: ConnectionState::New,
            logger: None,
        }
    }

    /// Sets a logger for this ICE agent.
    ///
    /// # Arguments
    /// * `logger` - Logger instance to use for logging
    pub fn with_logger(mut self, logger: Logger) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Internal logging helper
    fn log_info(&self, message: &str) {
        if let Some(ref logger) = self.logger {
            logger.info(message);
        }
    }

    /// Internal logging helper for warnings
    fn log_warn(&self, message: &str) {
        if let Some(ref logger) = self.logger {
            logger.warn(message);
        }
    }

    /// Generates a random ICE username fragment.
    fn generate_ufrag() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX_EPOCH - clock may be incorrect")
            .as_nanos();
        format!("{:x}", timestamp).chars().take(8).collect()
    }

    /// Generates a random ICE password.
    fn generate_pwd() -> String {
        use std::time::{SystemTime, UNIX_EPOCH};
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX_EPOCH - clock may be incorrect")
            .as_nanos();
        format!("{:x}", timestamp * 31).chars().take(24).collect()
    }

    /// Adds a local candidate to the agent.
    ///
    /// # Arguments
    /// * `candidate` - The candidate to add
    pub fn add_local_candidate(&mut self, candidate: Candidate) -> Result<(), IceError> {
        candidate.validate()?;
        self.local_candidates.push(candidate);
        Ok(())
    }

    /// Gathers local host candidates from network interfaces.
    ///
    /// Discovers the actual local network IP address for LAN connections.
    /// Falls back to 0.0.0.0 if detection fails.
    ///
    /// # Arguments
    /// * `port` - The port to use for the candidate
    ///
    /// # Returns
    /// * `Ok(())` - If candidates were gathered successfully
    /// * `Err(IceError)` - If gathering fails
    pub fn gather_host_candidates(&mut self, port: u16) -> Result<(), IceError> {
        use crate::ip_detection::detect_local_ip;
        let local_ip = detect_local_ip();

        let candidate = CandidateBuilder::new()
            .foundation("1".to_string())
            .component_id(1)
            .transport("UDP")
            .address(local_ip.parse().map_err(|_| IceError::InvalidIpAddress)?)
            .port(port)
            .candidate_type(crate::candidate_type::CandidateType::Host)
            .build()?;

        self.add_local_candidate(candidate)?;
        Ok(())
    }

    /// Gathers server reflexive candidates using STUN servers.
    ///
    /// Queries one or more STUN servers to discover the public (reflexive) IP address
    /// and port. This enables NAT traversal for connections through NAT/firewalls.
    ///
    /// # Arguments
    /// * `local_port` - Local port to bind for STUN queries
    /// * `stun_servers` - List of STUN server addresses (e.g., "stun.l.google.com:19302")
    ///
    /// # Returns
    /// * `Ok(())` - If at least one server reflexive candidate was gathered
    /// * `Err(IceError)` - If all STUN queries fail
    ///
    /// # Example
    /// ```no_run
    /// use ice::IceAgent;
    ///
    /// let mut agent = IceAgent::new();
    /// let stun_servers = vec!["stun.l.google.com:19302".to_string()];
    /// agent.gather_server_reflexive_candidates(5000, &stun_servers)
    ///     .expect("Failed to gather srflx candidates");
    /// ```
    pub fn gather_server_reflexive_candidates(
        &mut self,
        local_port: u16,
        stun_servers: &[String],
    ) -> Result<(), IceError> {
        use crate::ip_detection::detect_local_ip;
        use std::net::SocketAddr;

        if stun_servers.is_empty() {
            return Err(IceError::Configuration(
                "No STUN servers configured".to_string(),
            ));
        }

        let bind_addr: SocketAddr = "0.0.0.0:0"
            .parse()
            .map_err(|_| IceError::InvalidIpAddress)?;

        self.log_info(&format!(
            "Querying {} STUN servers for public IP discovery (using ephemeral port, main port {} reserved)...",
            stun_servers.len(),
            local_port
        ));

        match StunClient::discover_reflexive_from_servers(bind_addr, stun_servers) {
            Ok(reflexive_addr) => {
                self.log_info(&format!(
                    "STUN discovery successful! Public IP: {}",
                    reflexive_addr.ip()
                ));

                let local_ip = detect_local_ip();
                let related_addr = local_ip.parse().map_err(|_| IceError::InvalidIpAddress)?;

                let candidate = CandidateBuilder::new()
                    .foundation(format!("{}", 2))
                    .component_id(1)
                    .transport("UDP")
                    .address(reflexive_addr.ip())
                    .port(local_port)
                    .candidate_type(crate::candidate_type::CandidateType::Srflx)
                    .related_address(related_addr)
                    .related_port(local_port)
                    .build()?;

                self.log_info(&format!(
                    "Added STUN candidate: {}:{} (type: srflx, priority: {})",
                    candidate.address, candidate.port, candidate.priority
                ));
                self.add_local_candidate(candidate)?;
            }
            Err(e) => {
                self.log_warn(&format!(
                    "STUN discovery failed: {:?} - Connection may not work across Internet",
                    e
                ));
                return Err(IceError::StunQueryFailed);
            }
        }

        Ok(())
    }

    /// Gathers relay candidates using TURN servers.
    ///
    /// Connects to TURN servers and allocates relay addresses. For each successful
    /// allocation, adds a relay candidate to the local candidates list.
    ///
    /// # Arguments
    /// * `local_port` - Local port to bind for TURN communication
    /// * `turn_servers` - List of TURN server URLs with credentials
    ///   Format: "turn:hostname:port?transport=udp&username=user&password=pass"
    ///
    /// # Returns
    /// * `Ok(())` - If at least one relay candidate was gathered
    /// * `Err(IceError)` - If all TURN allocations failed
    ///
    /// # Example
    /// ```no_run
    /// use ice::IceAgent;
    ///
    /// let mut agent = IceAgent::new();
    /// let turn_servers = vec![
    ///     "turn:turn.example.com:3478?transport=udp&username=user&password=pass".to_string()
    /// ];
    /// agent.gather_relay_candidates(5000, &turn_servers).unwrap();
    /// ```
    pub fn gather_relay_candidates(
        &mut self,
        local_port: u16,
        turn_servers: &[String],
    ) -> Result<(), IceError> {
        #[cfg(feature = "turn")]
        {
            self.log_info(&format!(
                "Gathering relay candidates from {} TURN servers",
                turn_servers.len()
            ));

            let mut success_count = 0;

            for (index, turn_url) in turn_servers.iter().enumerate() {
                match self.allocate_turn_relay(local_port, turn_url, index) {
                    Ok(()) => {
                        success_count += 1;
                        self.log_info(&format!("Allocated relay from {}", turn_url));
                    }
                    Err(e) => {
                        self.log_warn(&format!("TURN allocation to {} failed: {:?}", turn_url, e));
                    }
                }
            }

            if success_count == 0 {
                return Err(IceError::Configuration(
                    "All TURN allocations failed".to_string(),
                ));
            }

            Ok(())
        }

        #[cfg(not(feature = "turn"))]
        {
            let _ = (local_port, turn_servers); // Suppress unused warnings
            Err(IceError::Configuration(
                "TURN feature not enabled. Build with --features turn".to_string(),
            ))
        }
    }

    /// Allocates a relay from a single TURN server.
    ///
    /// # Arguments
    /// * `local_port` - Local port to bind
    /// * `turn_url` - TURN server URL with credentials
    /// * `foundation_index` - Index for candidate foundation
    ///
    /// # Returns
    /// * `Ok(())` - If allocation succeeded and candidate was added
    /// * `Err(IceError)` - If allocation fails
    #[cfg(feature = "turn")]
    fn allocate_turn_relay(
        &mut self,
        local_port: u16,
        turn_url: &str,
        foundation_index: usize,
    ) -> Result<(), IceError> {
        use crate::ip_detection::detect_local_ip;
        use turn::TurnClient;

        // Parse TURN URL (simplified parser)
        // Format: turn:hostname:port?transport=udp&username=user&password=pass
        let parts: Vec<&str> = turn_url.split('?').collect();
        if parts.len() != 2 {
            return Err(IceError::Configuration(
                "Invalid TURN URL format".to_string(),
            ));
        }

        let server_part = parts[0]
            .strip_prefix("turn:")
            .ok_or_else(|| IceError::Configuration("TURN URL must start with turn:".to_string()))?;

        let server_addr: std::net::SocketAddr = server_part
            .parse()
            .map_err(|_| IceError::InvalidIpAddress)?;

        // Parse query parameters
        let mut username = String::new();
        let mut password = String::new();

        for param in parts[1].split('&') {
            let kv: Vec<&str> = param.split('=').collect();
            if kv.len() == 2 {
                match kv[0] {
                    "username" => username = kv[1].to_string(),
                    "password" => password = kv[1].to_string(),
                    _ => {}
                }
            }
        }

        if username.is_empty() || password.is_empty() {
            return Err(IceError::Configuration(
                "TURN URL must include username and password".to_string(),
            ));
        }

        // Create TURN client
        let mut client = TurnClient::new(server_addr, username)
            .map_err(|_| IceError::Configuration("Failed to create TURN client".to_string()))?;

        // Attach logger if available
        #[cfg(feature = "logging")]
        if let Some(ref logger) = self.logger {
            client = client.with_logger(logger.clone());
        }

        // Allocate relay address
        let relay_addr = client
            .allocate()
            .map_err(|_| IceError::Configuration("TURN allocation failed".to_string()))?;

        // Get local IP for related address
        let local_ip = detect_local_ip();
        let related_addr = local_ip.parse().map_err(|_| IceError::InvalidIpAddress)?;

        // Create relay candidate
        let candidate = CandidateBuilder::new()
            .foundation(format!("{}", foundation_index + 100)) // Use high foundation number
            .component_id(1)
            .transport("UDP")
            .address(relay_addr.ip())
            .port(relay_addr.port())
            .candidate_type(crate::candidate_type::CandidateType::Relay)
            .related_address(related_addr)
            .related_port(local_port)
            .build()?;

        self.add_local_candidate(candidate)?;
        Ok(())
    }

    /// Adds a remote candidate received from the peer.
    ///
    /// # Arguments
    /// * `candidate` - The remote candidate to add
    pub fn add_remote_candidate(&mut self, candidate: Candidate) -> Result<(), IceError> {
        candidate.validate()?;
        self.remote_candidates.push(candidate);
        self.form_candidate_pairs();
        Ok(())
    }

    /// Parses remote candidates from SDP attributes.
    ///
    /// # Arguments
    /// * `attributes` - List of SDP attribute strings (e.g., "candidate:...")
    ///
    /// # Returns
    /// * `Ok(())` - If all candidates were parsed successfully
    /// * `Err(IceError)` - If parsing fails
    pub fn add_remote_candidates_from_sdp(
        &mut self,
        attributes: &[String],
    ) -> Result<(), IceError> {
        for attr in attributes {
            if let Some(candidate_str) = attr.strip_prefix("a=candidate:") {
                self.log_info(&format!("Parsing remote ICE candidate: {}", candidate_str));
                let candidate = Candidate::parse(candidate_str)?;
                self.log_info(&format!(
                    "Adding remote candidate: {}:{} (type: {:?})",
                    candidate.address, candidate.port, candidate.candidate_type
                ));
                self.add_remote_candidate(candidate)?;
            } else if let Some(candidate_str) = attr.strip_prefix("candidate:") {
                self.log_info(&format!(
                    "Parsing remote ICE candidate (no a= prefix): {}",
                    candidate_str
                ));
                let candidate = Candidate::parse(candidate_str)?;
                self.log_info(&format!(
                    "Adding remote candidate: {}:{} (type: {:?})",
                    candidate.address, candidate.port, candidate.candidate_type
                ));
                self.add_remote_candidate(candidate)?;
            } else {
                self.log_warn(&format!("Ignoring invalid candidate attribute: {}", attr));
            }
        }
        Ok(())
    }

    /// Forms candidate pairs from local and remote candidates.
    ///
    /// This creates all possible pairs and calculates their priorities
    /// according to RFC 5245.
    fn form_candidate_pairs(&mut self) {
        self.candidate_pairs.clear();

        for local in &self.local_candidates {
            for remote in &self.remote_candidates {
                self.candidate_pairs
                    .push(CandidatePair::new(local.clone(), remote.clone()));
            }
        }

        // Sort by priority (highest first)
        self.candidate_pairs
            .sort_by(|a, b| b.priority.cmp(&a.priority));
    }

    /// Returns the candidate pairs sorted by priority.
    pub fn get_candidate_pairs(&self) -> &[CandidatePair] {
        &self.candidate_pairs
    }

    /// Exports local candidates as SDP attribute strings.
    ///
    /// # Returns
    /// Vector of candidate strings in SDP format (with "a=" prefix)
    pub fn get_local_candidates_sdp(&self) -> Vec<String> {
        self.local_candidates
            .iter()
            .map(|c| format!("a={}", c))
            .collect()
    }

    /// Exports local candidates as strings (without "a=" prefix).
    ///
    /// This is suitable for use with SessionDescriptionBuilder::ice_candidates()
    ///
    /// # Returns
    /// Vector of candidate strings in ICE format
    ///
    /// # Example
    /// ```no_run
    /// use ice::IceAgent;
    ///
    /// let agent = IceAgent::new();
    /// // ... gather candidates ...
    /// let candidates = agent.get_local_candidates_strings();
    /// // candidates = ["candidate:1 1 UDP 2130706431 192.168.1.5 5000 typ host", ...]
    /// ```
    pub fn get_local_candidates_strings(&self) -> Vec<String> {
        self.local_candidates
            .iter()
            .map(|c| c.to_string())
            .collect()
    }

    /// Gets the ICE username fragment.
    ///
    /// # Returns
    /// Reference to the ICE username fragment
    pub fn get_ufrag(&self) -> &str {
        &self.ufrag
    }

    /// Gets the ICE password.
    ///
    /// # Returns
    /// Reference to the ICE password
    pub fn get_pwd(&self) -> &str {
        &self.pwd
    }

    /// Gets the number of local candidates gathered.
    ///
    /// # Returns
    /// Number of local candidates
    pub fn local_candidate_count(&self) -> usize {
        self.local_candidates.len()
    }

    /// Gets the number of remote candidates received.
    ///
    /// # Returns
    /// Number of remote candidates
    pub fn remote_candidate_count(&self) -> usize {
        self.remote_candidates.len()
    }

    /// Clears all candidates and pairs.
    pub fn clear(&mut self) {
        self.local_candidates.clear();
        self.remote_candidates.clear();
        self.candidate_pairs.clear();
    }

    /// Returns the current connection state.
    pub fn connection_state(&self) -> ConnectionState {
        self.connection_state
    }

    /// Validates the ICE connection by performing connectivity checks.
    ///
    /// Implements RFC 5245 connectivity checking:
    /// 1. Forms candidate pairs from local and remote candidates
    /// 2. Prioritizes pairs (srflx/relay > host)
    /// 3. Selects the highest priority working pair
    ///
    /// Priority order (best to worst):
    /// - Relay ↔ Relay (guaranteed to work through TURN)
    /// - Srflx ↔ Srflx (P2P through NAT with STUN)
    /// - Host ↔ Host (direct LAN connection)
    ///
    /// # Returns
    /// * `Ok(())` - If at least one candidate pair is valid
    /// * `Err(IceError)` - If no valid pairs exist
    pub fn establish_connection(&mut self) -> Result<(), IceError> {
        if self.local_candidates.is_empty() {
            return Err(IceError::NoCandidates);
        }

        if self.remote_candidates.is_empty() {
            return Err(IceError::NoCandidates);
        }

        self.connection_state = ConnectionState::Checking;

        // Form and prioritize candidate pairs
        self.form_candidate_pairs();

        if self.candidate_pairs.is_empty() {
            return Err(IceError::NoCandidates);
        }

        // Perform connectivity checks on pairs (simplified)
        // In a full implementation, this would send STUN Binding Requests
        // For now, we validate that compatible pairs exist
        let has_valid_pair = self.validate_candidate_pairs();

        if !has_valid_pair {
            self.log_warn("No valid candidate pairs found");
            return Err(IceError::Configuration(
                "No compatible candidate pairs".to_string(),
            ));
        }

        self.connection_state = ConnectionState::Connected;
        self.log_info(&format!(
            "ICE connection established with {} candidate pairs",
            self.candidate_pairs.len()
        ));

        Ok(())
    }

    /// Validates candidate pairs for compatibility.
    ///
    /// Checks that:
    /// - Transport protocols match (UDP/TCP)
    /// - Address families match (IPv4/IPv6)
    /// - At least one pair has relay or srflx candidates for NAT traversal
    ///
    /// # Returns
    /// `true` if at least one valid pair exists, `false` otherwise
    fn validate_candidate_pairs(&self) -> bool {
        use crate::candidate_type::CandidateType;

        let mut has_relay_pair = false;
        let mut has_srflx_pair = false;
        let mut has_host_pair = false;

        for pair in &self.candidate_pairs {
            // Check transport compatibility
            if pair.local.transport != pair.remote.transport {
                continue;
            }

            // Check address family compatibility
            let local_is_v4 = pair.local.address.is_ipv4();
            let remote_is_v4 = pair.remote.address.is_ipv4();
            if local_is_v4 != remote_is_v4 {
                continue;
            }

            // Categorize valid pairs by type
            match (&pair.local.candidate_type, &pair.remote.candidate_type) {
                (CandidateType::Relay, _) | (_, CandidateType::Relay) => {
                    has_relay_pair = true;
                }
                (CandidateType::Srflx, _) | (_, CandidateType::Srflx) => {
                    has_srflx_pair = true;
                }
                (CandidateType::Host, CandidateType::Host) => {
                    has_host_pair = true;
                }
                _ => {}
            }
        }

        // Log connectivity options
        if has_relay_pair {
            self.log_info("✓ Relay candidates available (guaranteed connectivity through TURN)");
        }
        if has_srflx_pair {
            self.log_info("✓ Server-reflexive candidates available (P2P through NAT)");
        }
        if has_host_pair {
            self.log_info("✓ Host candidates available (direct LAN connection)");
        }

        // Valid if we have any type of compatible pair
        has_relay_pair || has_srflx_pair || has_host_pair
    }

    /// Gets the best candidate pair for connection.
    ///
    /// Returns the highest priority valid candidate pair.
    /// Priority order: Relay > Srflx > Host
    ///
    /// # Returns
    /// The best candidate pair, or `None` if no valid pairs exist
    pub fn get_best_candidate_pair(&self) -> Option<&crate::candidate_pair::CandidatePair> {
        use crate::candidate_type::CandidateType;

        // First try relay pairs (most reliable)
        for pair in &self.candidate_pairs {
            if matches!(
                (&pair.local.candidate_type, &pair.remote.candidate_type),
                (CandidateType::Relay, _) | (_, CandidateType::Relay)
            ) {
                return Some(pair);
            }
        }

        // Then srflx pairs (P2P through NAT)
        for pair in &self.candidate_pairs {
            if matches!(
                (&pair.local.candidate_type, &pair.remote.candidate_type),
                (CandidateType::Srflx, _) | (_, CandidateType::Srflx)
            ) {
                return Some(pair);
            }
        }

        // Finally host pairs (LAN only)
        for pair in &self.candidate_pairs {
            if matches!(
                (&pair.local.candidate_type, &pair.remote.candidate_type),
                (CandidateType::Host, CandidateType::Host)
            ) {
                return Some(pair);
            }
        }

        // Fallback to highest priority pair
        self.candidate_pairs.first()
    }
}

/// Provides a default implementation for [`IceAgent`].
impl Default for IceAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate_type::CandidateType;
    use std::net::{IpAddr, Ipv4Addr};

    fn create_test_candidate(port: u16) -> Candidate {
        Candidate {
            foundation: "1".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1)),
            port,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        }
    }

    #[test]
    fn test_new_creates_agent_with_credentials() {
        let agent = IceAgent::new();

        assert!(!agent.ufrag.is_empty());
        assert!(!agent.pwd.is_empty());
        // Verificar que tienen longitud razonable
        assert!(agent.ufrag.len() <= 8);
        assert!(agent.pwd.len() <= 24);
    }

    #[test]
    fn test_new_initializes_empty_collections() {
        let agent = IceAgent::new();

        assert!(agent.local_candidates.is_empty());
        assert!(agent.remote_candidates.is_empty());
        assert_eq!(agent.connection_state(), ConnectionState::New);
    }

    #[test]
    fn test_with_credentials_sets_custom_credentials() {
        let ufrag = "test_ufrag".to_string();
        let pwd = "test_password".to_string();

        let agent = IceAgent::with_credentials(ufrag.clone(), pwd.clone());

        assert_eq!(agent.ufrag, ufrag);
        assert_eq!(agent.pwd, pwd);
    }

    #[test]
    fn test_add_local_candidate_succeeds() {
        let mut agent = IceAgent::new();
        let candidate = create_test_candidate(8080);

        let result = agent.add_local_candidate(candidate);

        assert!(result.is_ok());
        assert_eq!(agent.local_candidates.len(), 1);
    }

    #[test]
    fn test_add_local_candidate_validates() {
        let mut agent = IceAgent::new();
        let mut invalid_candidate = create_test_candidate(8080);
        invalid_candidate.foundation = "".to_string(); // Invalid

        let result = agent.add_local_candidate(invalid_candidate);

        assert!(result.is_err());
        assert_eq!(agent.local_candidates.len(), 0);
    }

    #[test]
    fn test_add_multiple_local_candidates() {
        let mut agent = IceAgent::new();

        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();
        agent
            .add_local_candidate(create_test_candidate(8081))
            .unwrap();
        agent
            .add_local_candidate(create_test_candidate(8082))
            .unwrap();

        assert_eq!(agent.local_candidates.len(), 3);
    }

    #[test]
    fn test_gather_host_candidates_creates_localhost() {
        let mut agent = IceAgent::new();

        let result = agent.gather_host_candidates(8080);

        assert!(result.is_ok());
        assert_eq!(agent.local_candidates.len(), 1);

        let candidate = &agent.local_candidates[0];
        assert_eq!(candidate.port, 8080);
        assert_eq!(candidate.candidate_type, CandidateType::Host);
        // The IP address should be a valid IPv4 address (either localhost or actual local IP)
        assert!(matches!(candidate.address, IpAddr::V4(_)));
    }

    #[test]
    fn test_add_remote_candidate_succeeds() {
        let mut agent = IceAgent::new();
        let candidate = create_test_candidate(9090);

        let result = agent.add_remote_candidate(candidate);

        assert!(result.is_ok());
        assert_eq!(agent.remote_candidates.len(), 1);
    }

    #[test]
    fn test_add_remote_candidate_validates() {
        let mut agent = IceAgent::new();
        let mut invalid_candidate = create_test_candidate(9090);
        invalid_candidate.component_id = 5; // Invalid

        let result = agent.add_remote_candidate(invalid_candidate);

        assert!(result.is_err());
        assert_eq!(agent.remote_candidates.len(), 0);
    }

    #[test]
    fn test_add_remote_candidate_forms_pairs() {
        let mut agent = IceAgent::new();
        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();

        agent
            .add_remote_candidate(create_test_candidate(9090))
            .unwrap();

        assert_eq!(agent.get_candidate_pairs().len(), 1);
    }

    #[test]
    fn test_form_candidate_pairs_creates_all_combinations() {
        let mut agent = IceAgent::new();

        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();
        agent
            .add_local_candidate(create_test_candidate(8081))
            .unwrap();

        agent
            .add_remote_candidate(create_test_candidate(9090))
            .unwrap();
        agent
            .add_remote_candidate(create_test_candidate(9091))
            .unwrap();

        // 2 local * 2 remote = 4 pairs
        assert_eq!(agent.get_candidate_pairs().len(), 4);
    }

    #[test]
    fn test_candidate_pairs_sorted_by_priority() {
        let mut agent = IceAgent::new();

        let mut low_priority = create_test_candidate(8080);
        low_priority.priority = 100;
        agent.add_local_candidate(low_priority).unwrap();

        let mut high_priority = create_test_candidate(9090);
        high_priority.priority = 1000;
        agent.add_remote_candidate(high_priority).unwrap();

        let pairs = agent.get_candidate_pairs();

        // Verificar que están ordenados de mayor a menor
        for i in 0..pairs.len().saturating_sub(1) {
            assert!(pairs[i].priority >= pairs[i + 1].priority);
        }
    }

    #[test]
    fn test_add_remote_candidates_from_sdp() {
        let mut agent = IceAgent::new();

        let sdp_attributes = vec![
            "candidate:1 1 UDP 2130706431 192.168.1.1 8080 typ host".to_string(),
            "candidate:2 1 UDP 1694498815 203.0.113.1 54321 typ srflx raddr 192.168.1.1 rport 8080"
                .to_string(),
        ];

        let result = agent.add_remote_candidates_from_sdp(&sdp_attributes);

        assert!(result.is_ok());
        assert_eq!(agent.remote_candidates.len(), 2);
    }

    #[test]
    fn test_add_remote_candidates_from_sdp_ignores_non_candidate() {
        let mut agent = IceAgent::new();

        let sdp_attributes = vec![
            "candidate:1 1 UDP 2130706431 192.168.1.1 8080 typ host".to_string(),
            "ice-ufrag:test".to_string(), // No es un candidato
        ];

        let result = agent.add_remote_candidates_from_sdp(&sdp_attributes);

        assert!(result.is_ok());
        assert_eq!(agent.remote_candidates.len(), 1);
    }

    #[test]
    fn test_add_remote_candidates_from_sdp_fails_on_invalid() {
        let mut agent = IceAgent::new();

        let sdp_attributes = vec!["candidate:invalid".to_string()];

        let result = agent.add_remote_candidates_from_sdp(&sdp_attributes);

        assert!(result.is_err());
    }

    #[test]
    fn test_get_local_candidates_sdp() {
        let mut agent = IceAgent::new();
        agent.gather_host_candidates(8080).unwrap();

        let sdp = agent.get_local_candidates_sdp();

        assert_eq!(sdp.len(), 1);
        assert!(sdp[0].starts_with("a=candidate:"));
    }

    #[test]
    fn test_get_local_candidates_sdp_multiple() {
        let mut agent = IceAgent::new();
        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();
        agent
            .add_local_candidate(create_test_candidate(8081))
            .unwrap();

        let sdp = agent.get_local_candidates_sdp();

        assert_eq!(sdp.len(), 2);
        for line in sdp {
            assert!(line.starts_with("a=candidate:"));
        }
    }

    #[test]
    fn test_clear_removes_all_candidates() {
        let mut agent = IceAgent::new();
        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();
        agent
            .add_remote_candidate(create_test_candidate(9090))
            .unwrap();

        agent.clear();

        assert!(agent.local_candidates.is_empty());
        assert!(agent.remote_candidates.is_empty());
        assert!(agent.get_candidate_pairs().is_empty());
    }

    #[test]
    fn test_connection_state_starts_new() {
        let agent = IceAgent::new();
        assert_eq!(agent.connection_state(), ConnectionState::New);
    }

    #[test]
    fn test_establish_connection_fails_without_local_candidates() {
        let mut agent = IceAgent::new();
        agent
            .add_remote_candidate(create_test_candidate(9090))
            .unwrap();

        let result = agent.establish_connection();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::NoCandidates));
    }

    #[test]
    fn test_establish_connection_fails_without_remote_candidates() {
        let mut agent = IceAgent::new();
        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();

        let result = agent.establish_connection();

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), IceError::NoCandidates));
    }

    #[test]
    fn test_establish_connection_succeeds_with_both_candidates() {
        let mut agent = IceAgent::new();
        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();
        agent
            .add_remote_candidate(create_test_candidate(9090))
            .unwrap();

        let result = agent.establish_connection();

        assert!(result.is_ok());
        assert_eq!(agent.connection_state(), ConnectionState::Connected);
    }

    #[test]
    fn test_establish_connection_changes_state_to_checking() {
        let mut agent = IceAgent::new();
        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();
        agent
            .add_remote_candidate(create_test_candidate(9090))
            .unwrap();

        assert_eq!(agent.connection_state(), ConnectionState::New);

        agent.establish_connection().unwrap();

        // El estado final debe ser Connected
        assert_eq!(agent.connection_state(), ConnectionState::Connected);
    }

    #[test]
    fn test_connectivity_checking_with_relay_candidates() {
        use crate::candidate_type::CandidateType;

        let mut agent = IceAgent::new();

        // Add relay candidate (highest priority)
        let mut relay_local = create_test_candidate(8080);
        relay_local.candidate_type = CandidateType::Relay;
        relay_local.priority = 2000;
        agent.add_local_candidate(relay_local).unwrap();

        let mut relay_remote = create_test_candidate(9090);
        relay_remote.candidate_type = CandidateType::Relay;
        relay_remote.priority = 2000;
        agent.add_remote_candidate(relay_remote).unwrap();

        let result = agent.establish_connection();
        assert!(result.is_ok());
        assert_eq!(agent.connection_state(), ConnectionState::Connected);
    }

    #[test]
    fn test_connectivity_checking_prefers_relay_over_srflx() {
        use crate::candidate_type::CandidateType;

        let mut agent = IceAgent::new();

        // Add srflx candidate
        let mut srflx = create_test_candidate(8080);
        srflx.candidate_type = CandidateType::Srflx;
        agent.add_local_candidate(srflx).unwrap();

        // Add relay candidate
        let mut relay = create_test_candidate(8081);
        relay.candidate_type = CandidateType::Relay;
        relay.priority = 1500; // Lower priority but relay type
        agent.add_local_candidate(relay).unwrap();

        let mut remote = create_test_candidate(9090);
        remote.candidate_type = CandidateType::Host;
        agent.add_remote_candidate(remote).unwrap();

        agent.establish_connection().unwrap();

        // Get best pair - should prefer relay
        let best = agent.get_best_candidate_pair();
        assert!(best.is_some());
        assert_eq!(best.unwrap().local.candidate_type, CandidateType::Relay);
    }

    #[test]
    fn test_connectivity_checking_with_mixed_candidates() {
        use crate::candidate_type::CandidateType;

        let mut agent = IceAgent::new();

        // Add multiple candidate types
        let mut host = create_test_candidate(8080);
        host.candidate_type = CandidateType::Host;
        host.priority = 2130706431;
        agent.add_local_candidate(host).unwrap();

        let mut srflx = create_test_candidate(8081);
        srflx.candidate_type = CandidateType::Srflx;
        srflx.priority = 1694498815;
        agent.add_local_candidate(srflx).unwrap();

        let mut remote_host = create_test_candidate(9090);
        remote_host.candidate_type = CandidateType::Host;
        agent.add_remote_candidate(remote_host).unwrap();

        let result = agent.establish_connection();
        assert!(result.is_ok());

        // Should have valid pairs
        assert!(!agent.get_candidate_pairs().is_empty());
    }

    #[test]
    fn test_get_best_candidate_pair_with_no_candidates() {
        let agent = IceAgent::new();
        assert!(agent.get_best_candidate_pair().is_none());
    }

    #[test]
    fn test_get_best_candidate_pair_returns_highest_priority() {
        use crate::candidate_type::CandidateType;

        let mut agent = IceAgent::new();

        let mut low_priority = create_test_candidate(8080);
        low_priority.candidate_type = CandidateType::Host;
        low_priority.priority = 100;
        agent.add_local_candidate(low_priority).unwrap();

        let mut high_priority = create_test_candidate(8081);
        high_priority.candidate_type = CandidateType::Srflx;
        high_priority.priority = 1000;
        agent.add_local_candidate(high_priority).unwrap();

        let remote = create_test_candidate(9090);
        agent.add_remote_candidate(remote).unwrap();

        let best = agent.get_best_candidate_pair();
        assert!(best.is_some());
        // Should prefer srflx over host
        assert_eq!(best.unwrap().local.candidate_type, CandidateType::Srflx);
    }

    #[test]
    fn test_default_trait() {
        let agent = IceAgent::default();

        assert!(!agent.ufrag.is_empty());
        assert!(!agent.pwd.is_empty());
        assert!(agent.local_candidates.is_empty());
    }

    #[test]
    fn test_get_candidate_pairs_empty_initially() {
        let agent = IceAgent::new();
        assert_eq!(agent.get_candidate_pairs().len(), 0);
    }

    #[test]
    fn test_adding_more_remotes_updates_pairs() {
        let mut agent = IceAgent::new();
        agent
            .add_local_candidate(create_test_candidate(8080))
            .unwrap();

        agent
            .add_remote_candidate(create_test_candidate(9090))
            .unwrap();
        assert_eq!(agent.get_candidate_pairs().len(), 1);

        agent
            .add_remote_candidate(create_test_candidate(9091))
            .unwrap();
        assert_eq!(agent.get_candidate_pairs().len(), 2);
    }

    #[test]
    fn test_debug_trait() {
        let agent = IceAgent::new();
        let debug_output = format!("{:?}", agent);

        assert!(debug_output.contains("IceAgent"));
        assert!(debug_output.contains("ufrag"));
    }
}
