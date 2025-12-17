//! ICE Module - Interactive Connectivity Establishment
//!
//! Implementation of ICE (Interactive Connectivity Establishment) according to RFC 5245.

pub mod candidate;
pub mod candidate_builder;
pub mod candidate_pair;
pub mod candidate_type;
pub mod connection_state;
pub mod connectivity;
pub mod errors;
pub mod ice_agent;
pub mod ip_detection;

pub use candidate::Candidate;
pub use candidate_builder::CandidateBuilder;
pub use candidate_pair::CandidatePair;
pub use candidate_type::CandidateType;
pub use connection_state::ConnectionState;
pub use connectivity::{CandidateSocket, perform_connectivity_check};
pub use errors::IceError;
pub use ice_agent::IceAgent;
pub use ip_detection::detect_local_ip;
