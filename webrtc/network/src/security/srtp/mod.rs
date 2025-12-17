//! SRTP (Secure RTP) Implementation
//!
//! Provides encryption and authentication for RTP packets to ensure confidentiality
//! and integrity of media streams.

mod encryption;
mod replay;

use crate::codec::rtp::RtpPacket;
use crate::error::{NetworkError, Result};
use replay::ReplayWindow;
use std::collections::HashMap;

/// SRTP context for encrypting/decrypting RTP packets
pub struct SrtpContext {
    master_key: [u8; 16],
    master_salt: [u8; 14],
    replay_windows: HashMap<u32, ReplayWindow>,
}

impl SrtpContext {
    /// Creates a new SRTP context with the given master key and salt
    pub fn new(master_key: [u8; 16], master_salt: [u8; 14]) -> Self {
        Self {
            master_key,
            master_salt,
            replay_windows: HashMap::new(),
        }
    }

    /// Encrypts an RTP packet into an SRTP packet
    pub fn protect(&mut self, packet: &RtpPacket) -> Result<Vec<u8>> {
        let mut rtp_bytes = packet.to_bytes();
        self.validate_packet_size(&rtp_bytes)?;

        let ssrc = packet.header.ssrc;
        let seq_num = packet.header.sequence_number;

        self.encrypt_packet_payload(&mut rtp_bytes, ssrc, seq_num);
        let auth_tag = self.authenticate_packet(&rtp_bytes, ssrc)?;

        rtp_bytes.extend_from_slice(&auth_tag);
        Ok(rtp_bytes)
    }

    /// Decrypts an SRTP packet into an RTP packet
    pub fn unprotect(&mut self, srtp_bytes: &[u8]) -> Result<RtpPacket> {
        const AUTH_TAG_LEN: usize = 10;

        if srtp_bytes.len() < 12 + AUTH_TAG_LEN {
            return Err(NetworkError::InvalidPacket("SRTP packet too short".into()));
        }

        let (rtp_bytes, received_tag) = self.split_packet_and_tag(srtp_bytes, AUTH_TAG_LEN);
        let (ssrc, seq_num) = self.parse_header_fields(rtp_bytes)?;

        self.check_replay(ssrc, seq_num)?;
        self.verify_authentication(rtp_bytes, received_tag, ssrc)?;

        self.decrypt_and_parse(rtp_bytes, ssrc, seq_num)
    }

    fn validate_packet_size(&self, rtp_bytes: &[u8]) -> Result<()> {
        if rtp_bytes.len() < 12 {
            return Err(NetworkError::InvalidPacket("RTP packet too short".into()));
        }
        Ok(())
    }

    fn encrypt_packet_payload(&self, rtp_bytes: &mut [u8], ssrc: u32, seq_num: u16) {
        let session_key = encryption::derive_session_key(&self.master_key, ssrc, 0x00);
        let session_salt = encryption::derive_session_salt(&self.master_salt, ssrc);
        let iv = encryption::build_iv(&session_salt, ssrc, seq_num);

        let header_len = 12;
        if rtp_bytes.len() > header_len {
            encryption::encrypt_payload(&mut rtp_bytes[header_len..], &session_key, &iv);
        }
    }

    fn authenticate_packet(&self, rtp_bytes: &[u8], ssrc: u32) -> Result<[u8; 10]> {
        let auth_key = encryption::derive_auth_key(&self.master_key, ssrc);
        encryption::compute_auth_tag(&auth_key, rtp_bytes)
    }

