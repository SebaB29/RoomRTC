//! Infrastructure Layer
//!
//! This module contains infrastructure components like network clients,
//! external service integrations, and communication protocols.
//!
//! # Components
//!
//! - `tcp_client`: TCP client for signaling server communication
//! - `tls_client`: TLS connection utilities

pub mod tcp_client;
pub mod tls_client;

pub use tcp_client::TcpClient;
