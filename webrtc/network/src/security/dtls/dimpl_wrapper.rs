//! dimpl DTLS wrapper for WebRTC
//!
//! Provides a Sans-IO DTLS engine that integrates with our UDP demultiplexer

use super::SrtpKeys;
use dimpl::{Config, Dtls, DtlsCertificate, KeyingMaterial, Output, SrtpProfile};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

/// DTLS engine state wrapping dimpl
pub struct DtlsEngine {
    dtls: Dtls,
    cert: DtlsCertificate,
    srtp_keys: Option<SrtpKeys>,
    connected: bool,
    pending_packets: Vec<Vec<u8>>,
    is_server: bool,             // Track role for SRTP key ordering
    incoming_sctp: Vec<Vec<u8>>, // Buffer for received SCTP packets
}

impl DtlsEngine {
    /// Create new DTLS engine using existing certificate (MUST match SDP fingerprint!)
    pub fn new(
        is_server: bool,
        _remote_addr: SocketAddr,
        cert: DtlsCertificate,
    ) -> Result<Self, String> {
        let config = Arc::new(Config::default());
        let mut dtls = Dtls::new(config, cert.clone());

        // Set active (client) or passive (server)
        dtls.set_active(!is_server);

        let mut engine = DtlsEngine {
            dtls,
            cert,
            srtp_keys: None,
            connected: false,
            pending_packets: Vec::new(),
            is_server,                 // Store role for key extraction
            incoming_sctp: Vec::new(), // Initialize SCTP buffer
        };

        // dimpl requires handle_timeout before poll_output
        // For client, this generates the initial ClientHello
        engine
            .dtls
            .handle_timeout(Instant::now())
            .map_err(|e| format!("Failed to initialize DTLS timeout: {:?}", e))?;

        // Drain the initial packets (ClientHello for client)
        engine.process_output()?;

        Ok(engine)
    }

    /// Get local certificate fingerprint for SDP
    pub fn get_fingerprint(&self) -> String {
        // dimpl uses SHA-256, format as colon-separated hex
        use openssl::sha::sha256;

        // Get DER from dimpl certificate (already in DER format)
        let digest = sha256(&self.cert.certificate);

        digest
            .iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<_>>()
            .join(":")
    }

    /// Feed incoming DTLS packet to engine
    pub fn handle_packet(&mut self, packet: &[u8]) -> Result<(), String> {
        self.dtls
            .handle_packet(packet)
            .map_err(|e| format!("DTLS packet handling failed: {:?}", e))?;

        // Drain output after feeding packet
        self.process_output()?;
        Ok(())
    }

    /// Handle timeout (for retransmissions)
    pub fn handle_timeout(&mut self, now: Instant) -> Result<(), String> {
        self.dtls
            .handle_timeout(now)
            .map_err(|e| format!("DTLS timeout handling failed: {:?}", e))?;

        self.process_output()?;
        Ok(())
    }

    /// Process all pending output from dimpl
    fn process_output(&mut self) -> Result<(), String> {
        let mut out_buf = vec![0u8; 2048];

        loop {
            match self.dtls.poll_output(&mut out_buf) {
                Output::Packet(packet) => {
                    // Store packet to send via UDP
                    self.pending_packets.push(packet.to_vec());
                }
                Output::Timeout(_instant) => {
                    // Would schedule timer, but we'll poll regularly instead
                    break;
                }
                Output::Connected => {
                    self.connected = true;
                }
                Output::PeerCert(_der) => {
                    // Could validate peer certificate here
                    // For WebRTC we verify fingerprint from SDP
                }
                Output::KeyingMaterial(km, profile) => {
                    // Extract SRTP keys
                    self.srtp_keys = Some(extract_srtp_keys(&km, &profile, self.is_server)?);
                }
                Output::ApplicationData(data) => {
                    // SCTP data received from remote peer
                    self.incoming_sctp.push(data.to_vec());
                }
            }
        }

        Ok(())
    }

