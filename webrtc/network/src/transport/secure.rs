use crate::codec::rtcp::{ByePacket, ReceiverReport, RtcpPacketType, RtcpStats, SenderReport};
use crate::codec::rtp::RtpPacket;
use crate::error::MediaError;
use crate::security::dtls::SrtpKeys;
use crate::security::srtp::SrtpContext;
use std::net::{SocketAddr, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

/// Packet type classification for demultiplexing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketType {
    /// DTLS content (includes SCTP over DTLS)
    Dtls,
    /// STUN binding request/response
    Stun,
    /// RTP media packet
    Rtp,
    /// RTCP control packet
    Rtcp,
    /// SCTP packet (raw, not wrapped in DTLS)
    Sctp,
    Unknown,
}

/// Classify packet type by first byte (RFC 7983 + SCTP)
///
/// - DTLS: 20-63 (content type)
/// - STUN: 0-3 (first two bits 00)
/// - RTP: 128-191 (version 2, first bit of padding)
/// - RTCP: RTP with payload type 200-204
/// - SCTP: Check for SCTP common header pattern
pub fn classify_packet(data: &[u8]) -> PacketType {
    if data.is_empty() {
        return PacketType::Unknown;
    }

    // Check for SCTP first (12-byte header minimum)
    if data.len() >= 12 {
        // SCTP packets often use port 5000 (0x1388) for WebRTC DataChannels
        // Check if it looks like an SCTP packet structure
        let first_byte = data[0];
        if first_byte < 20 && first_byte != 0 && first_byte != 1 {
            // Likely SCTP - not DTLS (20-63), not STUN (0-1), not RTP (128+)
            return PacketType::Sctp;
        }
    }

    match data[0] {
        // DTLS records start with content type 20-63
        20..=63 => PacketType::Dtls,
        // STUN messages start with 0x00 or 0x01
        0 | 1 => PacketType::Stun,
        // RTP/RTCP have version 2 (bits 10xxxxxx = 128-191)
        128..=191 => {
            // Distinguish RTP from RTCP by payload type
            if data.len() > 1 && data[1] >= 200 && data[1] <= 204 {
                PacketType::Rtcp
            } else {
                PacketType::Rtp
            }
        }
        _ => PacketType::Unknown,
    }
}

/// Basic UDP transport (for comparison)
pub struct UdpTransport {
    socket: UdpSocket,
    remote_addr: Option<SocketAddr>,
}

impl UdpTransport {
    pub fn new(local_addr: &str) -> Result<Self, MediaError> {
        let socket = UdpSocket::bind(local_addr)
            .map_err(|e| MediaError::Network(format!("Failed to bind UDP: {}", e)))?;

        Ok(UdpTransport {
            socket,
            remote_addr: None,
        })
    }

    pub fn set_remote(&mut self, addr: SocketAddr) {
        self.remote_addr = Some(addr);
    }

    pub fn socket(&self) -> &UdpSocket {
        &self.socket
    }

    pub fn send(&self, data: &[u8]) -> Result<(), MediaError> {
        if let Some(addr) = self.remote_addr {
            self.socket
                .send_to(data, addr)
                .map_err(|e| MediaError::Network(format!("Failed to send: {}", e)))?;
            Ok(())
        } else {
            Err(MediaError::Network("No remote address set".to_string()))
        }
    }

