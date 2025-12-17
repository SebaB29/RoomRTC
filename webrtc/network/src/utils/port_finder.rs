//! UDP port discovery utilities

use logging::Logger;
use std::error::Error;
use std::net::UdpSocket;

/// Finds an available UDP port starting from the given port.
///
/// Tries up to 100 sequential ports to find one that's available.
///
/// # Arguments
/// * `start` - Starting port number to try
/// * `logger` - Logger for warnings when the preferred port is occupied
///
/// # Returns
/// * `Ok(port)` - An available port number
/// * `Err(_)` - If no port is available in the range [start, start+100)
pub fn find_available_port(start: u16, logger: &Logger) -> Result<u16, Box<dyn Error>> {
    (start..start + 100)
        .find(|&port| is_port_available(port))
        .inspect(|&port| {
            if port != start {
                logger.warn(&format!("Port {} occupied, using {} instead", start, port));
            }
        })
        .ok_or_else(|| format!("No available ports in range {}-{}", start, start + 100).into())
}

fn is_port_available(port: u16) -> bool {
    UdpSocket::bind(("0.0.0.0", port)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use logging::LogLevel;

    fn create_test_logger() -> Logger {
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let log_path = dir.path().join("test.log");
        logging::Logger::new(log_path, LogLevel::Debug).unwrap()
    }

    #[test]
    fn test_find_available_port() {
        let logger = create_test_logger();

        // Should find some port
        let port = find_available_port(50000, &logger);
        assert!(port.is_ok());

        let port_num = port.unwrap();
        assert!((50000..50100).contains(&port_num));
    }

    #[test]
    fn test_find_available_port_returns_start_if_available() {
        let logger = create_test_logger();

        // Use a high port that's likely to be available
        let start_port = 54321;
        let port = find_available_port(start_port, &logger).unwrap();

        // Should return the start port if available
        assert_eq!(port, start_port);
    }

    #[test]
    fn test_find_available_port_with_occupied() {
        let logger = create_test_logger();

        // Bind to a port to occupy it
        let occupied_port = 55000;
        let _socket = UdpSocket::bind(("0.0.0.0", occupied_port)).unwrap();

        // Try to find port starting from occupied port
        let port = find_available_port(occupied_port, &logger).unwrap();

        // Should return a different port
        assert_ne!(port, occupied_port);
        assert!(port > occupied_port);
        assert!(port < occupied_port + 100);
    }

    #[test]
    fn test_find_available_port_range() {
        let logger = create_test_logger();

        let start = 56000;
        let port = find_available_port(start, &logger).unwrap();

        // Port should be within the search range
        assert!(port >= start);
        assert!(port < start + 100);
    }

    #[test]
    fn test_find_available_port_multiple_calls() {
        let logger = create_test_logger();

        // Finding multiple ports should work
        let port1 = find_available_port(57000, &logger);
        let port2 = find_available_port(58000, &logger);
        let port3 = find_available_port(59000, &logger);

        assert!(port1.is_ok());
        assert!(port2.is_ok());
        assert!(port3.is_ok());
    }

    #[test]
    fn test_find_available_port_low_range() {
        let logger = create_test_logger();

        // Test with lower port numbers (but not privileged)
        let port = find_available_port(10000, &logger);
        assert!(port.is_ok());
    }

    #[test]
    fn test_find_available_port_high_range() {
        let logger = create_test_logger();

        // Test with higher port numbers (close to max)
        let port = find_available_port(60000, &logger);
        assert!(port.is_ok());

        let port_num = port.unwrap();
        assert!(port_num >= 60000);
        assert!(port_num < 65535);
    }

    #[test]
    fn test_port_actually_bindable() {
        let logger = create_test_logger();

        let port = find_available_port(40000, &logger).unwrap();

        // Verify we can actually bind to the returned port
        let bind_result = UdpSocket::bind(("0.0.0.0", port));
        assert!(bind_result.is_ok());
    }
}
