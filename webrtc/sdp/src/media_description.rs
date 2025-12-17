//! SDP media description representation.
//!
//! Media descriptions define the properties of individual media streams
//! within an SDP session.

use crate::{attribute::Attribute, connection::Connection};

/// Represents a media description (m=) in an SDP message as defined in RFC 4566.
///
/// A media description starts with an 'm=' line and contains all of the following
/// fields in the specified order:
/// ```text
/// m=<media> <port> <proto> <fmt> ...
/// ```
///
/// # Arguments
/// * `media_type` - Media type ("audio", "video", "text", "application", "message")
/// * `port` - Transport port number for the media stream
/// * `protocol` - Transport protocol (e.g., "RTP/AVP", "udp")
/// * `formats` - Media format descriptions (codec types, payload types)
/// * `connection` - Optional connection information specific to this media
/// * `attributes` - Media-level attributes that apply only to this media stream
#[derive(Debug, Clone)]
pub struct MediaDescription {
    pub media_type: String,
    pub port: u16,
    pub protocol: String,
    pub formats: Vec<String>,
    pub connection: Option<Connection>,
    pub attributes: Vec<Attribute>,
}

/// Valid media types as per RFC 4566.
/// Common types include "audio", "video", "text", "application", and "message".
const VALID_MEDIA_TYPES: &[&str] = &["audio", "video", "text", "application", "message"];

impl MediaDescription {
    /// Parses a media description line according to RFC 4566.
    ///
    /// The media description line must be in the format:
    /// `<media> <port> <proto> <fmt> ...`
    ///
    /// # Arguments
    /// * `value` - The media description string to parse (without the "m=" prefix)
    ///
    /// # Returns
    /// * `Ok(MediaDescription)` - A successfully parsed media description
    /// * `Err(SdpError::InvalidMediaFormat)` - If the format is incorrect
    /// * `Err(SdpError::InvalidPort)` - If the port number is invalid
    pub fn parse(value: &str) -> Result<Self, crate::errors::SdpError> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() < 4 {
            return Err(crate::errors::SdpError::InvalidMediaFormat);
        }

        let port = parts[1]
            .parse()
            .map_err(|_| crate::errors::SdpError::InvalidPort)?;

        Ok(MediaDescription {
            media_type: parts[0].to_string(),
            port,
            protocol: parts[2].to_string(),
            formats: parts[3..].iter().map(|s| s.to_string()).collect(),
            connection: None,
            attributes: Vec::new(),
        })
    }

    /// Validates the media description according to RFC 4566 specifications.
    ///
    /// This method performs the following checks:
    /// - Media type must be one of: "audio", "video", "text", "application", "message"
    /// - At least one media format must be specified
    ///
    /// # Returns
    /// * `Ok(())` - If all validation checks pass
    /// * `Err(SdpError::InvalidMediaType)` - If the media type is not recognized
    /// * `Err(SdpError::NoMediaFormats)` - If no media formats are specified
    pub fn validate(&self) -> Result<(), crate::errors::SdpError> {
        // Media type must be one of: audio, video, text, application, message
        if !VALID_MEDIA_TYPES.contains(&self.media_type.as_str()) {
            return Err(crate::errors::SdpError::InvalidMediaType(
                self.media_type.clone(),
            ));
        }

        // Must have at least one format
        if self.formats.is_empty() {
            return Err(crate::errors::SdpError::NoMediaFormats);
        }

        Ok(())
    }
}