    pub fn receive(&self) -> Result<Option<(Vec<u8>, SocketAddr)>, MediaError> {
        let mut buf = vec![0u8; 2048];

        // Set non-blocking to avoid hanging
        self.socket
            .set_nonblocking(true)
            .map_err(|e| MediaError::Network(format!("Failed to set non-blocking: {}", e)))?;

        match self.socket.recv_from(&mut buf) {
            Ok((size, addr)) => {
                buf.truncate(size);
                Ok(Some((buf, addr)))
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(MediaError::Network(format!("Failed to receive: {}", e))),
        }
    }
}

/// Secure UDP transport with SRTP encryption and RTCP statistics
pub struct SecureUdpTransport {
    udp_transport: UdpTransport, // Media socket for RTP/RTCP/DTLS (Sans-IO dimpl)
    srtp_tx: SrtpContext,        // For encrypting outgoing packets
    srtp_rx: SrtpContext,        // For decrypting incoming packets
    rtcp_stats: RtcpStats,       // RTCP statistics tracker
    last_sr_sent: Option<Instant>, // Last Sender Report time
    sr_interval: Duration,       // Sender Report interval (default: 5 seconds)
    rtp_buffer: Arc<Mutex<std::collections::VecDeque<RtpPacket>>>, // Buffer for RTP packets from unified receive
}

impl SecureUdpTransport {
    /// Create a new secure transport from DTLS-derived keys
    pub fn new_from_dtls(udp_transport: UdpTransport, srtp_keys: SrtpKeys) -> Self {
        let srtp_tx = SrtpContext::new(srtp_keys.local_master_key, srtp_keys.local_master_salt);

        let srtp_rx = SrtpContext::new(srtp_keys.remote_master_key, srtp_keys.remote_master_salt);

        // Use a random SSRC for RTCP stats
        let ssrc = rand::random();

        SecureUdpTransport {
            udp_transport,
            srtp_tx,
            srtp_rx,
            rtcp_stats: RtcpStats::new(ssrc),
            last_sr_sent: None,
            sr_interval: Duration::from_secs(5),
            rtp_buffer: Arc::new(Mutex::new(std::collections::VecDeque::new())),
        }
    }

    /// Get reference to media socket for DTLS handshake (Sans-IO dimpl)
    pub fn socket(&self) -> &UdpSocket {
        self.udp_transport.socket()
    }

    /// Update SRTP keys after DTLS handshake completes
    pub fn update_srtp_keys(&mut self, srtp_keys: SrtpKeys) {
        self.srtp_tx = SrtpContext::new(srtp_keys.local_master_key, srtp_keys.local_master_salt);
        self.srtp_rx = SrtpContext::new(srtp_keys.remote_master_key, srtp_keys.remote_master_salt);
    }

    /// Get remote address from UDP transport
    pub fn remote_addr(&self) -> SocketAddr {
        self.udp_transport
            .remote_addr
            .expect("Remote address not set")
    }

    /// Unified receive with proper multiplexing
    ///
    /// Peek, classify, then consume RTP/RTCP packets
    /// DTLS packets are handled separately by dimpl during handshake
    pub fn unified_receive(&mut self) -> Result<bool, MediaError> {
        // Peek at packet to classify without consuming
        let mut peek_buf = [0u8; 2048]; // Large enough for any packet
        let (peek_size, source_addr) = match self.udp_transport.socket().peek_from(&mut peek_buf) {
            Ok(res) => res,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(false); // No packet available
            }
            Err(e) => {
                return Err(MediaError::Network(format!("Peek failed: {}", e)));
            }
        };

        // Filter loopback
        if let Ok(local_addr) = self.udp_transport.socket().local_addr()
            && source_addr == local_addr {
                // Consume and discard loopback
                let _ = self.udp_transport.receive();
                return Ok(false);
            }

        // Classify based on peeked data
        let packet_type = classify_packet(&peek_buf[..peek_size]);

        match packet_type {
            PacketType::Dtls => {
                // DTLS packet after handshake - likely SCTP over DTLS
                // Don't discard - will be handled by session layer
                Ok(false)
            }
            PacketType::Rtp => {
                // RTP packet - now consume from socket
                let Some((encrypted, _)) = self.udp_transport.receive()? else {
                    return Ok(false);
                };

                // Decrypt with SRTP
                match self.srtp_rx.unprotect(&encrypted) {
                    Ok(packet) => {
                        let packet_size = packet.payload.len() + 12;
                        let arrival_time = SystemTime::now();
                        self.rtcp_stats.update_receiver(
                            packet_size,
                            packet.header.sequence_number,
                            packet.header.timestamp,
                            arrival_time,
                        );

                        if let Ok(mut buffer) = self.rtp_buffer.lock() {
                            buffer.push_back(packet);
                        }
                        Ok(true)// RTP packet processed
                    }
                    Err(_) => {
                        // SRTP decryption error
                        Ok(false)
                    }
                }
            }
            PacketType::Rtcp => {
                // RTCP - consume and handle
                let Some((encrypted, _)) = self.udp_transport.receive()? else {
                    return Ok(false);
                };
                let _ = self.handle_rtcp_packet(&encrypted);
                Ok(false)
            }
            PacketType::Stun | PacketType::Sctp | PacketType::Unknown => {
                // Consume and ignore
                let _ = self.udp_transport.receive();
                Ok(false)
            }
        }
    }

