//! TURN message types.
//!
//! TURN extends STUN with additional message types for relay functionality.

/// TURN message types according to RFC 5766.
///
/// TURN uses STUN's message format but adds new message types
/// for allocation, permission, and channel management.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnMessageType {
    // Allocate (0x0003)
    AllocateRequest,
    AllocateResponse,
    AllocateError,

    // Refresh (0x0004)
    RefreshRequest,
    RefreshResponse,
    RefreshError,

    // CreatePermission (0x0008)
    CreatePermissionRequest,
    CreatePermissionResponse,
    CreatePermissionError,

    // ChannelBind (0x0009)
    ChannelBindRequest,
    ChannelBindResponse,
    ChannelBindError,

    // Send/Data Indications
    SendIndication,
    DataIndication,
}

impl TurnMessageType {
    /// Converts TURN message type to its 16-bit numeric value.
    ///
    /// # Returns
    /// The message type value used in STUN/TURN message encoding
    pub fn to_u16(self) -> u16 {
        match self {
            // Allocate: 0x0003
            TurnMessageType::AllocateRequest => 0x0003,
            TurnMessageType::AllocateResponse => 0x0103,
            TurnMessageType::AllocateError => 0x0113,

            // Refresh: 0x0004
            TurnMessageType::RefreshRequest => 0x0004,
            TurnMessageType::RefreshResponse => 0x0104,
            TurnMessageType::RefreshError => 0x0114,

            // CreatePermission: 0x0008
            TurnMessageType::CreatePermissionRequest => 0x0008,
            TurnMessageType::CreatePermissionResponse => 0x0108,
            TurnMessageType::CreatePermissionError => 0x0118,

            // ChannelBind: 0x0009
            TurnMessageType::ChannelBindRequest => 0x0009,
            TurnMessageType::ChannelBindResponse => 0x0109,
            TurnMessageType::ChannelBindError => 0x0119,

            // Indications
            TurnMessageType::SendIndication => 0x0016,
            TurnMessageType::DataIndication => 0x0017,
        }
    }

    /// Parses a TURN message type from its numeric value.
    ///
    /// # Arguments
    /// * `value` - The 16-bit message type value from a TURN message
    ///
    /// # Returns
    /// * `Some(TurnMessageType)` - If the value represents a valid TURN message type
    /// * `None` - If the value is not recognized as a TURN message type
    pub fn from_u16(value: u16) -> Option<Self> {
        match value {
            0x0003 => Some(TurnMessageType::AllocateRequest),
            0x0103 => Some(TurnMessageType::AllocateResponse),
            0x0113 => Some(TurnMessageType::AllocateError),

            0x0004 => Some(TurnMessageType::RefreshRequest),
            0x0104 => Some(TurnMessageType::RefreshResponse),
            0x0114 => Some(TurnMessageType::RefreshError),

            0x0008 => Some(TurnMessageType::CreatePermissionRequest),
            0x0108 => Some(TurnMessageType::CreatePermissionResponse),
            0x0118 => Some(TurnMessageType::CreatePermissionError),

            0x0009 => Some(TurnMessageType::ChannelBindRequest),
            0x0109 => Some(TurnMessageType::ChannelBindResponse),
            0x0119 => Some(TurnMessageType::ChannelBindError),

            0x0016 => Some(TurnMessageType::SendIndication),
            0x0017 => Some(TurnMessageType::DataIndication),

            _ => None,
        }
    }

