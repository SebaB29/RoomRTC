//! STUN client implementation
//!
//! This module provides a STUN client for discovering reflexive (public) addresses.
//! The client sends Binding Requests to a STUN server and receives the reflexive
//! address in the response.

use std::io;
use std::net::{SocketAddr, UdpSocket};
use std::time::Duration;

use crate::attribute_type::AttributeType;
use crate::message::Message;
use crate::message_builder::MessageBuilder;
use crate::message_type::MessageType;
use crate::xor_mapped_address;

/// Maximum STUN message size (typical)
const MAX_STUN_MESSAGE_SIZE: usize = 548;

/// STUN client for discovering reflexive addresses.
///
/// The client sends Binding Requests to a STUN server and receives
/// the reflexive (public) address in the response.
pub struct StunClient {
    socket: UdpSocket,
    server_addr: SocketAddr,
}

impl StunClient {
    /// Creates a new STUN client.
    ///
    /// # Arguments
    /// * `bind_addr` - Local address to bind the socket to
    /// * `server_addr` - Address of the STUN server
    ///
    /// # Returns
    /// * `Ok(StunClient)` - If the client was created successfully
    /// * `Err(io::Error)` - If binding fails
    pub fn new(bind_addr: SocketAddr, server_addr: SocketAddr) -> io::Result<Self> {
        let socket = UdpSocket::bind(bind_addr)?;
        socket.set_read_timeout(Some(Duration::from_secs(3)))?;
        socket.set_write_timeout(Some(Duration::from_secs(3)))?;

        Ok(Self {
            socket,
            server_addr,
        })
    }

    /// Performs a STUN Binding Request to discover the reflexive address.
    ///
    /// # Returns
    /// * `Ok(SocketAddr)` - The reflexive address returned by the server
    /// * `Err(io::Error)` - If the request fails
    pub fn get_reflexive_address(&self) -> io::Result<SocketAddr> {
        // Create Binding Request using MessageBuilder
        let request = MessageBuilder::new(MessageType::Request)
            .random_transaction_id()
            .build()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e.to_string()))?;

        // Send request
        self.socket.send_to(&request.encode(), self.server_addr)?;

        // Receive response (use smaller buffer for efficiency)
        let mut buf = [0u8; MAX_STUN_MESSAGE_SIZE];
        let (size, _) = self.socket.recv_from(&mut buf)?;

        // Parse response
        let response = Message::decode(&buf[..size]).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidData, "Failed to decode STUN response")
        })?;

        // Verify it's a Binding Response
        if response.message_type() != MessageType::Response {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Received non-Binding Response",
            ));
        }

        // Verify transaction ID matches
        if response.transaction_id() != request.transaction_id() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Transaction ID mismatch",
            ));
        }

        // Extract XOR-MAPPED-ADDRESS (preferred) or MAPPED-ADDRESS
        self.extract_reflexive_address(&response)
    }

    /// Extracts the reflexive address from a Binding Response.
    ///
    /// # Arguments
    /// * `response` - The STUN Binding Response message
    ///
    /// # Returns
    /// * `Ok(SocketAddr)` - The extracted reflexive address
    /// * `Err(io::Error)` - If extraction fails
    fn extract_reflexive_address(&self, response: &Message) -> io::Result<SocketAddr> {
        let attrs = response.attributes_bytes();
        let mut offset = 0;
        let attrs_len = attrs.len();

        while offset + 4 <= attrs_len {
            let attr_type = u16::from_be_bytes([attrs[offset], attrs[offset + 1]]);
            let attr_length = u16::from_be_bytes([attrs[offset + 2], attrs[offset + 3]]) as usize;

            offset += 4;

            if offset + attr_length > attrs_len {
                break;
            }

            // Try XOR-MAPPED-ADDRESS first (preferred)
            if attr_type == AttributeType::XorMappedAddress.to_u16() {
                let attr_value = &attrs[offset..offset + attr_length];
                if let Some(addr) =
                    xor_mapped_address::decode(attr_value, &response.transaction_id())
                {
                    return Ok(addr);
                }
            }

            // Move to next attribute (with padding)
            let padding = (4 - (attr_length % 4)) % 4;
            offset += attr_length + padding;
        }

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "No reflexive address found in response",
        ))
    }

    /// Convenience method to discover reflexive address from multiple STUN server names.
    ///
    /// This helper resolves each server name (DNS lookup), tries each resolved address,
    /// and returns the reflexive address from the first successful server.
    ///
    /// # Arguments
    /// * `bind_addr` - Local address to bind the socket to
    /// * `servers` - Array of server names like "stun.l.google.com:19302"
    ///
    /// # Returns
    /// * `Ok(SocketAddr)` - The reflexive address from the first successful server
    /// * `Err(io::Error)` - If all servers/addresses fail
    /// ```
    pub fn discover_reflexive_from_servers(
        bind_addr: SocketAddr,
        servers: &[String],
    ) -> io::Result<SocketAddr> {
        use std::net::ToSocketAddrs;

        let mut last_error = None;

        for (i, server_str) in servers.iter().enumerate() {
            println!(
                "[STUN] Attempting server {}/{}: {}",
                i + 1,
                servers.len(),
                server_str
            );

            let addrs: Vec<SocketAddr> = match server_str.to_socket_addrs() {
                Ok(iter) => {
                    let addrs: Vec<_> = iter.collect();
                    println!(
                        "[STUN] Resolved {} to {} address(es)",
                        server_str,
                        addrs.len()
                    );
                    addrs
                }
                Err(e) => {
                    println!("[STUN] DNS resolution failed for {}: {}", server_str, e);
                    last_error = Some(e);
                    continue;
                }
            };

            if addrs.is_empty() {
                println!("[STUN] No addresses resolved for {}", server_str);
                continue;
            }

            for (j, server_addr) in addrs.iter().enumerate() {
                println!(
                    "[STUN] Trying address {}/{}: {}",
                    j + 1,
                    addrs.len(),
                    server_addr
                );

                match StunClient::new(bind_addr, *server_addr) {
                    Ok(client) => match client.get_reflexive_address() {
                        Ok(reflexive) => {
                            println!("[STUN] SUCCESS! Got reflexive address: {}", reflexive);
                            return Ok(reflexive);
                        }
                        Err(e) => {
                            println!("[STUN] Query failed to {}: {}", server_addr, e);
                            last_error = Some(e);
                        }
                    },
                    Err(e) => {
                        println!("[STUN] Failed to create client for {}: {}", server_addr, e);
                        last_error = Some(e);
                    }
                }
            }
        }

        Err(last_error.unwrap_or_else(|| io::Error::other("All STUN servers failed")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_stun_client() {
        let bind_addr = "127.0.0.1:0".parse().unwrap();
        let server_addr = "127.0.0.1:3478".parse().unwrap();
        let client = StunClient::new(bind_addr, server_addr);
        assert!(client.is_ok());
    }

    #[test]
    fn test_generate_transaction_id() {
        use crate::message_builder::MessageBuilder;
        use crate::message_type::MessageType;

        // Test that MessageBuilder generates different transaction IDs
        let msg1 = MessageBuilder::new(MessageType::Request)
            .random_transaction_id()
            .build()
            .unwrap();

        let msg2 = MessageBuilder::new(MessageType::Request)
            .random_transaction_id()
            .build()
            .unwrap();

        // Transaction IDs should be different
        assert_ne!(msg1.transaction_id(), msg2.transaction_id());
    }
}
