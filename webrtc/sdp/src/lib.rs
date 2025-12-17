//! SDP Module - Session Description Protocol
//!
//! Implementation of Session Description Protocol according to RFC 4566

pub mod attribute;
pub mod connection;
pub mod errors;
pub mod media_description;
pub mod origin;
pub mod sdp_type;
pub mod session_description;
pub mod session_description_builder;
pub mod timing;

pub use attribute::Attribute;
pub use connection::Connection;
pub use errors::SdpError;
pub use media_description::MediaDescription;
pub use origin::Origin;
pub use sdp_type::SdpType;
pub use session_description::SessionDescription;
pub use session_description_builder::SessionDescriptionBuilder;
pub use timing::Timing;
