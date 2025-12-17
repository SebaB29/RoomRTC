//! Complete SDP session description.
//!
//! This module provides the main `SessionDescription` type that represents
//! a complete SDP message according to RFC 4566.

use crate::{
    attribute::Attribute, connection::Connection, errors::SdpError,
    media_description::MediaDescription, origin::Origin, sdp_type::SdpType,
    session_description_builder::SessionDescriptionBuilder, timing::Timing,
};

/// Represents a complete Session Description according to RFC 4566.
#[derive(Debug, Clone)]
pub struct SessionDescription {
    pub sdp_type: SdpType,
    pub version: i32,
    pub origin: Origin,
    pub session_name: String,
    pub timing: Timing,
    pub media: Vec<MediaDescription>,
    pub attributes: Vec<Attribute>,
    pub connection: Option<Connection>,
}

/// Represents the different types of SDP lines for pattern matching.
enum SdpLineType {
    Version,
    Origin,
    SessionName,
    Timing,
    Connection,
    Media,
    Attribute,
    Unknown,
}

impl From<char> for SdpLineType {
    fn from(c: char) -> Self {
        match c {
            'v' => Self::Version,
            'o' => Self::Origin,
            's' => Self::SessionName,
            't' => Self::Timing,
            'c' => Self::Connection,
            'm' => Self::Media,
            'a' => Self::Attribute,
            _ => Self::Unknown,
        }
    }
}

impl SessionDescription {
    /// Creates a new empty `SessionDescription` with default values.
    ///
    /// # Arguments
    /// * `sdp_type` - The type of SDP message (Offer or Answer)
    ///
    /// # Returns
    /// A new `SessionDescription` instance with default values for all fields
    pub fn new(sdp_type: SdpType) -> Self {
        Self {
            sdp_type,
            ..Default::default()
        }
    }

    /// Creates a builder for constructing `SessionDescription` instances.
    ///
    /// # Arguments
    /// * `sdp_type` - The type of SDP message (Offer or Answer)
    ///
    /// # Returns
    /// A new `SessionDescriptionBuilder` instance
    pub fn builder(sdp_type: SdpType) -> SessionDescriptionBuilder {
        SessionDescriptionBuilder::new(sdp_type)
    }

    /// Parses an SDP string into a `SessionDescription`.
    ///
    /// # Arguments
    /// * `sdp_type` - The type of SDP message (Offer or Answer)
    /// * `sdp_str` - The SDP message string to parse
    ///
    /// # Returns
    /// * `Ok(SessionDescription)` - A fully parsed session description
    /// * `Err(SdpError)` - If there's any error in the SDP format or content
    pub fn parse(sdp_type: SdpType, sdp_str: &str) -> Result<Self, SdpError> {
        let mut session = SessionDescription::new(sdp_type);
        let mut current_media: Option<MediaDescription> = None;

        for line in sdp_str.lines() {
            if line.is_empty() {
                continue;
            }

            let (type_char, value) = Self::split_line(line)?;
            Self::process_line(&mut session, &mut current_media, type_char, value)?;
        }

        if let Some(media) = current_media {
            session.media.push(media);
        }

        session.validate()?;
        Ok(session)
    }

    /// Splits an SDP line into its type character and value components.
    ///
    /// Each SDP line must be in the format `<type>=<value>` where `type` is
    /// a single character followed by '=' and `value` is the rest of the line.
    ///
    /// # Arguments
    /// * `line` - The SDP line to split
    ///
    /// # Returns
    /// * `Ok((char, &str))` - The type character and value
    /// * `Err(SdpError)` - If the line format is invalid
    fn split_line(line: &str) -> Result<(char, &str), SdpError> {
        let mut parts = line.splitn(2, '=');

        let type_str = parts.next().ok_or(SdpError::InvalidLineFormat)?;
        let value = parts.next().ok_or(SdpError::InvalidLineFormat)?;

        let type_char = type_str.chars().next().ok_or(SdpError::EmptyTypeChar)?;

        if type_str.len() != 1 {
            return Err(SdpError::InvalidLineFormat);
        }

        Ok((type_char, value))
    }

