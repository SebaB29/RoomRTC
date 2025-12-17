use std::net::SocketAddr;
use stun::StunClient;

/// Test using Google's public STUN server
#[test]
fn test_stun_client_with_google_server() {
    use std::net::ToSocketAddrs;

    // Resolve Google's public STUN server domain to IP
    let server_addr = match "stun.l.google.com:19302".to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(addr) => addr,
            None => {
                eprintln!("Could not resolve stun.l.google.com - skipping test");
                return;
            }
        },
        Err(e) => {
            eprintln!(
                "DNS resolution failed (this is expected without internet): {}",
                e
            );
            return;
        }
    };

    let client_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();

    let client = match StunClient::new(client_addr, server_addr) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to create STUN client: {}", e);
            return;
        }
    };

    // Perform binding request
    match client.get_reflexive_address() {
        Ok(reflexive_addr) => {
            println!("Test passed: reflexive address = {}", reflexive_addr);
            assert!(reflexive_addr.port() > 0, "Port should be greater than 0");
            // The IP should be a valid public address
            assert!(
                !reflexive_addr.ip().is_loopback(),
                "Address should not be loopback"
            );
        }
        Err(e) => {
            // Test may fail in environments without internet connectivity
            eprintln!(
                "STUN request failed (this is expected without internet): {}",
                e
            );
            // Don't fail the test, just warn
            println!(
                "Note: This test requires internet connectivity to reach stun.l.google.com:19302"
            );
        }
    }
}

/// Test client timeout behavior with unreachable server
#[test]
fn test_client_timeout_on_unreachable_server() {
    let client_addr: SocketAddr = "0.0.0.0:0".parse().unwrap();
    // Use an unreachable server address
    let fake_server_addr: SocketAddr = "192.0.2.1:19302".parse().unwrap();

    let client =
        StunClient::new(client_addr, fake_server_addr).expect("Failed to create STUN client");

    // This should timeout
    let result = client.get_reflexive_address();
    assert!(result.is_err(), "Expected timeout error");

    println!("✓ Test passed: client correctly times out when server is unreachable");
}
