//! Stream type abstraction for Plain TCP and TLS connections.

use std::io;
use std::net::TcpStream;
use std::time::Duration;

use super::protocol::{ProtocolError, read_message, write_message};
use crate::tcp::messages::Message;
use crate::tcp::tls::TlsStream;

/// Stream type wrapper for Plain TCP or TLS connections
pub(crate) enum StreamType {
    Plain(TcpStream),
    Tls(TlsStream),
}

impl StreamType {
    /// Configure read timeout for both stream types
    pub(crate) fn set_read_timeout(&mut self, duration: Duration) -> io::Result<()> {
        match self {
            StreamType::Plain(stream) => stream.set_read_timeout(Some(duration)),
            StreamType::Tls(stream) => stream.get_mut().set_read_timeout(Some(duration)),
        }
    }

    /// Get peer address from stream
    pub(crate) fn peer_addr(&self) -> io::Result<std::net::SocketAddr> {
        match self {
            StreamType::Plain(stream) => stream.peer_addr(),
            StreamType::Tls(stream) => stream.get_ref().peer_addr(),
        }
    }

    /// Read message from stream
    pub(crate) fn read_message(&mut self) -> Result<Message, ProtocolError> {
        match self {
            StreamType::Plain(stream) => read_message(stream),
            StreamType::Tls(stream) => read_message(stream),
        }
    }

    /// Write message to stream
    pub(crate) fn write_message(&mut self, msg: &Message) -> Result<(), ProtocolError> {
        match self {
            StreamType::Plain(stream) => write_message(stream, msg),
            StreamType::Tls(stream) => write_message(stream, msg),
        }
    }
}
