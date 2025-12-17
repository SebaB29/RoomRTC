//! SDP (Session Description Protocol) handling for WebRTC connection

use ice::{IceAgent, detect_local_ip};
use logging::Logger;
use sdp::{Attribute, MediaDescription, Origin, SdpType, SessionDescription, Timing};
use std::error::Error;
use std::time::{SystemTime, UNIX_EPOCH};

/// Handles all SDP-related operations
pub(super) struct SdpHandler {
    logger: Logger,
}

impl SdpHandler {
    pub fn new(logger: Logger) -> Self {
        Self { logger }
    }

    pub fn create_offer(&self, ice_agent: &IceAgent) -> Result<String, Box<dyn Error>> {
        self.logger.info("Creating SDP offer with ICE candidates");
        let sdp = self.build_sdp(SdpType::Offer, ice_agent)?;
        Ok(sdp)
    }

    pub fn create_answer(&self, ice_agent: &IceAgent) -> Result<String, Box<dyn Error>> {
        self.logger.info("Creating SDP answer with ICE candidates");
        let sdp = self.build_sdp(SdpType::Answer, ice_agent)?;
        Ok(sdp)
    }

    pub fn add_fingerprint_to_sdp(sdp: String, fingerprint: &str, is_offerer: bool) -> String {
        let mut lines: Vec<String> = sdp.lines().map(|s| s.to_string()).collect();

        let insert_pos = lines
            .iter()
            .position(|line| line.starts_with("m="))
            .unwrap_or(lines.len());

        lines.insert(insert_pos, format!("a=fingerprint:sha-256 {}", fingerprint));

        // RFC 5763: offerer uses actpass, answerer chooses active or passive
        let setup = if is_offerer {
            "actpass" // Offerer: "I can be either"
        } else {
            "active" // Answerer: "I will be DTLS client"
        };
        lines.insert(insert_pos + 1, format!("a=setup:{}", setup));

        lines.join("\r\n") + "\r\n"
    }

    pub fn extract_fingerprint_from_sdp(sdp: &str) -> Option<String> {
        sdp.lines()
            .find(|line| line.starts_with("a=fingerprint:"))
            .and_then(|line| {
                line.strip_prefix("a=fingerprint:sha-256 ")
                    .or_else(|| line.strip_prefix("a=fingerprint:SHA-256 "))
                    .map(|s| s.trim().to_uppercase())
            })
    }

    fn build_sdp(&self, sdp_type: SdpType, ice_agent: &IceAgent) -> Result<String, Box<dyn Error>> {
        let media = MediaDescription {
            media_type: "application".to_string(),
            port: 9,
            protocol: "DTLS/SCTP".to_string(),
            formats: vec!["webrtc-datachannel".to_string()],
            connection: None,
            attributes: Vec::new(),
        };

        let candidate_ip = ice_agent
            .local_candidates
            .first()
            .map(|c| c.address.to_string());

        let mut builder = SessionDescription::builder(sdp_type)
            .origin(self.make_origin(candidate_ip.as_deref()))
            .session_name("Rust WebRTC")
            .timing(Timing::default())
            .add_media(media)
            .add_attribute(Attribute {
                name: "ice-ufrag".to_string(),
                value: Some(ice_agent.ufrag.clone()),
            })
            .add_attribute(Attribute {
                name: "ice-pwd".to_string(),
                value: Some(ice_agent.pwd.clone()),
            });

        for c in &ice_agent.local_candidates {
            builder = builder.add_attribute(Attribute {
                name: "candidate".to_string(),
                value: Some(format!("{}", c)),
            });
        }

        self.logger.info(&format!(
            "SDP built with {} ICE candidates (host + srflx + relay)",
            ice_agent.local_candidates.len()
        ));

        let sdp = builder.build()?;
        Ok(sdp.to_string())
    }

    fn make_origin(&self, local_candidate_ip: Option<&str>) -> Origin {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(1);

        let local_ip = if let Some(ip) = local_candidate_ip {
            ip.to_string()
        } else {
            detect_local_ip()
        };

        Origin {
            username: "-".to_string(),
            session_id: now,
            session_version: 1,
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            unicast_address: local_ip,
        }
    }
}
