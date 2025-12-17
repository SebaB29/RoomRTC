//! TLS stream wrapper with Read/Write trait implementations.

use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;

use super::error::TlsError;

/// TLS-wrapped TCP stream using native-tls
pub struct TlsStream {
    stream: native_tls::TlsStream<TcpStream>,
}

impl TlsStream {
    /// Accept TLS connection from TCP stream
    pub fn accept(
        stream: TcpStream,
        acceptor: Arc<native_tls::TlsAcceptor>,
    ) -> Result<Self, TlsError> {
        let tls_stream = acceptor.accept(stream)?;
        Ok(TlsStream { stream: tls_stream })
    }

    /// Get reference to underlying TCP stream
    pub fn get_ref(&self) -> &TcpStream {
        self.stream.get_ref()
    }

    /// Get mutable reference to underlying TCP stream
    pub fn get_mut(&mut self) -> &mut TcpStream {
        self.stream.get_mut()
    }
}

impl Read for TlsStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }
}

impl Write for TlsStream {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.stream.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stream.flush()
    }
}
