//! SDP message type enumeration.
//!
//! Defines whether an SDP message is an offer or an answer.

/// Represents the type of SDP message
#[derive(Debug, Clone, PartialEq)]
pub enum SdpType {
    /// Represents an SDP offer from the initiator
    Offer,
    /// Represents an SDP answer from the receiver
    Answer,
}