    /// Processes a single line of SDP and updates the session accordingly.
    ///
    /// # Arguments
    /// * `session` - The session description to update
    /// * `current_media` - The current media section being parsed, if any
    /// * `type_char` - The type character from the SDP line
    /// * `value` - The value part of the SDP line
    ///
    /// # Returns
    /// * `Ok(())` - If the line was processed successfully
    /// * `Err(SdpError)` - If there was an error processing the line
    fn process_line(
        session: &mut Self,
        current_media: &mut Option<MediaDescription>,
        type_char: char,
        value: &str,
    ) -> Result<(), SdpError> {
        match SdpLineType::from(type_char) {
            SdpLineType::Version => session.set_version(value),
            SdpLineType::Origin => session.set_origin(value),
            SdpLineType::SessionName => session.set_session_name(value),
            SdpLineType::Timing => session.set_timing(value),
            SdpLineType::Connection => session.set_connection(current_media, value),
            SdpLineType::Media => session.set_media(current_media, value),
            SdpLineType::Attribute => session.set_attribute(current_media, value),
            SdpLineType::Unknown => Ok(()),
        }
    }

    /// Sets the SDP version number.
    ///
    /// According to RFC 4566, the version must be zero (0).
    ///
    /// # Arguments
    /// * `value` - The version string to parse
    fn set_version(&mut self, value: &str) -> Result<(), SdpError> {
        self.version = value.parse().map_err(|_| SdpError::InvalidVersion)?;
        Ok(())
    }

    /// Sets the origin field.
    ///
    /// The origin field identifies the session creator and a session identifier.
    ///
    /// # Arguments
    /// * `value` - The origin string to parse
    fn set_origin(&mut self, value: &str) -> Result<(), SdpError> {
        self.origin = Origin::parse(value)?;
        Ok(())
    }

    /// Sets the session name.
    ///
    /// The session name provides a human-readable identifier for the session.
    ///
    /// # Arguments
    /// * `value` - The session name
    fn set_session_name(&mut self, value: &str) -> Result<(), SdpError> {
        self.session_name = value.to_string();
        Ok(())
    }

    /// Sets the timing information.
    ///
    /// The timing field specifies when the session starts and stops.
    ///
    /// # Arguments
    /// * `value` - The timing string to parse
    fn set_timing(&mut self, value: &str) -> Result<(), SdpError> {
        self.timing = Timing::parse(value)?;
        Ok(())
    }

    /// Sets the connection data.
    ///
    /// Connection data can appear at the session level and/or in media descriptions.
    ///
    /// # Arguments
    /// * `current_media` - The current media section being parsed, if any
    /// * `value` - The connection data string to parse
    fn set_connection(
        &mut self,
        current_media: &mut Option<MediaDescription>,
        value: &str,
    ) -> Result<(), SdpError> {
        let conn = Connection::parse(value)?;
        match current_media.as_mut() {
            Some(media) => media.connection = Some(conn),
            None => self.connection = Some(conn),
        }
        Ok(())
    }

    /// Sets a media description.
    ///
    /// # Arguments
    /// * `current_media` - The current media section being parsed
    /// * `value` - The media description string to parse
    fn set_media(
        &mut self,
        current_media: &mut Option<MediaDescription>,
        value: &str,
    ) -> Result<(), SdpError> {
        if let Some(media) = current_media.take() {
            self.media.push(media);
        }
        *current_media = Some(MediaDescription::parse(value)?);
        Ok(())
    }

    /// Sets an attribute.
    ///
    /// # Arguments
    /// * `current_media` - The current media section being parsed, if any
    /// * `value` - The attribute string to parse
    fn set_attribute(
        &mut self,
        current_media: &mut Option<MediaDescription>,
        value: &str,
    ) -> Result<(), SdpError> {
        let attr = Attribute::parse(value)?;
        match current_media.as_mut() {
            Some(media) => media.attributes.push(attr),
            None => self.attributes.push(attr),
        }
        Ok(())
    }

