use native_tls::{TlsConnector, TlsStream};
use std::io;
use std::net::TcpStream;

/// Connect to a TLS server
///
/// Note: For development with self-signed certificates, this disables certificate validation.
/// If we wnd up deploying -> change to properly signed certificates and enable validation.
pub fn connect_tls(addr: &str) -> io::Result<TlsStream<TcpStream>> {
    let hostname = addr.split(':').next().unwrap_or(addr);

    let stream = TcpStream::connect(addr)?;

    // Build TLS connector with certificate validation disabled for development
    // Note: here is what we need to change for production with valid certs (no danger)
    let connector = TlsConnector::builder()
        .danger_accept_invalid_certs(true)
        .danger_accept_invalid_hostnames(true)
        .build()
        .map_err(|e| io::Error::other(format!("TLS connector error: {}", e)))?;

    // Perform TLS handshake
    let tls_stream = connector
        .connect(hostname, stream)
        .map_err(|e| io::Error::other(format!("TLS handshake failed: {}", e)))?;

    Ok(tls_stream)
}
