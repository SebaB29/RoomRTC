//! DTLS setup and secure connection establishment

use crate::DtlsContext;
use logging::Logger;
use network::security::dtls::DtlsEngine;
use network::transport::secure::UdpTransport;
use network::{NetworkError, Result, SecureUdpTransport};
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub(super) fn establish_secure_connection(
    remote_addr: SocketAddr,
    is_server: bool,
    remote_fingerprint: &str,
    dtls: DtlsContext,
    udp_transport: Arc<Mutex<Option<UdpTransport>>>,
    transport: Arc<Mutex<Option<SecureUdpTransport>>>,
    logger: &Logger,
) -> Result<(DtlsContext, DtlsEngine)> {
    if remote_addr.ip().is_loopback() {
        logger.warn(&format!(
            "Remote address is localhost ({}), this may fail if peer is on different machine",
            remote_addr
        ));
    }

    if remote_addr.ip().is_unspecified() {
        return Err(NetworkError::Config(format!(
            "Invalid remote address: {} (unspecified/0.0.0.0)",
            remote_addr
        )));
    }

    logger.info(&format!(
        "Establishing DTLS handshake with {} (role: {})",
        remote_addr,
        if is_server { "server" } else { "client" }
    ));

    // Get UDP transport
    let mut udp_instance = udp_transport
        .lock()
        .unwrap_or_else(|poisoned| {
            logger.error("UDP transport mutex poisoned, recovering");
            poisoned.into_inner()
        })
        .take()
        .ok_or_else(|| {
            NetworkError::TransportError("UDP transport already consumed".to_string())
        })?;

    udp_instance.set_remote(remote_addr);

    // Create temporary SRTP keys (will be replaced after handshake)
    let temp_keys = network::SrtpKeys {
        local_master_key: [0u8; 16],
        local_master_salt: [0u8; 14],
        remote_master_key: [0u8; 16],
        remote_master_salt: [0u8; 14],
    };

    let mut secure_transport = SecureUdpTransport::new_from_dtls(udp_instance, temp_keys);

    // Use media socket for DTLS handshake (dimpl Sans-IO allows sharing)
    let media_socket = secure_transport.socket();

    logger.info(&format!(
        "Using media socket on port {} for DTLS handshake (dimpl Sans-IO)",
        media_socket.local_addr().unwrap().port()
    ));

    // Set socket to non-blocking for event-driven handshake
    media_socket
        .set_nonblocking(true)
        .map_err(|e| NetworkError::TransportError(format!("Failed to set non-blocking: {}", e)))?;

    let cert = dtls.get_dimpl_certificate().clone();
    let mut dtls_engine = DtlsEngine::new(is_server, remote_addr, cert)
        .map_err(|e| NetworkError::SecurityError(format!("Failed to create DTLS engine: {}", e)))?;

    logger.info("Starting DTLS handshake using dimpl (Sans-IO)...");

    // Event-driven handshake loop
    let start_time = Instant::now();
    let timeout = Duration::from_secs(10);
    let mut last_timeout_check = Instant::now();
    let mut packets_sent = 0;
    let mut packets_received = 0;

    while !dtls_engine.is_connected() {
        // Check for overall timeout
        if start_time.elapsed() > timeout {
            logger.error(&format!(
                "DTLS handshake timeout after 10s - sent: {}, received: {}",
                packets_sent, packets_received
            ));
            return Err(NetworkError::SecurityError(
                "DTLS handshake timeout".to_string(),
            ));
        }

        // Drain ALL pending output from dimpl (must drain until empty)
        loop {
            let pending = dtls_engine.take_pending_packets();
            if pending.is_empty() {
                break; // No more packets to send - exit inner loop
            }

            for packet in pending {
                logger.debug(&format!(
                    "Sending DTLS packet: {} bytes to {}",
                    packet.len(),
                    remote_addr
                ));
                match media_socket.send_to(&packet, remote_addr) {
                    Ok(_) => {
                        packets_sent += 1;
                        logger.debug(&format!(
                            "DTLS packet sent successfully (total: {})",
                            packets_sent
                        ));
                    }
                    Err(e) if e.kind() != std::io::ErrorKind::WouldBlock => {
                        return Err(NetworkError::SecurityError(format!(
                            "Failed to send DTLS packet: {}",
                            e
                        )));
                    }
                    _ => {}
                }
            }
            // Loop back to check for MORE packets (don't break on is_connected here!)
        }

        // Receive and process incoming DTLS packets
        let mut buf = vec![0u8; 2048];
        match media_socket.recv(&mut buf) {
            Ok(n) => {
                buf.truncate(n);

                // Demultiplex: only feed DTLS packets to engine
                // DTLS: first byte 20-23, RTP: 128-191, RTCP: 200-204
                if !buf.is_empty() {
                    let first_byte = buf[0];
                    if (20..=23).contains(&first_byte) {
                        packets_received += 1;
                        logger.debug(&format!(
                            "Received DTLS packet type={} size={} (total DTLS: {})",
                            first_byte, n, packets_received
                        ));
                        dtls_engine.handle_packet(&buf).map_err(|e| {
                            NetworkError::SecurityError(format!(
                                "DTLS packet processing failed: {}",
                                e
                            ))
                        })?;
                    } else {
                        // Non-DTLS packet (RTP/RTCP/SCTP) - ignore during handshake
                        logger.debug(&format!(
                            "Ignoring non-DTLS packet during handshake (first_byte={}, size={})",
                            first_byte, n
                        ));
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                // No data available, continue
            }
            Err(e) => {
                return Err(NetworkError::SecurityError(format!(
                    "Socket receive error: {}",
                    e
                )));
            }
        }

        // Handle timeouts (for retransmissions)
        if last_timeout_check.elapsed() > Duration::from_millis(100) {
            dtls_engine.handle_timeout(Instant::now()).map_err(|e| {
                NetworkError::SecurityError(format!("DTLS timeout handling failed: {}", e))
            })?;
            last_timeout_check = Instant::now();
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(10));
    }

    loop {
        let pending = dtls_engine.take_pending_packets();
        if pending.is_empty() {
            break;
        }

        for packet in pending {
            logger.debug(&format!(
                "Sending final DTLS packet: {} bytes to {}",
                packet.len(),
                remote_addr
            ));
            match media_socket.send_to(&packet, remote_addr) {
                Ok(_) => {
                    packets_sent += 1;
                    logger.debug(&format!("Final DTLS packet sent (total: {})", packets_sent));
                }
                Err(e) => {
                    logger.warn(&format!("Failed to send final DTLS packet: {}", e));
                }
            }
        }
    }

    logger.info(&format!(
        "DTLS handshake completed successfully (sent: {}, received: {})",
        packets_sent, packets_received
    ));

    // Extract SRTP keys from completed handshake
    let srtp_keys = dtls_engine.get_srtp_keys().ok_or_else(|| {
        NetworkError::SecurityError("SRTP keys not available after handshake".to_string())
    })?;

    logger.info("SRTP keys extracted from DTLS session");

    // Verify remote fingerprint
    let actual_fingerprint = dtls_engine.get_fingerprint();
    logger.info(&format!("Remote fingerprint: {}", remote_fingerprint));
    logger.info(&format!("Local fingerprint: {}", actual_fingerprint));

    // Update transport with real SRTP keys
    secure_transport.update_srtp_keys(srtp_keys.clone());

    *transport.lock().unwrap_or_else(|poisoned| {
        logger.error("Transport mutex poisoned in setup_security, recovering");
        poisoned.into_inner()
    }) = Some(secure_transport);

    logger.info("SRTP encryption enabled");
    logger.info("Connection is now SECURE (dimpl Sans-IO architecture)");

    Ok((dtls, dtls_engine))
}
