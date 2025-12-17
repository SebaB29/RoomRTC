//! TURN client implementation.
//!
//! Provides a TURN client for allocating relay addresses and managing permissions.

use crate::errors::{Result, TurnError};
use crate::message::{
    add_turn_attribute, build_turn_message, generate_transaction_id, parse_turn_message_type,
};
use crate::turn_attribute_type::{TransportProtocol, TurnAttributeType};
use crate::turn_message_type::TurnMessageType;
use std::net::{SocketAddr, UdpSocket};
use std::time::{Duration, Instant};

use logging::Logger;

/// Default allocation lifetime in seconds (10 minutes).
const DEFAULT_LIFETIME: u32 = 600;
/// STUN magic cookie used for XOR operations (RFC 5389)
const MAGIC_COOKIE: u32 = 0x2112A442;
/// Upper 16 bits of magic cookie for XOR port calculation
const MAGIC_COOKIE_HIGH: u16 = 0x2112;
/// USERNAME attribute type (STUN/TURN)
const ATTR_USERNAME: u16 = 0x0006;
/// Maximum UDP packet size
const MAX_UDP_PACKET_SIZE: usize = 1500;

/// TURN client for allocating relay addresses and managing permissions.
///
/// The client supports Allocate, Refresh, CreatePermission, ChannelBind,
/// and Send operations.
pub struct TurnClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
    username: String,
    relay_addr: Option<SocketAddr>,
    lifetime: u32,
    last_refresh: Option<Instant>,
    logger: Option<Logger>,
}

impl TurnClient {
    /// Creates a new TURN client.
    ///
    /// # Arguments
    /// * `server_addr` - TURN server address
    /// * `username` - Authentication username
    /// * `password` - Authentication password
    ///
    /// # Returns
    /// * `Ok(TurnClient)` - Successfully created client
    /// * `Err(TurnError)` - Failed to create socket
    pub fn new(server_addr: SocketAddr, username: String) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(TurnError::Io)?;

        socket
            .set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(TurnError::Io)?;

