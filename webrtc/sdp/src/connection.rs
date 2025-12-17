//! SDP connection data representation.
//!
//! Defines the connection information (c= line) for establishing
//! network connections in SDP sessions.

use std::net::IpAddr;

/// Represents the connection data field (c=) in an SDP message as specified in RFC 4566.
///
/// The connection field contains the information required to establish a network connection.
///
/// According to RFC 4566 section 5.7, the connection field has the following format:
/// ```text
/// c=<network-type> <address-type> <connection-address>
/// ```
#[derive(Debug, Clone)]
pub struct Connection {
    pub network_type: String,
    pub address_type: String,
    pub address: IpAddr,
    pub ttl: Option<u8>,
    pub num_addresses: Option<u8>,
}

impl Connection {
    /// Parses a connection data string into a Connection struct according to RFC 4566.
    ///
    /// The connection field specifies the connection data for the session or media stream.
    ///
    /// # Arguments
    /// * `value` - The connection data string to parse, without the leading "c="
    ///
    /// # Returns
    /// * `Ok(Connection)` - A successfully parsed connection field
    /// * `Err(SdpError)` - If the format is invalid or any component is incorrect
    pub fn parse(value: &str) -> Result<Self, crate::errors::SdpError> {
        let parts: Vec<&str> = value.split_whitespace().collect();
        if parts.len() < 3 {
            return Err(crate::errors::SdpError::InvalidOriginFormat);
        }

        // Validate network and address types
        Self::validate_network_type(parts[0])?;
        Self::validate_address_type(parts[1])?;

        // Parse address information
        let (address, ttl, num_addresses) = Self::parse_address_info(parts[2])?;

        Ok(Connection {
            network_type: parts[0].to_string(),
            address_type: parts[1].to_string(),
            address,
            ttl,
            num_addresses,
        })
    }

    /// Validates the network type string according to RFC 4566.
    ///
    /// According to the RFC, the network type must be "IN" (Internet).
    ///
    /// # Arguments
    /// * `network_type` - The network type string to validate
    ///
    /// # Returns
    /// * `Ok(())` - If the network type is valid
    /// * `Err(SdpError::InvalidNetworkType)` - If the network type is not "IN"
    fn validate_network_type(network_type: &str) -> Result<(), crate::errors::SdpError> {
        if network_type != "IN" {
            return Err(crate::errors::SdpError::InvalidNetworkType);
        }
        Ok(())
    }

    /// Validates the address type string according to RFC 4566.
    ///
    /// The address type must be either "IP4" or "IP6".
    ///
    /// # Arguments
    /// * `address_type` - The address type string to validate
    ///
    /// # Returns
    /// * `Ok(())` - If the address type is valid
    /// * `Err(SdpError::InvalidAddressType)` - If the address type is neither "IP4" nor "IP6"
    fn validate_address_type(address_type: &str) -> Result<(), crate::errors::SdpError> {
        if address_type != "IP4" && address_type != "IP6" {
            return Err(crate::errors::SdpError::InvalidAddressType);
        }
        Ok(())
    }

    /// Parses the connection address information, including optional TTL and number of addresses.
    ///
    /// The connection address has different formats depending on the address type and whether
    /// it's being used for unicast or multicast:
    /// - For unicast: A single IP address
    /// - For multicast: IP address with optional TTL
    /// - For address ranges: IP address with TTL and number of addresses
    ///
    /// # Arguments
    /// * `address_str` - The connection address string to parse, in the format:
    ///   `<ip-address>[/<ttl>][/<number-of-addresses>]`
    ///
    /// # Returns
    /// * `Ok((IpAddr, Option<u8>, Option<u8>))` - The parsed IP address, TTL, and number of addresses
    /// * `Err(SdpError)` - If any part of the address string is invalid
    fn parse_address_info(
        address_str: &str,
    ) -> Result<(IpAddr, Option<u8>, Option<u8>), crate::errors::SdpError> {
        let parts: Vec<&str> = address_str.split('/').collect();

        // Parse IP address
        let address = parts[0]
            .parse()
            .map_err(|_| crate::errors::SdpError::InvalidIpAddress)?;

        // Parse TTL if present
        let ttl = if parts.len() > 1 {
            Some(
                parts[1]
                    .parse()
                    .map_err(|_| crate::errors::SdpError::InvalidTtl)?,
            )
        } else {
            None
        };

        // Parse number of addresses if present
        let num_addresses = if parts.len() > 2 {
            Some(
                parts[2]
                    .parse()
                    .map_err(|_| crate::errors::SdpError::InvalidAddressCount)?,
            )
        } else {
            None
        };

        Ok((address, ttl, num_addresses))
    }
}

