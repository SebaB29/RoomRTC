//! XOR-MAPPED-ADDRESS attribute decoding
//!
//! This module implements the XOR-MAPPED-ADDRESS attribute according to RFC 5389.
//! XOR-MAPPED-ADDRESS is the preferred method for conveying reflexive addresses
//! as it XORs the address with the magic cookie and transaction ID, providing
//! better NAT traversal behavior.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::message_header::MAGIC_COOKIE;

/// Address family values according to RFC 5389.
const FAMILY_IPV4: u8 = 0x01;
const FAMILY_IPV6: u8 = 0x02;

/// Decodes an XOR-MAPPED-ADDRESS attribute value.
///
/// # Arguments
/// * `bytes` - The attribute value bytes
/// * `transaction_id` - The transaction ID from the message header
///
/// # Returns
/// * `Some(SocketAddr)` - If decoding succeeds
/// * `None` - If the format is invalid
pub fn decode(bytes: &[u8], transaction_id: &[u8; 12]) -> Option<SocketAddr> {
    if bytes.len() < 4 {
        return None;
    }

    let family = bytes[1];

    // XOR mask: magic cookie + transaction ID
    let mut xor_mask = Vec::new();
    xor_mask.extend_from_slice(&MAGIC_COOKIE.to_be_bytes());
    xor_mask.extend_from_slice(transaction_id);

    // XOR port
    let xor_port = u16::from_be_bytes([bytes[2], bytes[3]]);
    let port = xor_port ^ (MAGIC_COOKIE >> 16) as u16;

    match family {
        FAMILY_IPV4 => {
            // IPv4
            if bytes.len() < 8 {
                return None;
            }
            let magic_bytes = MAGIC_COOKIE.to_be_bytes();
            let ip = Ipv4Addr::new(
                bytes[4] ^ magic_bytes[0],
                bytes[5] ^ magic_bytes[1],
                bytes[6] ^ magic_bytes[2],
                bytes[7] ^ magic_bytes[3],
            );
            Some(SocketAddr::new(IpAddr::V4(ip), port))
        }
        FAMILY_IPV6 => {
            // IPv6
            if bytes.len() < 20 {
                return None;
            }
            let mut octets = [0u8; 16];
            for i in 0..16 {
                octets[i] = bytes[4 + i] ^ xor_mask[i];
            }
            let ip = Ipv6Addr::from(octets);
            Some(SocketAddr::new(IpAddr::V6(ip), port))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_ipv4() {
        // Pre-calculated XOR-MAPPED-ADDRESS for 192.168.1.100:8080
        // with transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let encoded = vec![
            0x00, // Reserved
            0x01, // Family IPv4
            0x3E, 0x82, // XOR'd port (8080 ^ 0x2112 = 0x3E82)
            0xE1, 0xBA, 0xA5, 0x26, // XOR'd IP (192.168.1.100 ^ 0x2112A442)
        ];

        let decoded = decode(&encoded, &transaction_id);
        let expected = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100)), 8080);
        assert_eq!(Some(expected), decoded);
    }

    #[test]
    fn test_decode_ipv6() {
        // Pre-calculated XOR-MAPPED-ADDRESS for [2001:db8::1]:8080
        // with transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        let encoded = vec![
            0x00, // Reserved
            0x02, // Family IPv6
            0x3E, 0x82, // XOR'd port
            // XOR'd IPv6 (16 bytes)
            0x01, 0x13, 0xA9, 0xFA, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A,
            0x0B, 0x0D,
        ];

        let decoded = decode(&encoded, &transaction_id);
        let expected = SocketAddr::new(
            IpAddr::V6(Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)),
            8080,
        );
        assert_eq!(Some(expected), decoded);
    }

    #[test]
    fn test_decode_invalid_family() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        // Invalid family (0xFF)
        let bytes = vec![0x00, 0xFF, 0x1F, 0x90, 192, 168, 1, 100];
        assert_eq!(None, decode(&bytes, &transaction_id));
    }

    #[test]
    fn test_decode_too_short() {
        let transaction_id = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
        // Buffer too short
        let bytes = vec![0x00, 0x01];
        assert_eq!(None, decode(&bytes, &transaction_id));
    }
}
