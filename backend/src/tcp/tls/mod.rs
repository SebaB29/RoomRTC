//! TLS support for TCP connections.

mod acceptor;
mod error;
mod stream;

pub use acceptor::load_tls_acceptor;
pub use stream::TlsStream;
