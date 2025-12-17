//! STUN Module - Session Traversal Utilities for NAT
//!
//! Implementation of STUN (Session Traversal Utilities for NAT) as per RFC 5389.

// Internal modules
mod attribute_type;
mod client;
mod errors;
mod message;
mod message_builder;
mod message_header;
mod message_type;
mod xor_mapped_address;

pub use client::StunClient;
pub use errors::StunError;