    /// Validates the SDP session description according to RFC 4566.
    ///
    /// This method checks:
    /// - Version number is 0
    /// - Origin field is valid
    /// - Session name is not empty
    /// - Timing information is valid
    /// - At least one media section is present
    /// - All media sections are valid
    ///
    /// # Returns
    /// * `Ok(())` - If the session description is valid
    /// * `Err(SdpError)` - If any validation check fails
    pub fn validate(&self) -> Result<(), SdpError> {
        // Validate version
        if self.version != 0 {
            return Err(SdpError::InvalidVersionNumber);
        }

        // Validate origin
        self.origin.validate()?;

        // Session name must not be empty
        if self.session_name.is_empty() {
            return Err(SdpError::EmptySessionName);
        }

        // Validate timing
        self.timing.validate()?;

        // Validate media sections
        if self.media.is_empty() {
            return Err(SdpError::NoMediaSections);
        }

        for media in &self.media {
            media.validate()?;
        }

        Ok(())
    }
}

/// Provides a default implementation for [`SessionDescription`].
impl Default for SessionDescription {
    fn default() -> Self {
        Self {
            sdp_type: SdpType::Offer,
            version: 0,
            origin: Origin::default(),
            session_name: String::from("-"),
            timing: Timing::default(),
            media: Vec::new(),
            attributes: Vec::new(),
            connection: None,
        }
    }
}