/// Implements the Display trait to format media descriptions according to RFC 4566.
///
/// The media description is formatted in the following order:
/// 1. Media line ('m=') with type, port, protocol, and formats
/// 2. Connection data ('c=') if present
/// 3. Media-level attributes ('a=')
impl std::fmt::Display for MediaDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "m={} {} {} {}\r\n",
            self.media_type,
            self.port,
            self.protocol,
            self.formats.join(" ")
        )?;

        if let Some(ref conn) = self.connection {
            write!(f, "{}", conn)?;
        }

        for attr in &self.attributes {
            write!(f, "{}", attr)?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_media_description_parse_audio() {
        let media = MediaDescription::parse("audio 49170 RTP/AVP 0").unwrap();
        assert_eq!(media.media_type, "audio");
        assert_eq!(media.port, 49170);
        assert_eq!(media.protocol, "RTP/AVP");
        assert_eq!(media.formats, vec!["0"]);
        assert!(media.connection.is_none());
        assert!(media.attributes.is_empty());
    }

    #[test]
    fn test_media_description_parse_video() {
        let media = MediaDescription::parse("video 51372 RTP/AVP 99").unwrap();
        assert_eq!(media.media_type, "video");
        assert_eq!(media.port, 51372);
        assert_eq!(media.protocol, "RTP/AVP");
        assert_eq!(media.formats, vec!["99"]);
    }

    #[test]
    fn test_media_description_parse_multiple_formats() {
        let media = MediaDescription::parse("audio 49170 RTP/AVP 0 8 97").unwrap();
        assert_eq!(media.media_type, "audio");
        assert_eq!(media.port, 49170);
        assert_eq!(media.protocol, "RTP/AVP");
        assert_eq!(media.formats, vec!["0", "8", "97"]);
    }

    #[test]
    fn test_media_description_parse_invalid_format() {
        assert!(MediaDescription::parse("audio 49170 RTP/AVP").is_err());
        assert!(MediaDescription::parse("audio 49170").is_err());
        assert!(MediaDescription::parse("audio").is_err());
        assert!(MediaDescription::parse("").is_err());
    }

    #[test]
    fn test_media_description_parse_invalid_port() {
        assert!(MediaDescription::parse("audio abc RTP/AVP 0").is_err());
        assert!(MediaDescription::parse("audio 99999 RTP/AVP 0").is_err());
    }

    #[test]
    fn test_media_description_validate_valid_audio() {
        let media = MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: Vec::new(),
        };
        assert!(media.validate().is_ok());
    }

    #[test]
    fn test_media_description_validate_valid_video() {
        let media = MediaDescription {
            media_type: "video".to_string(),
            port: 51372,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["99".to_string()],
            connection: None,
            attributes: Vec::new(),
        };
        assert!(media.validate().is_ok());
    }

    #[test]
    fn test_media_description_validate_all_valid_types() {
        for media_type in &["audio", "video", "text", "application", "message"] {
            let media = MediaDescription {
                media_type: media_type.to_string(),
                port: 49170,
                protocol: "RTP/AVP".to_string(),
                formats: vec!["0".to_string()],
                connection: None,
                attributes: Vec::new(),
            };
            assert!(media.validate().is_ok());
        }
    }

    #[test]
    fn test_media_description_validate_invalid_media_type() {
        let media = MediaDescription {
            media_type: "invalid".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: Vec::new(),
        };
        assert!(media.validate().is_err());
    }

    #[test]
    fn test_media_description_validate_no_formats() {
        let media = MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: Vec::new(),
            connection: None,
            attributes: Vec::new(),
        };
        assert!(media.validate().is_err());
    }

    #[test]
    fn test_media_description_display() {
        let media = MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string(), "8".to_string()],
            connection: None,
            attributes: Vec::new(),
        };
        let display = format!("{}", media);
        assert_eq!(display, "m=audio 49170 RTP/AVP 0 8\r\n");
    }

    #[test]
    fn test_media_description_display_with_connection() {
        let media = MediaDescription {
            media_type: "video".to_string(),
            port: 51372,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["99".to_string()],
            connection: Some(Connection {
                network_type: "IN".to_string(),
                address_type: "IP4".to_string(),
                address: "192.168.1.1".parse().unwrap(),
                ttl: None,
                num_addresses: None,
            }),
            attributes: Vec::new(),
        };
        let display = format!("{}", media);
        assert!(display.contains("m=video 51372 RTP/AVP 99"));
        assert!(display.contains("c=IN IP4 192.168.1.1"));
    }

    #[test]
    fn test_media_description_display_with_attributes() {
        let media = MediaDescription {
            media_type: "audio".to_string(),
            port: 49170,
            protocol: "RTP/AVP".to_string(),
            formats: vec!["0".to_string()],
            connection: None,
            attributes: vec![
                Attribute {
                    name: "rtpmap".to_string(),
                    value: Some("0 PCMU/8000".to_string()),
                },
                Attribute {
                    name: "sendrecv".to_string(),
                    value: None,
                },
            ],
        };
        let display = format!("{}", media);
        assert!(display.contains("m=audio 49170 RTP/AVP 0"));
        assert!(display.contains("a=rtpmap:0 PCMU/8000"));
        assert!(display.contains("a=sendrecv"));
    }
}
