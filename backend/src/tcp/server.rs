//! TCP server for WebRTC signaling over binary protocol.

use std::io;
use std::net::TcpListener;
use std::sync::Arc;
use std::thread;

use crate::infrastructure::storage::Storage;
use crate::tcp::tls::load_tls_acceptor;

use super::client_handler::ClientHandler;

/// TCP Server for persistent connections with TLS support
pub struct TcpServer {
    storage: Storage,
    logger: logging::Logger,
    tls_acceptor: Option<Arc<native_tls::TlsAcceptor>>,
}

impl TcpServer {
    pub fn new(storage: Storage, logger: logging::Logger) -> Self {
        TcpServer {
            storage,
            logger,
            tls_acceptor: None,
        }
    }

    /// Enable TLS with the given PKCS#12 file and password
    pub fn with_tls(mut self, pkcs12_path: &str, password: &str) -> Result<Self, String> {
        match load_tls_acceptor(pkcs12_path, password) {
            Ok(acceptor) => {
                self.logger
                    .info(&format!("TLS enabled with certificate: {}", pkcs12_path));
                self.tls_acceptor = Some(acceptor);
                Ok(self)
            }
            Err(e) => Err(format!("Failed to load TLS certificate: {}", e)),
        }
    }

    pub fn start(&self, bind_addr: &str) -> io::Result<()> {
        let listener = TcpListener::bind(bind_addr)?;

        let protocol = if self.tls_acceptor.is_some() {
            "TLS"
        } else {
            "Plain TCP"
        };
        self.logger.info(&format!(
            "TCP Server listening on {} ({} protocol)",
            bind_addr, protocol
        ));

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let storage = self.storage.clone();
                    let logger = self
                        .logger
                        .for_component("ClientHandler")
                        .unwrap_or_else(|_| self.logger.clone());
                    let tls_acceptor = self.tls_acceptor.clone();

                    thread::spawn(move || {
                        match ClientHandler::new(stream, storage, logger.clone(), tls_acceptor) {
                            Ok(mut handler) => {
                                if let Err(e) = handler.handle() {
                                    logger.error(&format!("Client handler error: {}", e));
                                }
                            }
                            Err(e) => {
                                logger.error(&format!("Failed to create client handler: {}", e));
                            }
                        }
                    });
                }
                Err(e) => {
                    self.logger
                        .error(&format!("Failed to accept connection: {}", e));
                }
            }
        }

        Ok(())
    }
}
