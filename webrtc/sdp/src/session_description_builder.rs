//! Builder pattern for SDP session descriptions.
//!
//! Provides a fluent API for constructing SDP session descriptions
//! with validation.

use crate::{
    attribute::Attribute, connection::Connection, errors::SdpError,
    media_description::MediaDescription, origin::Origin, sdp_type::SdpType,
    session_description::SessionDescription, timing::Timing,
};

/// Builder for constructing `SessionDescription` instances.
pub struct SessionDescriptionBuilder {
    session: SessionDescription,
}

impl SessionDescriptionBuilder {
    /// Creates a new builder with the specified SDP type.
    ///
    /// # Arguments
    /// * `sdp_type` - The type of SDP message (Offer or Answer)
    pub fn new(sdp_type: SdpType) -> Self {
        Self {
            session: SessionDescription::new(sdp_type),
        }
    }

    /// Sets the origin field.
    ///
    /// # Arguments
    /// * `origin` - The origin information for the session
    pub fn origin(mut self, origin: Origin) -> Self {
        self.session.origin = origin;
        self
    }

    /// Sets the session name.
    ///
    /// # Arguments
    /// * `name` - The session name (can be any type that converts to String)
    pub fn session_name(mut self, name: impl Into<String>) -> Self {
        self.session.session_name = name.into();
        self
    }

    /// Sets the timing information.
    ///
    /// # Arguments
    /// * `timing` - The timing information for the session
    pub fn timing(mut self, timing: Timing) -> Self {
        self.session.timing = timing;
        self
    }

    /// Sets the connection information.
    ///
    /// # Arguments
    /// * `connection` - The connection information for the session
    pub fn connection(mut self, connection: Connection) -> Self {
        self.session.connection = Some(connection);
        self
    }

    /// Adds a media description to the session.
    ///
    /// # Arguments
    /// * `media` - The media description to add
    pub fn add_media(mut self, media: MediaDescription) -> Self {
        self.session.media.push(media);
        self
    }

    /// Adds an attribute to the session.
    ///
    /// # Arguments
    /// * `attr` - The attribute to add
    pub fn add_attribute(mut self, attr: Attribute) -> Self {
        self.session.attributes.push(attr);
        self
    }

    /// Adds ICE credentials to the session.
    ///
    /// # Arguments
    /// * `ufrag` - ICE username fragment
    /// * `pwd` - ICE password
    ///
    /// # Example
    /// ```no_run
    /// use sdp::{SessionDescriptionBuilder, SdpType};
    ///
    /// let sdp = SessionDescriptionBuilder::new(SdpType::Offer)
    ///     .ice_credentials("abcd1234", "secret_password_here")
    ///     // ... other fields
    ///     .build();
    /// ```
    pub fn ice_credentials(mut self, ufrag: &str, pwd: &str) -> Self {
        self.session.attributes.push(Attribute {
            name: "ice-ufrag".to_string(),
            value: Some(ufrag.to_string()),
        });
        self.session.attributes.push(Attribute {
            name: "ice-pwd".to_string(),
            value: Some(pwd.to_string()),
        });
        self
    }

    /// Adds ICE candidates to the session.
    ///
    /// # Arguments
    /// * `candidates` - Vector of ICE candidate strings in SDP format
    ///
    /// # Example
    /// ```no_run
    /// use sdp::{SessionDescriptionBuilder, SdpType};
    ///
    /// let candidates = vec![
    ///     "candidate:1 1 UDP 2130706431 192.168.1.5 5000 typ host".to_string(),
    ///     "candidate:2 1 UDP 1694498815 203.0.113.10 5000 typ srflx raddr 192.168.1.5 rport 5000".to_string(),
    /// ];
    ///
    /// let sdp = SessionDescriptionBuilder::new(SdpType::Offer)
    ///     .ice_candidates(&candidates)
    ///     // ... other fields
    ///     .build();
    /// ```
    pub fn ice_candidates(mut self, candidates: &[String]) -> Self {
        for candidate in candidates {
            self.session.attributes.push(Attribute {
                name: "candidate".to_string(),
                value: Some(candidate.clone()),
            });
        }
        self
    }

    /// Adds a single ICE candidate to the session.
    ///
    /// # Arguments
    /// * `candidate` - ICE candidate string in SDP format
    pub fn add_ice_candidate(mut self, candidate: &str) -> Self {
        self.session.attributes.push(Attribute {
            name: "candidate".to_string(),
            value: Some(candidate.to_string()),
        });
        self
    }