/// Implements the Display trait to format connection data according to RFC 4566.
///
/// The connection field is formatted as:
/// ```text
/// c=<network-type> <address-type> <connection-address>
/// ```
impl std::fmt::Display for Connection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "c={} {} {}",
            self.network_type, self.address_type, self.address
        )?;
        if let Some(ttl) = self.ttl {
            write!(f, "/{}", ttl)?;
            if let Some(num_addresses) = self.num_addresses {
                write!(f, "/{}", num_addresses)?;
            }
        }
        writeln!(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    #[test]
    fn test_connection_parse_ipv4() {
        let conn = Connection::parse("IN IP4 192.168.1.1").unwrap();
        assert_eq!(conn.network_type, "IN");
        assert_eq!(conn.address_type, "IP4");
        assert_eq!(conn.address, "192.168.1.1".parse::<IpAddr>().unwrap());
        assert_eq!(conn.ttl, None);
        assert_eq!(conn.num_addresses, None);
    }

    #[test]
    fn test_connection_parse_ipv6() {
        let conn = Connection::parse("IN IP6 ::1").unwrap();
        assert_eq!(conn.network_type, "IN");
        assert_eq!(conn.address_type, "IP6");
        assert_eq!(conn.address, "::1".parse::<IpAddr>().unwrap());
        assert_eq!(conn.ttl, None);
        assert_eq!(conn.num_addresses, None);
    }

    #[test]
    fn test_connection_parse_with_ttl() {
        let conn = Connection::parse("IN IP4 224.2.1.1/127").unwrap();
        assert_eq!(conn.network_type, "IN");
        assert_eq!(conn.address_type, "IP4");
        assert_eq!(conn.address, "224.2.1.1".parse::<IpAddr>().unwrap());
        assert_eq!(conn.ttl, Some(127));
        assert_eq!(conn.num_addresses, None);
    }

    #[test]
    fn test_connection_parse_with_ttl_and_count() {
        let conn = Connection::parse("IN IP4 224.2.1.1/127/3").unwrap();
        assert_eq!(conn.network_type, "IN");
        assert_eq!(conn.address_type, "IP4");
        assert_eq!(conn.address, "224.2.1.1".parse::<IpAddr>().unwrap());
        assert_eq!(conn.ttl, Some(127));
        assert_eq!(conn.num_addresses, Some(3));
    }

    #[test]
    fn test_connection_parse_invalid_format() {
        assert!(Connection::parse("IN IP4").is_err());
        assert!(Connection::parse("IN").is_err());
        assert!(Connection::parse("").is_err());
    }

    #[test]
    fn test_connection_parse_invalid_network_type() {
        assert!(Connection::parse("XX IP4 192.168.1.1").is_err());
    }

    #[test]
    fn test_connection_parse_invalid_address_type() {
        assert!(Connection::parse("IN IP5 192.168.1.1").is_err());
    }

    #[test]
    fn test_connection_parse_invalid_ip_address() {
        assert!(Connection::parse("IN IP4 999.999.999.999").is_err());
        assert!(Connection::parse("IN IP4 invalid").is_err());
    }

    #[test]
    fn test_connection_parse_invalid_ttl() {
        assert!(Connection::parse("IN IP4 224.2.1.1/abc").is_err());
        assert!(Connection::parse("IN IP4 224.2.1.1/256").is_err());
    }

    #[test]
    fn test_connection_parse_invalid_address_count() {
        assert!(Connection::parse("IN IP4 224.2.1.1/127/abc").is_err());
    }

    #[test]
    fn test_connection_display_simple() {
        let conn = Connection {
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            address: "192.168.1.1".parse().unwrap(),
            ttl: None,
            num_addresses: None,
        };
        assert_eq!(format!("{}", conn), "c=IN IP4 192.168.1.1\n");
    }

    #[test]
    fn test_connection_display_with_ttl() {
        let conn = Connection {
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            address: "224.2.1.1".parse().unwrap(),
            ttl: Some(127),
            num_addresses: None,
        };
        assert_eq!(format!("{}", conn), "c=IN IP4 224.2.1.1/127\n");
    }

    #[test]
    fn test_connection_display_with_ttl_and_count() {
        let conn = Connection {
            network_type: "IN".to_string(),
            address_type: "IP4".to_string(),
            address: "224.2.1.1".parse().unwrap(),
            ttl: Some(127),
            num_addresses: Some(3),
        };
        assert_eq!(format!("{}", conn), "c=IN IP4 224.2.1.1/127/3\n");
    }
}