    /// Send an RTP packet (encrypted with SRTP)
    pub fn send_rtp(&mut self, packet: &RtpPacket) -> Result<(), MediaError> {
        // Update sender statistics
        let packet_size = packet.payload.len() + 12; // RTP header + payload
        self.rtcp_stats
            .update_sender(packet_size, packet.header.timestamp);

        // Encrypt with SRTP (works directly with codec::rtp::RtpPacket)
        let encrypted = self
            .srtp_tx
            .protect(packet)
            .map_err(|e| MediaError::Network(format!("SRTP encryption failed: {:?}", e)))?;

        // Send over UDP
        self.udp_transport.send(&encrypted)?;

        // Check if we should send a Sender Report
        self.check_and_send_sr()?;

        Ok(())
    }

    /// Receive an RTP packet (from buffer populated by unified_receive)
    ///
    /// NOTE: This no longer reads from UDP directly. Call unified_receive() first!
    pub fn receive_rtp(&mut self) -> Result<Option<RtpPacket>, MediaError> {
        // Get packet from buffer (populated by unified_receive)
        if let Ok(mut buffer) = self.rtp_buffer.lock() {
            Ok(buffer.pop_front())
        } else {
            Ok(None)
        }
    }

    /// Set remote address for sending
    pub fn set_remote(&mut self, addr: SocketAddr) {
        self.udp_transport.set_remote(addr);
    }

    /// Get current RTCP statistics
    pub fn get_stats(&self) -> &RtcpStats {
        &self.rtcp_stats
    }

    /// Reset SRTP replay protection (call when remote stream restarts)
    /// This is needed when the peer turns their camera off and back on,
    /// causing sequence numbers to restart from 0
    pub fn reset_srtp_receiver(&mut self) {
        self.srtp_rx.reset_replay_protection();
    }

    /// Receive DTLS packet for SCTP over DTLS (peek + consume)
    pub fn receive_dtls_packet(&mut self) -> Result<Option<Vec<u8>>, MediaError> {
        let mut peek_buf = [0u8; 2048];
        let (peek_size, _) = match self.udp_transport.socket().peek_from(&mut peek_buf) {
            Ok(res) => res,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                return Ok(None);
            }
            Err(e) => {
                return Err(MediaError::Network(format!("Peek failed: {}", e)));
            }
        };