/// Implements the Display trait to format a session description according to RFC 4566.
///
/// The session description is formatted in the standard SDP format with the following order:
/// 1. Protocol version ('v=')
/// 2. Origin ('o=')
/// 3. Session name ('s=')
/// 4. Connection data ('c=') if present
/// 5. Timing ('t=')
/// 6. Session-level attributes ('a=')
/// 7. Media descriptions ('m=') with their attributes
///
/// Each line is terminated with a CRLF as per RFC 4566.
impl std::fmt::Display for SessionDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "v={}", self.version)?;
        write!(f, "{}", self.origin)?;
        writeln!(f, "s={}", self.session_name)?;
        if let Some(ref conn) = self.connection {
            write!(f, "{}", conn)?;
        }
        write!(f, "{}", self.timing)?;

        for attr in &self.attributes {
            write!(f, "{}", attr)?;
        }

        for media in &self.media {
            write!(f, "{}", media)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_simple_sdp() -> String {
        "v=0\r\n\
         o=- 123456 789012 IN IP4 192.168.1.1\r\n\
         s=Test Session\r\n\
         t=0 0\r\n\
         m=audio 49170 RTP/AVP 0\r\n"
            .to_string()
    }

    fn create_complete_sdp() -> String {
        "v=0\r\n\
         o=jdoe 2890844526 2890842807 IN IP4 10.47.16.5\r\n\
         s=SDP Seminar\r\n\
         c=IN IP4 224.2.17.12/127\r\n\
         t=2873397496 2873404696\r\n\
         a=recvonly\r\n\
         m=audio 49170 RTP/AVP 0\r\n\
         a=rtpmap:0 PCMU/8000\r\n\
         m=video 51372 RTP/AVP 99\r\n\
         a=rtpmap:99 h263-1998/90000\r\n"
            .to_string()
    }

    #[test]
    fn test_session_description_new() {
        let session = SessionDescription::new(SdpType::Offer);
        assert_eq!(session.sdp_type, SdpType::Offer);
        assert_eq!(session.version, 0);
        assert_eq!(session.session_name, "-");
    }

    #[test]
    fn test_session_description_parse_simple() {
        let sdp = create_simple_sdp();
        let session = SessionDescription::parse(SdpType::Offer, &sdp).unwrap();

        assert_eq!(session.version, 0);
        assert_eq!(session.origin.session_id, 123456);
        assert_eq!(session.session_name, "Test Session");
        assert_eq!(session.media.len(), 1);
        assert_eq!(session.media[0].media_type, "audio");
    }

    #[test]
    fn test_session_description_parse_complete() {
        let sdp = create_complete_sdp();
        let session = SessionDescription::parse(SdpType::Offer, &sdp).unwrap();

        assert_eq!(session.version, 0);
        assert_eq!(session.origin.username, "jdoe");
        assert_eq!(session.session_name, "SDP Seminar");
        assert!(session.connection.is_some());
        assert_eq!(session.attributes.len(), 1);
        assert_eq!(session.media.len(), 2);
    }

    #[test]
    fn test_session_description_parse_with_media_attributes() {
        let sdp = create_complete_sdp();
        let session = SessionDescription::parse(SdpType::Offer, &sdp).unwrap();

        assert_eq!(session.media[0].attributes.len(), 1);
        assert_eq!(session.media[0].attributes[0].name, "rtpmap");
        assert_eq!(session.media[1].attributes.len(), 1);
    }

    #[test]
    fn test_session_description_validate_invalid_version() {
        let mut session = SessionDescription::new(SdpType::Offer);
        session.version = 1;
        session.session_name = "Test".to_string();
        session.origin.session_id = 123;
        session.media.push(MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: Vec::new(),
        });

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_description_validate_empty_session_name() {
        let mut session = SessionDescription::new(SdpType::Offer);
        session.session_name = "".to_string();
        session.origin.session_id = 123;
        session.media.push(MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: Vec::new(),
        });

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_description_validate_no_media() {
        let mut session = SessionDescription::new(SdpType::Offer);
        session.session_name = "Test".to_string();
        session.origin.session_id = 123;

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_description_validate_invalid_origin() {
        let mut session = SessionDescription::new(SdpType::Offer);
        session.session_name = "Test".to_string();
        session.origin.session_id = 0; // Invalid
        session.media.push(MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: Vec::new(),
        });

        assert!(session.validate().is_err());
    }

    #[test]
    fn test_session_description_parse_invalid_line_format() {
        let sdp = "v=0\r\ninvalid line\r\ns=Test\r\n";
        assert!(SessionDescription::parse(SdpType::Offer, sdp).is_err());
    }

    #[test]
    fn test_session_description_split_line_valid() {
        let (type_char, value) = SessionDescription::split_line("v=0").unwrap();
        assert_eq!(type_char, 'v');
        assert_eq!(value, "0");
    }

    #[test]
    fn test_session_description_split_line_invalid() {
        assert!(SessionDescription::split_line("invalid").is_err());
        assert!(SessionDescription::split_line("").is_err());
        assert!(SessionDescription::split_line("=value").is_err());
    }

    #[test]
    fn test_session_description_builder() {
        let origin = Origin {
            session_id: 1,
            ..Default::default()
        };

        let session = SessionDescription::builder(SdpType::Offer)
            .origin(origin)
            .session_name("Test Session")
            .add_media(MediaDescription {
                media_type: "audio".to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            })
            .build();

        assert!(session.is_ok());
        let session = session.unwrap();
        assert_eq!(session.session_name, "Test Session");
        assert_eq!(session.media.len(), 1);
    }

    #[test]
    fn test_session_description_display() {
        let mut session = SessionDescription::new(SdpType::Offer);
        session.session_name = "Test".to_string();
        session.origin.session_id = 123456;
        session.origin.session_version = 1;
        session.media.push(MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: Vec::new(),
        });

        let display = format!("{}", session);
        assert!(display.contains("v=0"));
        assert!(display.contains("o=- 123456 1"));
        assert!(display.contains("s=Test"));
        assert!(display.contains("t=0 0"));
        assert!(display.contains("m=audio 49170 RTP/AVP 0"));
    }

    #[test]
    fn test_session_description_default() {
        let session = SessionDescription::default();
        assert_eq!(session.sdp_type, SdpType::Offer);
        assert_eq!(session.version, 0);
        assert_eq!(session.session_name, "-");
        assert!(session.media.is_empty());
    }
}
