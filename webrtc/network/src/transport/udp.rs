//! UDP transport for sending and receiving RTP packets

use crate::error::{NetworkError, Result};
use logging::Logger;
use std::net::{SocketAddr, UdpSocket};

/// Transport UDP for RTP
pub struct UdpTransport {
    socket: UdpSocket,
    remote_addr: Option<SocketAddr>,
    logger: Logger,
    bytes_sent: u64,
    bytes_received: u64,
}

impl UdpTransport {
    /// Create new UDP transport
    ///
    /// # Arguments
    /// * `bind_addr` - Local Address for bind
    /// * `logger` - Logger
    pub fn new(bind_addr: SocketAddr, logger: Logger) -> Result<Self> {
        logger.info(&format!("Creating UDP socket at {}", bind_addr));

        let socket = UdpSocket::bind(bind_addr)
            .map_err(|e| NetworkError::Network(format!("Error creating socket: {}", e)))?;

        socket
            .set_nonblocking(true)
            .map_err(|e| NetworkError::Network(format!("Error setting non-blocking: {}", e)))?;

        logger.info("UDP socket correctly configured");

        Ok(UdpTransport {
            socket,
            remote_addr: None,
            logger,
            bytes_sent: 0,
            bytes_received: 0,
        })
    }

    /// Establishes the remote address
    pub fn set_remote(&mut self, addr: SocketAddr) {
        self.logger
            .info(&format!("Establishes remote address: {}", addr));
        self.remote_addr = Some(addr);
    }

    /// Get the configured remote address
    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    /// Send data to remote peer
    ///
    /// # Arguments
    /// * `data` - Data to send
    pub fn send(&mut self, data: &[u8]) -> Result<usize> {
        let remote_addr = self.get_remote_addr()?;
        let sent = self.send_to_socket(data, remote_addr)?;
        self.update_send_stats(sent);
        Ok(sent)
    }

    /// Get remote address or return error if not set
    fn get_remote_addr(&self) -> Result<SocketAddr> {
        self.remote_addr
            .ok_or_else(|| NetworkError::Network("Remote address not set".to_string()))
    }

    /// Send data to socket
    fn send_to_socket(&self, data: &[u8], addr: SocketAddr) -> Result<usize> {
        self.socket
            .send_to(data, addr)
            .map_err(|e| NetworkError::Network(format!("Error sending: {}", e)))
    }

    /// Update send statistics and log if necessary
    fn update_send_stats(&mut self, sent: usize) {
        self.bytes_sent += sent as u64;
        if self.bytes_sent % 100_000 < sent as u64 {
            self.logger
                .debug(&format!("Total bytes sent: {}", self.bytes_sent));
        }
    }