    /// Get packets to send (drain pending)
    pub fn take_pending_packets(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.pending_packets)
    }

    /// Check if DTLS handshake is complete
    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Get extracted SRTP keys (if handshake complete)
    pub fn get_srtp_keys(&self) -> Option<&SrtpKeys> {
        self.srtp_keys.as_ref()
    }

    /// Send application data (SCTP over DTLS)
    pub fn send_application_data(&mut self, data: &[u8]) -> Result<(), String> {
        self.dtls
            .send_application_data(data)
            .map_err(|e| format!("Failed to send application data: {:?}", e))?;

        self.process_output()?;
        Ok(())
    }

    /// Take incoming SCTP packets (drain buffer)
    pub fn take_incoming_sctp(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.incoming_sctp)
    }
}

/// Extract SRTP keys from dimpl KeyingMaterial
fn extract_srtp_keys(
    km: &KeyingMaterial,
    profile: &SrtpProfile,
    is_server: bool,
) -> Result<SrtpKeys, String> {
    // dimpl KeyingMaterial format (RFC 5764):
    // client_write_key | client_write_salt | server_write_key | server_write_salt
    //
    // For CLIENT (active/offerer): local=client_write, remote=server_write
    // For SERVER (passive/answerer): local=server_write, remote=client_write

    match profile {
        SrtpProfile::Aes128CmSha1_80 => {
            // AES-128: 16-byte keys, 14-byte salts (or 5 for dimpl compact format)
            extract_keys_with_params(km, 16, 14, is_server)
        }
        SrtpProfile::AeadAes128Gcm => {
            // AES-128-GCM: 16-byte keys, 12-byte salts
            extract_keys_with_params(km, 16, 12, is_server)
        }
        SrtpProfile::AeadAes256Gcm => {
            // AES-256-GCM: 32-byte keys, 12-byte salts
            extract_keys_with_params(km, 32, 12, is_server)
        }
    }
}

fn extract_keys_with_params(
    km: &KeyingMaterial,
    key_len: usize,
    salt_len: usize,
    is_server: bool,
) -> Result<SrtpKeys, String> {
    // Validate total length
    let expected_len = key_len * 2 + salt_len * 2;
    if km.len() != expected_len {
        return Err(format!(
            "Invalid keying material length: {} (expected {} for key={}, salt={})",
            km.len(),
            expected_len,
            key_len,
            salt_len
        ));
    }

    // Extract from KeyingMaterial: client_key | client_salt | server_key | server_salt
    let actual_key_len = std::cmp::min(key_len, 16);
    let actual_salt_len = std::cmp::min(salt_len, 14);

    let client_key_offset = 0;
    let client_salt_offset = key_len;
    let server_key_offset = key_len + salt_len;
    let server_salt_offset = key_len + salt_len + key_len;

    let mut client_key = [0u8; 16];
    let mut client_salt = [0u8; 14];
    let mut server_key = [0u8; 16];
    let mut server_salt = [0u8; 14];

    client_key[..actual_key_len]
        .copy_from_slice(&km[client_key_offset..client_key_offset + actual_key_len]);
    client_salt[..actual_salt_len]
        .copy_from_slice(&km[client_salt_offset..client_salt_offset + actual_salt_len]);
    server_key[..actual_key_len]
        .copy_from_slice(&km[server_key_offset..server_key_offset + actual_key_len]);
    server_salt[..actual_salt_len]
        .copy_from_slice(&km[server_salt_offset..server_salt_offset + actual_salt_len]);

    // Assign local/remote based on role
    let (local_master_key, local_master_salt, remote_master_key, remote_master_salt) = if is_server
    {
        // Server: local=server_write, remote=client_write
        (server_key, server_salt, client_key, client_salt)
    } else {
        // Client: local=client_write, remote=server_write
        (client_key, client_salt, server_key, server_salt)
    };

    Ok(SrtpKeys {
        local_master_key,
        local_master_salt,
        remote_master_key,
        remote_master_salt,
    })
}