    /// Returns the string representation of the message type.
    pub fn as_str(&self) -> &'static str {
        match self {
            TurnMessageType::AllocateRequest => "Allocate Request",
            TurnMessageType::AllocateResponse => "Allocate Success Response",
            TurnMessageType::AllocateError => "Allocate Error Response",
            TurnMessageType::RefreshRequest => "Refresh Request",
            TurnMessageType::RefreshResponse => "Refresh Success Response",
            TurnMessageType::RefreshError => "Refresh Error Response",
            TurnMessageType::CreatePermissionRequest => "CreatePermission Request",
            TurnMessageType::CreatePermissionResponse => "CreatePermission Success Response",
            TurnMessageType::CreatePermissionError => "CreatePermission Error Response",
            TurnMessageType::ChannelBindRequest => "ChannelBind Request",
            TurnMessageType::ChannelBindResponse => "ChannelBind Success Response",
            TurnMessageType::ChannelBindError => "ChannelBind Error Response",
            TurnMessageType::SendIndication => "Send Indication",
            TurnMessageType::DataIndication => "Data Indication",
        }
    }

    /// Checks if this is a request message type.
    pub fn is_request(&self) -> bool {
        matches!(
            self,
            TurnMessageType::AllocateRequest
                | TurnMessageType::RefreshRequest
                | TurnMessageType::CreatePermissionRequest
                | TurnMessageType::ChannelBindRequest
        )
    }

    /// Checks if this is a success response message type.
    pub fn is_success_response(&self) -> bool {
        matches!(
            self,
            TurnMessageType::AllocateResponse
                | TurnMessageType::RefreshResponse
                | TurnMessageType::CreatePermissionResponse
                | TurnMessageType::ChannelBindResponse
        )
    }

    /// Checks if this is an error response message type.
    pub fn is_error_response(&self) -> bool {
        matches!(
            self,
            TurnMessageType::AllocateError
                | TurnMessageType::RefreshError
                | TurnMessageType::CreatePermissionError
                | TurnMessageType::ChannelBindError
        )
    }

    /// Checks if this is an indication message type.
    pub fn is_indication(&self) -> bool {
        matches!(
            self,
            TurnMessageType::SendIndication | TurnMessageType::DataIndication
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_turn_message_type_conversion() {
        assert_eq!(TurnMessageType::AllocateRequest.to_u16(), 0x0003);
        assert_eq!(TurnMessageType::AllocateResponse.to_u16(), 0x0103);
        assert_eq!(TurnMessageType::AllocateError.to_u16(), 0x0113);
        assert_eq!(TurnMessageType::RefreshRequest.to_u16(), 0x0004);
        assert_eq!(TurnMessageType::SendIndication.to_u16(), 0x0016);
        assert_eq!(TurnMessageType::DataIndication.to_u16(), 0x0017);
    }

    #[test]
    fn test_turn_message_type_parsing() {
        assert_eq!(
            TurnMessageType::from_u16(0x0003),
            Some(TurnMessageType::AllocateRequest)
        );
        assert_eq!(
            TurnMessageType::from_u16(0x0104),
            Some(TurnMessageType::RefreshResponse)
        );
        assert_eq!(
            TurnMessageType::from_u16(0x0118),
            Some(TurnMessageType::CreatePermissionError)
        );
        assert_eq!(TurnMessageType::from_u16(0xFFFF), None);
    }

    #[test]
    fn test_message_type_checks() {
        assert!(TurnMessageType::AllocateRequest.is_request());
        assert!(!TurnMessageType::AllocateResponse.is_request());

        assert!(TurnMessageType::AllocateResponse.is_success_response());
        assert!(!TurnMessageType::AllocateRequest.is_success_response());

        assert!(TurnMessageType::AllocateError.is_error_response());
        assert!(!TurnMessageType::AllocateRequest.is_error_response());

        assert!(TurnMessageType::SendIndication.is_indication());
        assert!(TurnMessageType::DataIndication.is_indication());
        assert!(!TurnMessageType::AllocateRequest.is_indication());
    }

    #[test]
    fn test_as_str() {
        assert_eq!(
            TurnMessageType::AllocateRequest.as_str(),
            "Allocate Request"
        );
        assert_eq!(
            TurnMessageType::AllocateResponse.as_str(),
            "Allocate Success Response"
        );
        assert_eq!(TurnMessageType::SendIndication.as_str(), "Send Indication");
    }
}