        Ok(TurnClient {
            socket,
            server_addr,
            username,
            relay_addr: None,
            lifetime: DEFAULT_LIFETIME,
            last_refresh: None,
            logger: None,
        })
    }

    /// Attaches a logger to the TURN client.
    pub fn with_logger(mut self, logger: Logger) -> Self {
        self.logger = Some(logger);
        self
    }

    /// Allocates a relay address on the TURN server.
    ///
    /// Sends an Allocate request with REQUESTED-TRANSPORT and LIFETIME attributes.
    ///
    /// # Returns
    /// * `Ok(SocketAddr)` - The allocated relay address
    /// * `Err(TurnError)` - Allocation failed
    pub fn allocate(&mut self) -> Result<SocketAddr> {
        self.log_info("Sending TURN Allocate request");

        let msg = self.build_allocate_request();
        let response = self.send_and_receive(&msg)?;

        self.verify_success_response(
            &response,
            TurnMessageType::AllocateError,
            "TURN Allocate failed",
        )?;

        let relay_addr = self.extract_xor_relayed_address(&response)?;
        self.relay_addr = Some(relay_addr);
        self.last_refresh = Some(Instant::now());

        self.log_info(&format!("Allocated relay address: {}", relay_addr));
        Ok(relay_addr)
    }

    /// Refreshes the allocation lifetime.
    ///
    /// # Arguments
    /// * `lifetime` - New lifetime in seconds (0 to deallocate)
    ///
    /// # Returns
    /// * `Ok(())` - Refresh successful
    /// * `Err(TurnError)` - Refresh failed
    pub fn refresh(&mut self, lifetime: u32) -> Result<()> {
        self.ensure_allocation()?;
        self.log_info(&format!(
            "Refreshing TURN allocation (lifetime: {}s)",
            lifetime
        ));

        let msg = self.build_refresh_request(lifetime);
        let response = self.send_and_receive(&msg)?;

        self.verify_success_response(
            &response,
            TurnMessageType::RefreshError,
            "TURN Refresh failed",
        )?;

        self.lifetime = lifetime;
        self.last_refresh = Some(Instant::now());
        self.log_info("TURN allocation refreshed");
        Ok(())
    }

    /// Creates a permission for a peer address.
    ///
    /// This must be called before sending data to a peer through the relay.
    ///
    /// # Arguments
    /// * `peer_addr` - The peer's address
    ///
    /// # Returns
    /// * `Ok(())` - Permission created
    /// * `Err(TurnError)` - Failed to create permission
    pub fn create_permission(&mut self, peer_addr: SocketAddr) -> Result<()> {
        self.ensure_allocation()?;
        self.log_info(&format!("Creating TURN permission for {}", peer_addr));

        let msg = self.build_create_permission_request(peer_addr);
        let response = self.send_and_receive(&msg)?;

        self.verify_success_response(
            &response,
            TurnMessageType::CreatePermissionError,
            "TURN CreatePermission failed",
        )?;

        self.log_info("TURN permission created");
        Ok(())
    }

    /// Binds a channel to a peer address.
    ///
    /// Channel binding provides a more efficient data transfer mechanism.
    ///
    /// # Arguments
    /// * `peer_addr` - The peer's address
    /// * `channel` - Channel number (0x4000-0x7FFF)
    ///
    /// # Returns
    /// * `Ok(())` - Channel bound successfully
    /// * `Err(TurnError)` - Failed to bind channel
    pub fn channel_bind(&mut self, peer_addr: SocketAddr, channel: u16) -> Result<()> {
        self.ensure_allocation()?;
        self.validate_channel_number(channel)?;
        self.log_info(&format!(
            "Binding TURN channel {} to {}",
            channel, peer_addr
        ));

        let msg = self.build_channel_bind_request(peer_addr, channel);
        let response = self.send_and_receive(&msg)?;

        self.verify_success_response(
            &response,
            TurnMessageType::ChannelBindError,
            "TURN ChannelBind failed",
        )?;

        self.log_info("TURN channel bound");
        Ok(())
    }

    /// Sends data to a peer through the relay.
    ///
    /// A permission must be created for the peer before sending data.
    ///
    /// # Arguments
    /// * `data` - Data to send
    /// * `peer_addr` - Destination peer address
    ///
    /// # Returns
    /// * `Ok(())` - Data sent successfully
    /// * `Err(TurnError)` - Failed to send data
    pub fn send(&self, data: &[u8], peer_addr: SocketAddr) -> Result<()> {
        self.ensure_allocation()?;

        let msg = self.build_send_indication(data, peer_addr);
        self.socket
            .send_to(&msg, self.server_addr)
            .map_err(TurnError::Io)?;

        self.log_info(&format!(
            "Sent {} bytes to {} via relay",
            data.len(),
            peer_addr
        ));
        Ok(())
    }

    /// Returns the allocated relay address, if any.
    pub fn relay_address(&self) -> Option<SocketAddr> {
        self.relay_addr
    }

    /// Checks if the allocation needs refresh (within 60 seconds of expiry).
    pub fn needs_refresh(&self) -> bool {
        if let Some(last_refresh) = self.last_refresh {
            let elapsed = last_refresh.elapsed().as_secs();
            elapsed >= (self.lifetime as u64).saturating_sub(60)
        } else {
            false
        }
    }

    // ========== Request Builders ==========

    /// Builds a generic TURN request message with username attribute
    fn build_request(&self, msg_type: TurnMessageType) -> Vec<u8> {
        let transaction_id = generate_transaction_id();
        let mut msg = build_turn_message(msg_type, transaction_id);
        self.add_username_attribute(&mut msg);
        msg
    }

    /// Adds a lifetime attribute to a message
    fn add_lifetime_attr(&self, msg: &mut Vec<u8>, lifetime: u32) {
        add_turn_attribute(
            msg,
            TurnAttributeType::Lifetime.to_u16(),
            &lifetime.to_be_bytes(),
        );
    }

    /// Adds a XOR-PEER-ADDRESS attribute to a message
    fn add_xor_peer_addr(&self, msg: &mut Vec<u8>, peer_addr: SocketAddr) {
        let peer_bytes = self.encode_xor_address(peer_addr);
        add_turn_attribute(msg, TurnAttributeType::XorPeerAddress.to_u16(), &peer_bytes);
    }

    /// Builds an Allocate request message
    fn build_allocate_request(&self) -> Vec<u8> {
        let mut msg = self.build_request(TurnMessageType::AllocateRequest);

        // REQUESTED-TRANSPORT (UDP)
        let transport = TransportProtocol::Udp.to_u8();
        add_turn_attribute(
            &mut msg,
            TurnAttributeType::RequestedTransport.to_u16(),
            &[transport, 0, 0, 0],
        );

        self.add_lifetime_attr(&mut msg, self.lifetime);
        msg
    }

    /// Builds a Refresh request message
    fn build_refresh_request(&self, lifetime: u32) -> Vec<u8> {
        let mut msg = self.build_request(TurnMessageType::RefreshRequest);
        self.add_lifetime_attr(&mut msg, lifetime);
        msg
    }

    /// Builds a CreatePermission request message
    fn build_create_permission_request(&self, peer_addr: SocketAddr) -> Vec<u8> {
        let mut msg = self.build_request(TurnMessageType::CreatePermissionRequest);
        self.add_xor_peer_addr(&mut msg, peer_addr);
        msg
    }

    /// Builds a ChannelBind request message
    fn build_channel_bind_request(&self, peer_addr: SocketAddr, channel: u16) -> Vec<u8> {
        let mut msg = self.build_request(TurnMessageType::ChannelBindRequest);

        // CHANNEL-NUMBER (4 bytes: channel number + 2 bytes reserved)
        let channel_bytes = channel.to_be_bytes();
        add_turn_attribute(
            &mut msg,
            TurnAttributeType::ChannelNumber.to_u16(),
            &[channel_bytes[0], channel_bytes[1], 0, 0],
        );

        self.add_xor_peer_addr(&mut msg, peer_addr);
        msg
    }

    /// Builds a Send indication message
    fn build_send_indication(&self, data: &[u8], peer_addr: SocketAddr) -> Vec<u8> {
        let transaction_id = generate_transaction_id();
        let mut msg = build_turn_message(TurnMessageType::SendIndication, transaction_id);

        self.add_xor_peer_addr(&mut msg, peer_addr);
        add_turn_attribute(&mut msg, TurnAttributeType::Data.to_u16(), data);
        msg
    }

    // ========== Helper Methods ==========

    /// Ensures an allocation exists, returns error if not
    fn ensure_allocation(&self) -> Result<()> {
        if self.relay_addr.is_none() {
            return Err(TurnError::NoAllocation);
        }
        Ok(())
    }

    /// Validates channel number is in valid range
    fn validate_channel_number(&self, channel: u16) -> Result<()> {
        if !(0x4000..=0x7FFF).contains(&channel) {
            return Err(TurnError::ChannelBindFailed(
                "Invalid channel number (must be 0x4000-0x7FFF)".to_string(),
            ));
        }
        Ok(())
    }

    /// Adds USERNAME attribute to message
    fn add_username_attribute(&self, msg: &mut Vec<u8>) {
        add_turn_attribute(msg, ATTR_USERNAME, self.username.as_bytes());
    }

    /// Sends a message and receives the response
    fn send_and_receive(&self, msg: &[u8]) -> Result<Vec<u8>> {
        self.socket
            .send_to(msg, self.server_addr)
            .map_err(TurnError::Io)?;

        let mut buffer = [0u8; MAX_UDP_PACKET_SIZE];
        let (len, _) = self.socket.recv_from(&mut buffer).map_err(TurnError::Io)?;

        Ok(buffer[..len].to_vec())
    }

    /// Verifies the response is a success, not an error
    fn verify_success_response(
        &self,
        response: &[u8],
        error_type: TurnMessageType,
        error_msg: &str,
    ) -> Result<()> {
        let response_type = parse_turn_message_type(response).ok_or(TurnError::InvalidResponse)?;

        if response_type.is_error_response() && response_type == error_type {
            self.log_error(error_msg);

            // Map error type to appropriate TurnError variant
            return Err(match error_type {
                TurnMessageType::AllocateError => {
                    TurnError::AllocationFailed(error_msg.to_string())
                }
                TurnMessageType::RefreshError => TurnError::RefreshFailed(error_msg.to_string()),
                TurnMessageType::CreatePermissionError => {
                    TurnError::PermissionFailed(error_msg.to_string())
                }
                TurnMessageType::ChannelBindError => {
                    TurnError::ChannelBindFailed(error_msg.to_string())
                }
                _ => TurnError::InvalidResponse,
            });
        }

        Ok(())
    }

    /// Extracts XOR-RELAYED-ADDRESS from response
    fn extract_xor_relayed_address(&self, bytes: &[u8]) -> Result<SocketAddr> {
        self.find_attribute(bytes, TurnAttributeType::XorRelayedAddress.to_u16())
            .and_then(|value| self.decode_xor_address(value))
    }

    /// Finds an attribute in a TURN message
    fn find_attribute<'a>(&self, bytes: &'a [u8], attr_type: u16) -> Result<&'a [u8]> {
        const HEADER_SIZE: usize = 20;
        const ATTR_HEADER_SIZE: usize = 4;
        const PADDING_ALIGNMENT: usize = 4;

        if bytes.len() < HEADER_SIZE {
            return Err(TurnError::InvalidResponse);
        }

        let mut offset = HEADER_SIZE;

        while offset + ATTR_HEADER_SIZE <= bytes.len() {
            // Parse attribute header
            let current_type = u16::from_be_bytes([bytes[offset], bytes[offset + 1]]);
            let attr_len = u16::from_be_bytes([bytes[offset + 2], bytes[offset + 3]]) as usize;
            offset += ATTR_HEADER_SIZE;

            // Validate attribute length
            if offset + attr_len > bytes.len() {
                break;
            }

            // Check if this is the attribute we're looking for
            if current_type == attr_type {
                return Ok(&bytes[offset..offset + attr_len]);
            }

            // Calculate padding and move to next attribute
            let padding = (PADDING_ALIGNMENT - (attr_len % PADDING_ALIGNMENT)) % PADDING_ALIGNMENT;
            offset += attr_len + padding;
        }

        Err(TurnError::AttributeError(format!(
            "Attribute 0x{:04X} not found",
            attr_type
        )))
    }

    // ========== XOR Address Encoding/Decoding ==========

    /// Encodes SocketAddr as XOR-ed bytes (RFC 5389)
    fn encode_xor_address(&self, addr: SocketAddr) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);

        // Family (IPv4 = 0x01, IPv6 = 0x02)
        bytes.extend_from_slice(&[0x00, 0x01]);

        // XOR port with upper 16 bits of magic cookie
        let xor_port = addr.port() ^ MAGIC_COOKIE_HIGH;
        bytes.extend_from_slice(&xor_port.to_be_bytes());

        // XOR IP address with magic cookie
        if let SocketAddr::V4(addr_v4) = addr {
            let ip = u32::from(*addr_v4.ip());
            let xor_ip = ip ^ MAGIC_COOKIE;
            bytes.extend_from_slice(&xor_ip.to_be_bytes());
        }

        bytes
    }

    /// Decodes XOR-ed address bytes (RFC 5389)
    fn decode_xor_address(&self, bytes: &[u8]) -> Result<SocketAddr> {
        const MIN_XOR_ADDR_LEN: usize = 8;

        if bytes.len() < MIN_XOR_ADDR_LEN {
            return Err(TurnError::AttributeError(
                "XOR address too short".to_string(),
            ));
        }

        // Decode XOR port
        let xor_port = u16::from_be_bytes([bytes[2], bytes[3]]);
        let port = xor_port ^ MAGIC_COOKIE_HIGH;

        // Decode XOR IP
        let xor_ip = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
        let ip = xor_ip ^ MAGIC_COOKIE;
        let ip_addr = std::net::Ipv4Addr::from(ip);

        Ok(SocketAddr::new(ip_addr.into(), port))
    }

    // ========== Logging ==========

    /// Formats a log message with TURN Client prefix
    fn format_log(&self, message: &str) -> String {
        format!("[TURN Client] {}", message)
    }

    /// Logs an info message
    fn log_info(&self, message: &str) {
        if let Some(ref logger) = self.logger {
            logger.info(&self.format_log(message));
        }
    }

    /// Logs an error message
    fn log_error(&self, message: &str) {
        if let Some(ref logger) = self.logger {
            logger.error(&self.format_log(message));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xor_address_encoding() {
        let client =
            TurnClient::new("127.0.0.1:3478".parse().unwrap(), "user".to_string()).unwrap();

        let addr: SocketAddr = "192.0.2.1:1234".parse().unwrap();
        let encoded = client.encode_xor_address(addr);

        // Should have IPv4 family and XOR-ed values
        assert_eq!(encoded[0], 0x00);
        assert_eq!(encoded[1], 0x01);
        assert!(encoded.len() >= 8);
    }

    #[test]
    fn test_needs_refresh() {
        let mut client =
            TurnClient::new("127.0.0.1:3478".parse().unwrap(), "user".to_string()).unwrap();

        // No allocation yet
        assert!(!client.needs_refresh());

        // Simulate allocation
        client.relay_addr = Some("198.51.100.1:5000".parse().unwrap());
        client.last_refresh = Some(Instant::now());

        // Fresh allocation shouldn't need refresh
        assert!(!client.needs_refresh());
    }
}
