//! ICE candidate management for WebRTC connection

use ice::{Candidate, IceAgent};
use logging::Logger;
use std::error::Error;

/// Handles all ICE-related operations
pub(super) struct IceHandler {
    pub(super) ice_agent: IceAgent,
    stun_servers: Vec<String>,
    turn_servers: Vec<String>,
    logger: Logger,
}

impl IceHandler {
    pub fn new(logger: Logger) -> Self {
        let stun_servers = vec![
            "stun.l.google.com:19302".to_string(),
            "stun1.l.google.com:19302".to_string(),
            "stun2.l.google.com:19302".to_string(),
        ];

        let turn_servers = vec![
            "turn:openrelay.metered.ca:80?transport=udp&username=openrelayproject&password=openrelayproject".to_string(),
            "turn:openrelay.metered.ca:443?transport=tcp&username=openrelayproject&password=openrelayproject".to_string(),
        ];

        logger.info(&format!(
            "ICE configured: {} STUN servers, {} TURN servers for NAT traversal",
            stun_servers.len(),
            turn_servers.len()
        ));

        Self {
            ice_agent: IceAgent::new(),
            stun_servers,
            turn_servers,
            logger,
        }
    }

    pub fn set_stun_servers(&mut self, servers: Vec<String>) {
        self.logger.info(&format!(
            "Configured {} STUN servers for NAT traversal",
            servers.len()
        ));
        self.stun_servers = servers;
    }

    pub fn set_turn_servers(&mut self, servers: Vec<String>) {
        self.logger.info(&format!(
            "Configured {} TURN servers for relay",
            servers.len()
        ));
        self.turn_servers = servers;
    }

    pub fn add_ice_candidate(
        &mut self,
        candidate: &str,
        _sdp_mid: &str,
        _sdp_mline_index: u16,
    ) -> Result<(), Box<dyn Error>> {
        self.logger
            .info(&format!("Adding ICE candidate: {}", candidate));

        let cleaned = candidate.trim_start_matches("candidate:");
        let parsed_candidate = Candidate::parse(cleaned)?;

        self.logger.info(&format!(
            "Remote ICE candidate added: {}:{} (type: {:?}, priority: {})",
            parsed_candidate.address,
            parsed_candidate.port,
            parsed_candidate.candidate_type,
            parsed_candidate.priority
        ));

        self.ice_agent.add_remote_candidate(parsed_candidate)?;

        Ok(())
    }

    pub fn gather_candidates(&mut self, port: u16) -> Result<(), Box<dyn Error>> {
        if self.ice_agent.local_candidates.is_empty() {
            self.ice_agent.gather_host_candidates(port)?;
        }
        self.logger.info("Host candidates gathered");

        self.gather_stun_candidates(port)?;
        self.gather_turn_candidates(port)?;

        Ok(())
    }

    fn gather_stun_candidates(&mut self, port: u16) -> Result<(), Box<dyn Error>> {
        if !self.stun_servers.is_empty() {
            self.logger.info(&format!(
                "Starting STUN candidate gathering from {} servers...",
                self.stun_servers.len()
            ));

            match self
                .ice_agent
                .gather_server_reflexive_candidates(port, &self.stun_servers)
            {
                Ok(_) => {
                    self.logger.info(
                        "STUN candidates gathered successfully - Internet connectivity enabled",
                    );
                }
                Err(e) => {
                    self.logger.error(&format!(
                        "STUN gathering FAILED: {} - Connection limited to local network only!",
                        e
                    ));
                }
            }
        } else {
            self.logger
                .warn("No STUN servers configured - Internet connections may not work");
        }
        Ok(())
    }

    fn gather_turn_candidates(&mut self, port: u16) -> Result<(), Box<dyn Error>> {
        if !self.turn_servers.is_empty() {
            self.logger.info(&format!(
                "Gathering TURN candidates from {} servers...",
                self.turn_servers.len()
            ));
            match self
                .ice_agent
                .gather_relay_candidates(port, &self.turn_servers)
            {
                Ok(_) => self.logger.info("TURN candidates gathered (relay enabled)"),
                Err(e) => self.logger.warn(&format!(
                    "TURN gathering failed: {} (direct/STUN will be tried)",
                    e
                )),
            }
        }
        Ok(())
    }

    pub fn get_remote_address(&self) -> Option<(String, u16)> {
        self.ice_agent
            .remote_candidates
            .first()
            .map(|candidate| (candidate.address.to_string(), candidate.port))
    }

    pub fn extract_remote_endpoint_from_sdp(&self, sdp: &str) -> Option<(String, u16)> {
        sdp.lines()
            .find(|line| line.starts_with("a=candidate:") || line.starts_with("candidate:"))
            .and_then(|line| {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 6 {
                    let ip = parts[4].to_string();
                    let port = parts[5].parse::<u16>().ok()?;
                    Some((ip, port))
                } else {
                    None
                }
            })
    }
}
