//! Error types for SDP operations.
//!
//! This module defines all possible errors that can occur during
//! Session Description Protocol (SDP) parsing and validation.

/// Error type for SDP operations
#[derive(Debug)]
pub enum SdpError {
    /// Error when a line doesn't follow the format type=value
    InvalidLineFormat,
    /// Error when a type character is empty
    EmptyTypeChar,
    /// Error when version is invalid
    InvalidVersion,
    /// Error when version is not 0
    InvalidVersionNumber,
    /// Error when session name is empty
    EmptySessionName,
    /// Error when no media sections are present
    NoMediaSections,
    /// Error when the media type is invalid
    InvalidMediaType(String),
    /// Error when the media description has no formats
    NoMediaFormats,
    /// Error when network type is not IN
    InvalidNetworkType,
    /// Error when address type is not IP4 or IP6
    InvalidAddressType,
    /// Error when session ID is 0
    InvalidSessionId,
    /// Error when stop time is less than start time
    InvalidTiming,
    /// Error when parsing IP address fails
    InvalidIpAddress,
    /// Error when parsing TTL fails
    InvalidTtl,
    /// Error when parsing number of addresses fails
    InvalidAddressCount,
    /// Error when parsing origin format
    InvalidOriginFormat,
    /// Error when parsing timing format
    InvalidTimingFormat,
    /// Error when parsing media description format
    InvalidMediaFormat,
    /// Error when parsing port number
    InvalidPort,
    /// Error when parsing attribute format
    InvalidAttributeFormat,
}

impl std::fmt::Display for SdpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use SdpError::*;
        let msg = match self {
            InvalidLineFormat => "Line must be in format 'type=value'",
            EmptyTypeChar => "Type character cannot be empty",
            InvalidVersion => "Failed to parse version",
            InvalidVersionNumber => "SDP version must be 0",
            EmptySessionName => "Session name cannot be empty",
            NoMediaSections => "SDP must contain at least one media description",
            InvalidMediaType(t) => return write!(f, "Invalid media type: {}", t),
            NoMediaFormats => "Media description must have at least one format",
            InvalidNetworkType => "Network type must be IN",
            InvalidAddressType => "Address type must be IP4 or IP6",
            InvalidSessionId => "Session ID cannot be 0",
            InvalidTiming => "Stop time must be greater than or equal to start time",
            InvalidIpAddress => "Invalid IP address format",
            InvalidTtl => "Invalid TTL value",
            InvalidAddressCount => "Invalid number of addresses",
            InvalidOriginFormat => "Invalid origin format",
            InvalidTimingFormat => "Invalid timing format",
            InvalidMediaFormat => "Invalid media description format",
            InvalidPort => "Invalid port number",
            InvalidAttributeFormat => "Invalid attribute format",
        };
        write!(f, "{}", msg)
    }
}

impl std::error::Error for SdpError {}
