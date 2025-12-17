//! Client connection handler managing authentication and message routing.

use std::io::{self, ErrorKind};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use crate::application::handlers::message_handler::MessageHandler;
use crate::application::usecases::AuthUseCase;
use crate::infrastructure::storage::Storage;
use crate::tcp::messages::{LoginRequest, Message};
use crate::tcp::stream_type::StreamType;
use crate::tcp::tls::TlsStream;

/// Client connection handler managing authentication and message routing
pub struct ClientHandler {
    stream: StreamType,
    auth_usecase: AuthUseCase,
    message_handler: MessageHandler,
    logger: logging::Logger,
    authenticated_user_id: Option<String>,
    msg_receiver: Option<Receiver<Message>>,
}

impl ClientHandler {
    pub fn new(
        stream: TcpStream,
        storage: Storage,
        logger: logging::Logger,
        tls_acceptor: Option<Arc<native_tls::TlsAcceptor>>,
    ) -> io::Result<Self> {
        let peer_addr = stream.peer_addr()?;

        // Perform TLS handshake if enabled
        let mut stream = if let Some(acceptor) = tls_acceptor {
            logger.info(&format!("Performing TLS handshake with {}", peer_addr));
            match TlsStream::accept(stream, acceptor) {
                Ok(tls_stream) => {
                    logger.info(&format!("TLS handshake successful with {}", peer_addr));
                    StreamType::Tls(tls_stream)
                }
                Err(e) => {
                    logger.error(&format!("TLS handshake failed with {}: {}", peer_addr, e));
                    return Err(io::Error::other(format!("TLS handshake failed: {}", e)));
                }
            }
        } else {
            StreamType::Plain(stream)
        };

        // Set read timeout for non-blocking receiver check
        if let Err(e) = stream.set_read_timeout(Duration::from_millis(100)) {
            logger.warn(&format!("Failed to set read timeout: {}", e));
        }

        let auth_logger = logger
            .for_component("Auth Usecase")
            .unwrap_or_else(|_| logger.clone());
        let message_handler = MessageHandler::new(storage.clone(), logger.clone());
        let auth_usecase = AuthUseCase::new(storage, auth_logger);

        Ok(ClientHandler {
            stream,
            auth_usecase,
            message_handler,
            logger,
            authenticated_user_id: None,
            msg_receiver: None,
        })
    }

    pub fn handle(&mut self) -> io::Result<()> {
        let peer_addr = self.stream.peer_addr()?;
        self.logger
            .info(&format!("New connection from {}", peer_addr));

        loop {
            self.send_pending_messages()?;

            let message = match self.read_message_with_timeout() {
                Ok(Some(msg)) => msg,
                Ok(None) => continue, // Timeout, check for pending messages
                Err(e) => {
                    self.logger
                        .error(&format!("Failed to read message from {}: {}", peer_addr, e));
                    self.cleanup_disconnect();
                    return Err(e);
                }
            };

            if let Err(e) = self.handle_and_respond(message, &peer_addr) {
                self.cleanup_disconnect();
                return Err(e);
            }
        }
    }

    /// Send pending broadcast messages to client (non-blocking)
    fn send_pending_messages(&mut self) -> io::Result<()> {
        if let Some(ref rx) = self.msg_receiver {
            match rx.try_recv() {
                Ok(pending_msg) => {
                    self.logger.info("Sending pending message to client");
                    self.stream.write_message(&pending_msg).map_err(|e| {
                        self.logger
                            .error(&format!("Failed to send pending message: {}", e));
                        io::Error::new(ErrorKind::BrokenPipe, e)
                    })?;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.logger.warn("Message channel disconnected");
                }
            }
        }
        Ok(())
    }

    /// Read message with timeout handling
    fn read_message_with_timeout(&mut self) -> io::Result<Option<Message>> {
        match self.stream.read_message() {
            Ok(msg) => Ok(Some(msg)),
            Err(e) => {
                if let super::protocol::ProtocolError::Io(io_err) = &e
                    && (io_err.kind() == io::ErrorKind::WouldBlock
                        || io_err.kind() == io::ErrorKind::TimedOut)
                {
                    return Ok(None); // Timeout, not an error
                }
                Err(io::Error::other(format!("{}", e)))
            }
        }
    }

    /// Process message and send response if needed
    fn handle_and_respond(
        &mut self,
        message: Message,
        peer_addr: &std::net::SocketAddr,
    ) -> io::Result<()> {
        let response = self.process_message(message)?;

        if let Some(msg) = response {
            self.stream.write_message(&msg).map_err(|e| {
                self.logger
                    .error(&format!("Failed to send response to {}: {}", peer_addr, e));
                io::Error::new(ErrorKind::BrokenPipe, e)
            })?;
        }
        Ok(())
    }

    fn process_message(&mut self, message: Message) -> io::Result<Option<Message>> {
        match message {
            // Login handled here (needs TcpStream for writer thread setup)
            Message::LoginRequest(req) => self.handle_login(req),
            // All other messages delegated to application layer
            _ => self
                .message_handler
                .process_message(message, self.authenticated_user_id.as_ref()),
        }
    }

    /// Handle login request
    /// Exceptional case - needs TcpStream for writer thread setup
    fn handle_login(&mut self, req: LoginRequest) -> io::Result<Option<Message>> {
        let result = match &mut self.stream {
            StreamType::Plain(stream) => self.auth_usecase.handle_login(&req, stream),
            StreamType::Tls(stream) => self.auth_usecase.handle_login(&req, stream),
        };

        match result {
            Ok(Some((user_id, receiver))) => {
                self.authenticated_user_id = Some(user_id.clone());
                self.msg_receiver = Some(receiver);
                self.logger
                    .info(&format!("User authenticated: {}", user_id));
                Ok(None)
            }
            Ok(None) => {
                self.logger
                    .warn(&format!("Login failed for username: {}", req.username));
                Ok(None)
            }
            Err(e) => {
                self.logger
                    .error(&format!("Login error for username {}: {}", req.username, e));
                Err(e)
            }
        }
    }

    fn cleanup_disconnect(&self) {
        if let Some(user_id) = &self.authenticated_user_id {
            self.message_handler.cleanup_user_disconnect(user_id);
        }
    }
}