        // Check if it's a DTLS packet
        let packet_type = classify_packet(&peek_buf[..peek_size]);
        if matches!(packet_type, PacketType::Dtls) {
            // Consume the DTLS packet
            let Some((dtls_packet, _)) = self.udp_transport.receive()? else {
                return Ok(None);
            };
            Ok(Some(dtls_packet))
        } else {
            Ok(None)
        }
    }

    /// Send SCTP packet - will be handled by DtlsEngine in session layer
    pub fn send_sctp(&self, _data: &[u8]) -> Result<(), MediaError> {
        // SCTP over DTLS is now handled by DtlsEngine in the session layer
        // This method is kept for API compatibility but does nothing
        Ok(())
    }

    /// Receive SCTP packet - will be handled by DtlsEngine in session layer
    pub fn receive_sctp(&mut self) -> Result<Option<Vec<u8>>, MediaError> {
        // SCTP over DTLS is now handled by DtlsEngine in the session layer
        // This method is kept for API compatibility but returns None
        Ok(None)
    }

    /// Poll for buffered SCTP packets (legacy)
    pub fn poll_sctp_buffer(&self) -> Result<Option<Vec<u8>>, MediaError> {
        Ok(None)
    }

    /// Send a BYE packet (on call end)
    pub fn send_bye(&mut self, reason: Option<String>) -> Result<(), MediaError> {
        let bye_packet = ByePacket {
            ssrcs: vec![self.rtcp_stats.ssrc],
            reason,
        };

        let bytes = bye_packet.to_bytes();
        self.udp_transport.send(&bytes)?;

        Ok(())
    }

    /// Check if it's time to send a Sender Report
    fn check_and_send_sr(&mut self) -> Result<(), MediaError> {
        let now = Instant::now();

        // Send SR every sr_interval seconds
        if let Some(last_sr) = self.last_sr_sent
            && now.duration_since(last_sr) < self.sr_interval {
                return Ok(());
            }

        // Generate and send Sender Report
        let sr = SenderReport::new(&self.rtcp_stats);
        let bytes = sr.to_bytes();
        self.udp_transport.send(&bytes)?;

        // Track when SR was sent for RTT calculation
        self.rtcp_stats.last_sr_sent_at = Some(SystemTime::now());
        self.last_sr_sent = Some(now);

        Ok(())
    }

    /// Handle received RTCP packet
    fn handle_rtcp_packet(&mut self, bytes: &[u8]) -> Result<(), MediaError> {
        if bytes.len() < 8 {
            return Ok(()); // Too short to be valid RTCP
        }

        let packet_type = bytes[1];

        match RtcpPacketType::from_u8(packet_type) {
            Some(RtcpPacketType::SR) => {
                // Sender Report received - could send RR in response
                if let Ok(sr) = SenderReport::from_bytes(bytes) {
                    // Update last SR timestamp for RTT calculation
                    self.rtcp_stats.last_sr_timestamp =
                        ((sr.ntp_timestamp_msw as u64) << 32 | sr.ntp_timestamp_lsw as u64) as u32;
                    self.rtcp_stats.last_sr_received_at = Some(std::time::SystemTime::now());
                }
            }
            Some(RtcpPacketType::RR) => {
                // Receiver Report received - could use for RTT calculation
                if let Ok(_rr) = ReceiverReport::from_bytes(bytes) {
                    // Process receiver feedback
                    // This could be used to adjust sending rate
                }
            }
            Some(RtcpPacketType::BYE) => {
                // Peer is ending the session
                if let Ok(bye) = ByePacket::from_bytes(bytes)
                    && let Some(reason) = bye.reason {
                        println!("Peer sent BYE: {}", reason);
                    }
            }
            _ => {
                // Unknown or unsupported RTCP packet type
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::rtp::RtpHeader;

    #[test]
    fn test_rtp_packet_serialization() {
        let header = RtpHeader {
            version: 2,
            padding: false,
            extension: false,
            csrc_count: 0,
            marker: false,
            payload_type: 96,
            sequence_number: 100,
            timestamp: 1000,
            ssrc: 12345,
        };

        let packet = RtpPacket::new(header, vec![1, 2, 3, 4, 5]);

        let bytes = packet.to_bytes();
        assert!(bytes.len() >= 12);

        let parsed = RtpPacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.header.sequence_number, 100);
        assert_eq!(parsed.header.timestamp, 1000);
        assert_eq!(parsed.header.ssrc, 12345);
        assert_eq!(parsed.header.payload_type, 96);
        assert_eq!(parsed.payload, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_udp_transport_creation() {
        let transport = UdpTransport::new("127.0.0.1:0");
        assert!(transport.is_ok());
    }

    #[test]
    fn test_packet_classification() {
        assert_eq!(classify_packet(&[22, 3, 1]), PacketType::Dtls);

        assert_eq!(classify_packet(&[0, 1, 0, 0]), PacketType::Stun);

        assert_eq!(classify_packet(&[0x80, 96, 0, 0]), PacketType::Rtp);

        assert_eq!(classify_packet(&[0x80, 200, 0, 0]), PacketType::Rtcp);

        assert_eq!(classify_packet(&[]), PacketType::Unknown);
    }
}
