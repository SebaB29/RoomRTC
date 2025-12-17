//! Connectivity checking utilities.
//!
//! Provides socket management and connectivity check functionality
//! for ICE candidate pairs.

use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::{candidate::Candidate, errors::IceError};

/// Represents a UDP socket bound to a local candidate.
#[derive(Debug)]
pub struct CandidateSocket {
    pub candidate: Candidate,
    pub socket: UdpSocket,
}

impl CandidateSocket {
    /// Creates a new socket bound to the candidate's address and port.
    ///
    /// # Arguments
    /// * `candidate` - The candidate to bind the socket to
    ///
    /// # Returns
    /// * `Ok(CandidateSocket)` - Successfully created and bound socket
    /// * `Err(IceError)` - If binding fails
    pub fn new(candidate: Candidate) -> Result<Self, IceError> {
        let addr = SocketAddr::new(candidate.address, candidate.port);
        let socket = UdpSocket::bind(addr).map_err(|e| IceError::SocketBindError(e.to_string()))?;

        // Set non-blocking mode for async operations
        socket
            .set_nonblocking(true)
            .map_err(|e| IceError::SocketError(e.to_string()))?;

        // Set read timeout
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(|e| IceError::SocketError(e.to_string()))?;

        Ok(Self { candidate, socket })
    }

    /// Sends data to a remote address.
    ///
    /// # Arguments
    /// * `data` - The data to send
    /// * `addr` - The destination address
    pub fn send_to(&self, data: &[u8], addr: SocketAddr) -> Result<usize, IceError> {
        self.socket
            .send_to(data, addr)
            .map_err(|e| IceError::SocketError(e.to_string()))
    }

    /// Receives data from the socket.
    ///
    /// # Arguments
    /// * `buf` - Buffer to store received data
    ///
    /// # Returns
    /// * `Ok((usize, SocketAddr))` - Number of bytes received and sender address
    /// * `Err(IceError)` - If receive fails
    pub fn recv_from(&self, buf: &mut [u8]) -> Result<(usize, SocketAddr), IceError> {
        self.socket
            .recv_from(buf)
            .map_err(|e| IceError::SocketError(e.to_string()))
    }
}

/// Performs a basic connectivity check between two candidates.
///
/// This is a simplified version that sends a test message and waits for a response.
/// A full implementation would use STUN Binding requests.
///
/// # Arguments
/// * `local_socket` - The local candidate socket
/// * `remote_candidate` - The remote candidate to check
///
/// # Returns
/// * `Ok(true)` - If connectivity check succeeds
/// * `Ok(false)` - If connectivity check fails
pub fn perform_connectivity_check(
    local_socket: &CandidateSocket,
    remote_candidate: &Candidate,
) -> Result<bool, IceError> {
    let remote_addr = SocketAddr::new(remote_candidate.address, remote_candidate.port);

    // Send a simple test message (in real WebRTC this would be a STUN Binding Request)
    let test_message = b"ICE_CHECK";
    local_socket.send_to(test_message, remote_addr)?;

    // Try to receive a response (with timeout)
    let mut buf = [0u8; 1024];
    match local_socket.recv_from(&mut buf) {
        Ok((size, addr)) if addr == remote_addr && size > 0 => Ok(true),
        Ok(_) => Ok(false),
        Err(IceError::SocketError(ref e))
            if e.contains("would block") || e.contains("timed out") =>
        {
            Ok(false)
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::candidate_type::CandidateType;
    use std::net::{IpAddr, Ipv4Addr};
    use std::thread;
    use std::time::Duration;

    // Helper para crear un candidato de prueba
    fn create_test_candidate(port: u16) -> Candidate {
        Candidate {
            foundation: "test".to_string(),
            component_id: 1,
            transport: "UDP".to_string(),
            priority: 2130706431,
            address: IpAddr::V4(Ipv4Addr::LOCALHOST),
            port,
            candidate_type: CandidateType::Host,
            related_address: None,
            related_port: None,
        }
    }

    #[test]
    fn test_new_creates_socket_successfully() {
        let candidate = create_test_candidate(0); // Puerto 0 = asignación automática
        let result = CandidateSocket::new(candidate);

        assert!(result.is_ok());
    }

    #[test]
    fn test_new_binds_to_localhost() {
        let candidate = create_test_candidate(0);
        let socket_wrapper = CandidateSocket::new(candidate).unwrap();

        let local_addr = socket_wrapper.socket.local_addr().unwrap();
        assert_eq!(local_addr.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    }

    #[test]
    fn test_new_with_invalid_address_fails() {
        let mut candidate = create_test_candidate(8080);
        candidate.address = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1)); // IP no local

        let result = CandidateSocket::new(candidate);
        assert!(result.is_err());
    }

    #[test]
    fn test_send_to_succeeds() {
        let sender = CandidateSocket::new(create_test_candidate(0)).unwrap();
        let receiver = CandidateSocket::new(create_test_candidate(0)).unwrap();

        let receiver_addr = receiver.socket.local_addr().unwrap();
        let data = b"Hello";

        let result = sender.send_to(data, receiver_addr);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), data.len());
    }

    #[test]
    fn test_recv_from_receives_data() {
        let sender = CandidateSocket::new(create_test_candidate(0)).unwrap();
        let receiver = CandidateSocket::new(create_test_candidate(0)).unwrap();

        let receiver_addr = receiver.socket.local_addr().unwrap();
        let test_data = b"Test message";

        sender.send_to(test_data, receiver_addr).unwrap();
        thread::sleep(Duration::from_millis(50));

        let mut buf = [0u8; 1024];
        let result = receiver.recv_from(&mut buf);

        assert!(result.is_ok());
        let (size, _addr) = result.unwrap();
        assert_eq!(size, test_data.len());
        assert_eq!(&buf[..size], test_data);
    }

    #[test]
    fn test_socket_preserves_candidate_info() {
        let original_candidate = create_test_candidate(0);
        let foundation = original_candidate.foundation.clone();

        let socket_wrapper = CandidateSocket::new(original_candidate).unwrap();

        assert_eq!(socket_wrapper.candidate.foundation, foundation);
        assert_eq!(socket_wrapper.candidate.component_id, 1);
    }

    #[test]
    fn test_multiple_sockets_can_coexist() {
        let socket1 = CandidateSocket::new(create_test_candidate(0));
        let socket2 = CandidateSocket::new(create_test_candidate(0));

        assert!(socket1.is_ok());
        assert!(socket2.is_ok());
    }
}