    /// Builds and validates the `SessionDescription`.
    ///
    /// # Returns
    /// * `Ok(SessionDescription)` - If the session description is valid
    /// * `Err(SdpError)` - If validation fails
    pub fn build(self) -> Result<SessionDescription, SdpError> {
        self.session.validate()?;
        Ok(self.session)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_builder_new() {
        let builder = SessionDescriptionBuilder::new(SdpType::Offer);
        assert_eq!(builder.session.sdp_type, SdpType::Offer);
    }

    #[test]
    fn test_builder_origin() {
        let origin = Origin {
            username: "test".to_string(),
            session_id: 12345,
            session_version: 1,
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            unicast_address: "192.168.1.1".to_string(),
        };

        let session = SessionDescriptionBuilder::new(SdpType::Offer)
            .origin(origin.clone())
            .session_name("Test")
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build()
            .unwrap();

        assert_eq!(session.origin.username, "test");
        assert_eq!(session.origin.session_id, 12345);
    }

    #[test]
    fn test_builder_session_name() {
        let origin = Origin {
            session_id: 1,
            ..Default::default()
        };

        let session = SessionDescriptionBuilder::new(SdpType::Offer)
            .origin(origin)
            .session_name("My Session")
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build()
            .unwrap();

        assert_eq!(session.session_name, "My Session");
    }

    #[test]
    fn test_builder_timing() {
        let origin = Origin {
            session_id: 1,
            ..Default::default()
        };

        let timing = Timing {
            start_time: 100,
            stop_time: 200,
        };

        let session = SessionDescriptionBuilder::new(SdpType::Offer)
            .origin(origin)
            .session_name("Test")
            .timing(timing)
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build()
            .unwrap();

        assert_eq!(session.timing.start_time, 100);
        assert_eq!(session.timing.stop_time, 200);
    }

    #[test]
    fn test_builder_connection() {
        let origin = Origin {
            session_id: 1,
            ..Default::default()
        };

        let connection = Connection {
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            address: "192.168.1.1".parse::<IpAddr>().unwrap(),
            ttl: None,
            num_addresses: None,
        };

        let session = SessionDescriptionBuilder::new(SdpType::Offer)
            .origin(origin)
            .session_name("Test")
            .connection(connection)
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build()
            .unwrap();

        assert!(session.connection.is_some());
        assert_eq!(
            session.connection.unwrap().address,
            "192.168.1.1".parse::<IpAddr>().unwrap()
        );
    }

    #[test]
    fn test_builder_add_media() {
        let origin = Origin {
            session_id: 1,
            ..Default::default()
        };

        let media1 = MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: Vec::new(),
        };

        let media2 = MediaDescription {
            media_type: "video".to_string(),
            port: 51372,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["99".to_string()],
            connection: None,
            attributes: Vec::new(),
        };

        let session = SessionDescriptionBuilder::new(SdpType::Offer)
            .origin(origin)
            .session_name("Test")
            .add_media(media1)
            .add_media(media2)
            .build()
            .unwrap();

        assert_eq!(session.media.len(), 2);
        assert_eq!(session.media[0].media_type, "audio");
        assert_eq!(session.media[1].media_type, "video");
    }

    #[test]
    fn test_builder_add_attribute() {
        let origin = Origin {
            session_id: 1,
            ..Default::default()
        };

        let attr1 = Attribute {
            name: "sendrecv".to_string(),
            value: None,
        };

        let attr2 = Attribute {
            name: "tool".to_string(),
            value: Some("test-tool".to_string()),
        };

        let session = SessionDescriptionBuilder::new(SdpType::Offer)
            .origin(origin)
            .session_name("Test")
            .add_attribute(attr1)
            .add_attribute(attr2)
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build()
            .unwrap();

        assert_eq!(session.attributes.len(), 2);
        assert_eq!(session.attributes[0].name, "sendrecv");
        assert_eq!(session.attributes[1].name, "tool");
    }

    #[test]
    fn test_builder_complete_session() {
        let session = SessionDescriptionBuilder::new(SdpType::Answer)
            .origin(Origin {
                username: "user".to_string(),
                session_id: 999,
                session_version: 1,
                network_type: "IN".to_string(),
                address_type: "IP4".to_string(),
                unicast_address: "10.0.0.1".to_string(),
            })
            .session_name("Complete Session")
            .timing(Timing {
                start_time: 0,
                stop_time: 0,
            })
            .connection(Connection {
                network_type: "IN".to_string(),
                address_type: "IP4".to_string(),
                address: "224.2.1.1".parse::<IpAddr>().unwrap(),
                ttl: Some(127),
                num_addresses: None,
            })
            .add_attribute(Attribute {
                name: "recvonly".to_string(),
                value: None,
            })
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build()
            .unwrap();

        assert_eq!(session.sdp_type, SdpType::Answer);
        assert_eq!(session.origin.session_id, 999);
        assert_eq!(session.session_name, "Complete Session");
        assert!(session.connection.is_some());
        assert_eq!(session.attributes.len(), 1);
        assert_eq!(session.media.len(), 1);
    }

    #[test]
    fn test_builder_validation_fails_no_media() {
        let result = SessionDescriptionBuilder::new(SdpType::Offer)
            .session_name("Test")
            .build();

        assert!(result.is_err());
    }

    #[test]
    fn test_builder_validation_fails_empty_session_name() {
        let result = SessionDescriptionBuilder::new(SdpType::Offer)
            .session_name("")
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build();

        assert!(result.is_err());
    }
}
