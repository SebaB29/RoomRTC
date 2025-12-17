//! TLS error types and implementations.

use native_tls::HandshakeError;
use std::io;
use std::net::TcpStream;

/// TLS configuration error
#[derive(Debug)]
pub enum TlsError {
    /// IO error reading certificate file
    Io(io::Error),
    /// Invalid or corrupted certificate
    InvalidCertificate(String),
    /// Native TLS library error
    NativeTls(native_tls::Error),
    /// TLS handshake failed
    HandshakeError(HandshakeError<TcpStream>),
}

impl From<io::Error> for TlsError {
    fn from(err: io::Error) -> Self {
        TlsError::Io(err)
    }
}

impl From<native_tls::Error> for TlsError {
    fn from(err: native_tls::Error) -> Self {
        TlsError::NativeTls(err)
    }
}

impl From<HandshakeError<TcpStream>> for TlsError {
    fn from(err: HandshakeError<TcpStream>) -> Self {
        TlsError::HandshakeError(err)
    }
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::Io(e) => write!(f, "IO error: {}", e),
            TlsError::InvalidCertificate(msg) => write!(f, "Invalid certificate: {}", msg),
            TlsError::NativeTls(e) => write!(f, "TLS error: {}", e),
            TlsError::HandshakeError(e) => write!(f, "TLS handshake failed: {}", e),
        }
    }
}

impl std::error::Error for TlsError {}