    /// Receive data from socket (non-blocking)
    ///
    /// # Returns
    /// * `Ok(Some((data, addr)))` - Data and sender address received
    /// * `Ok(None)` - No data available
    /// * `Err` - Network error
    pub fn receive(&mut self) -> Result<Option<(Vec<u8>, SocketAddr)>> {
        let mut buf = vec![0u8; 65536];

        match self.socket.recv_from(&mut buf) {
            Ok((size, addr)) => {
                buf.truncate(size);
                self.update_receive_stats(size);
                Ok(Some((buf, addr)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(NetworkError::Network(format!("Error receiving: {}", e))),
        }
    }

    /// Update receive statistics and log if necessary
    fn update_receive_stats(&mut self, size: usize) {
        self.bytes_received += size as u64;
        if self.bytes_received % 100_000 < size as u64 {
            self.logger
                .debug(&format!("Total bytes received: {}", self.bytes_received));
        }
    }

    /// Returns transport statistics (bytes_sent, bytes_received)
    pub fn stats(&self) -> (u64, u64) {
        (self.bytes_sent, self.bytes_received)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;
    use tempfile::tempdir;

    fn create_test_logger() -> Logger {
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        logging::Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    #[test]
    fn test_udp_transport_creation() {
        let logger = create_test_logger();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = UdpTransport::new(addr, logger);
        assert!(transport.is_ok());
    }

    #[test]
    fn test_udp_transport_set_remote() {
        let logger = create_test_logger();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut transport = UdpTransport::new(addr, logger).unwrap();

        let remote: SocketAddr = "127.0.0.1:5000".parse().unwrap();
        transport.set_remote(remote);

        assert!(transport.remote_addr.is_some());
        assert_eq!(transport.remote_addr.unwrap(), remote);
    }

    #[test]
    fn test_udp_transport_send_without_remote() {
        let logger = create_test_logger();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut transport = UdpTransport::new(addr, logger).unwrap();

        let data = vec![1, 2, 3, 4, 5];
        let result = transport.send(&data);

        assert!(result.is_err());
    }

    #[test]
    fn test_udp_transport_stats_initial() {
        let logger = create_test_logger();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let transport = UdpTransport::new(addr, logger).unwrap();

        let (sent, received) = transport.stats();
        assert_eq!(sent, 0);
        assert_eq!(received, 0);
    }

    #[test]
    fn test_udp_transport_receive_nonblocking() {
        let logger = create_test_logger();
        let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let mut transport = UdpTransport::new(addr, logger).unwrap();

        // Should return None immediately (non-blocking)
        let result = transport.receive().unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn test_udp_transport_send_receive() {
        let logger1 = create_test_logger();
        let logger2 = create_test_logger();

        // Create two transports
        let addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let mut transport1 = UdpTransport::new(addr1, logger1).unwrap();
        let mut transport2 = UdpTransport::new(addr2, logger2).unwrap();

        // Get actual bound addresses
        let actual_addr1 = transport1.socket.local_addr().unwrap();
        let actual_addr2 = transport2.socket.local_addr().unwrap();

        // Set remote addresses
        transport1.set_remote(actual_addr2);
        transport2.set_remote(actual_addr1);

        // Send data
        let test_data = vec![10, 20, 30, 40, 50];
        let sent = transport1.send(&test_data).unwrap();
        assert_eq!(sent, test_data.len());

        // Give some time for packet to arrive
        std::thread::sleep(std::time::Duration::from_millis(10));

        // Receive data
        let received = transport2.receive().unwrap();
        assert!(received.is_some());

        let (data, from_addr) = received.unwrap();
        assert_eq!(data, test_data);
        assert_eq!(from_addr, actual_addr1);
    }

    #[test]
    fn test_udp_transport_stats_after_send() {
        let logger1 = create_test_logger();
        let logger2 = create_test_logger();

        let addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let mut transport1 = UdpTransport::new(addr1, logger1).unwrap();
        let transport2 = UdpTransport::new(addr2, logger2).unwrap();

        let actual_addr2 = transport2.socket.local_addr().unwrap();
        transport1.set_remote(actual_addr2);

        let test_data = vec![1, 2, 3, 4, 5];
        transport1.send(&test_data).unwrap();

        let (sent, _) = transport1.stats();
        assert_eq!(sent, test_data.len() as u64);
    }

    #[test]
    fn test_udp_transport_multiple_sends() {
        let logger1 = create_test_logger();
        let logger2 = create_test_logger();

        let addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let mut transport1 = UdpTransport::new(addr1, logger1).unwrap();
        let transport2 = UdpTransport::new(addr2, logger2).unwrap();

        let actual_addr2 = transport2.socket.local_addr().unwrap();
        transport1.set_remote(actual_addr2);

        // Send multiple packets
        for i in 0..5 {
            let data = vec![i; 10];
            transport1.send(&data).unwrap();
        }

        let (sent, _) = transport1.stats();
        assert_eq!(sent, 50); // 5 packets * 10 bytes
    }

    #[test]
    fn test_udp_transport_empty_data() {
        let logger1 = create_test_logger();
        let logger2 = create_test_logger();

        let addr1: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:0".parse().unwrap();

        let mut transport1 = UdpTransport::new(addr1, logger1).unwrap();
        let transport2 = UdpTransport::new(addr2, logger2).unwrap();

        let actual_addr2 = transport2.socket.local_addr().unwrap();
        transport1.set_remote(actual_addr2);

        let empty_data: Vec<u8> = vec![];
        let result = transport1.send(&empty_data);
        assert!(result.is_ok());
    }
}