    fn split_packet_and_tag<'a>(
        &self,
        srtp_bytes: &'a [u8],
        tag_len: usize,
    ) -> (&'a [u8], &'a [u8]) {
        let rtp_len = srtp_bytes.len() - tag_len;
        (&srtp_bytes[..rtp_len], &srtp_bytes[rtp_len..])
    }

    fn parse_header_fields(&self, rtp_bytes: &[u8]) -> Result<(u32, u16)> {
        let ssrc = crate::codec::rtp::parse_u32_be(rtp_bytes, 8);
        let seq_num = crate::codec::rtp::parse_u16_be(rtp_bytes, 2);
        Ok((ssrc, seq_num))
    }

    fn check_replay(&mut self, ssrc: u32, seq_num: u16) -> Result<()> {
        let window = self
            .replay_windows
            .entry(ssrc)
            .or_insert_with(|| ReplayWindow::new(64));

        if !window.check_and_update(seq_num as u64) {
            return Err(NetworkError::InvalidPacket("Replay attack detected".into()));
        }
        Ok(())
    }

    fn verify_authentication(
        &self,
        rtp_bytes: &[u8],
        received_tag: &[u8],
        ssrc: u32,
    ) -> Result<()> {
        let auth_key = encryption::derive_auth_key(&self.master_key, ssrc);
        let computed_tag = encryption::compute_auth_tag(&auth_key, rtp_bytes)?;

        if received_tag != &computed_tag[..] {
            return Err(NetworkError::InvalidPacket("Authentication failed".into()));
        }
        Ok(())
    }

    fn decrypt_and_parse(&self, rtp_bytes: &[u8], ssrc: u32, seq_num: u16) -> Result<RtpPacket> {
        let session_key = encryption::derive_session_key(&self.master_key, ssrc, 0x00);
        let session_salt = encryption::derive_session_salt(&self.master_salt, ssrc);
        let iv = encryption::build_iv(&session_salt, ssrc, seq_num);

        let mut decrypted = rtp_bytes.to_vec();
        let header_len = 12;
        if decrypted.len() > header_len {
            encryption::encrypt_payload(&mut decrypted[header_len..], &session_key, &iv);
        }

        RtpPacket::from_bytes(&decrypted)
    }

    /// Reset replay protection windows for all SSRCs
    /// Call this when the remote peer's stream restarts (e.g., camera toggled off/on)
    pub fn reset_replay_protection(&mut self) {
        self.replay_windows.clear();
    }

    /// Reset replay protection for a specific SSRC
    /// Useful when only one stream restarts
    pub fn reset_replay_protection_for_ssrc(&mut self, ssrc: u32) {
        self.replay_windows.remove(&ssrc);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::rtp::{RtpHeader, RtpPacket};

    #[test]
    fn test_srtp_encrypt_decrypt() {
        let master_key = [
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F,
        ];
        let master_salt = [
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D,
        ];

        let mut tx_context = SrtpContext::new(master_key, master_salt);
        let mut rx_context = SrtpContext::new(master_key, master_salt);

        let mut header = RtpHeader::new(96, 12345);
        header.sequence_number = 100;
        header.timestamp = 90000;
        let payload = b"Hello, SRTP!".to_vec();
        let original_packet = RtpPacket::new(header, payload.clone());

        let encrypted = tx_context.protect(&original_packet).unwrap();

        // Verify payload is encrypted
        assert_ne!(&encrypted[12..encrypted.len() - 10], &payload[..]);

        let decrypted = rx_context.unprotect(&encrypted).unwrap();

        // Verify payload matches original
        assert_eq!(decrypted.payload, payload);
        assert_eq!(decrypted.header.sequence_number, 100);
    }

    #[test]
    fn test_replay_protection() {
        let master_key = [1u8; 16];
        let master_salt = [2u8; 14];

        let mut tx_context = SrtpContext::new(master_key, master_salt);
        let mut rx_context = SrtpContext::new(master_key, master_salt);

        let header = RtpHeader::new(96, 12345);
        let packet = RtpPacket::new(header, vec![1, 2, 3]);
        let encrypted = tx_context.protect(&packet).unwrap();

        // First decrypt should succeed
        assert!(rx_context.unprotect(&encrypted).is_ok());

        // Replay should fail
        assert!(rx_context.unprotect(&encrypted).is_err());
    }
}
