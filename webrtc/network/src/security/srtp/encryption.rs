//! SRTP encryption and decryption operations

use crate::error::{NetworkError, Result};
use aes::Aes128;
use ctr::Ctr128BE;
use ctr::cipher::{KeyIvInit, StreamCipher};
use hmac::{Hmac, Mac};
use sha1::Sha1;

type HmacSha1 = Hmac<Sha1>;
type Aes128Ctr = Ctr128BE<Aes128>;

/// Derives session encryption key from  master key using AES-CTR
pub fn derive_session_key(master_key: &[u8; 16], ssrc: u32, label: u8) -> [u8; 16] {
    // Build key derivation input: label || ssrc || 0x00...
    let mut input = [0u8; 16];
    input[0] = label;
    let ssrc_bytes = ssrc.to_be_bytes();
    input[1..5].copy_from_slice(&ssrc_bytes);

    // AES-CTR mode with master key and input as IV
    let cipher = Aes128Ctr::new(master_key.into(), &input.into());
    let mut output = [0u8; 16];
    let plaintext = [0u8; 16];

    // Encrypt zeros to generate derived key
    let mut cipher_clone = cipher.clone();
    cipher_clone
        .apply_keystream_b2b(&plaintext, &mut output)
        .expect("AES-CTR keystream operation failed - cipher state corrupted");

    output
}

/// Derives session salt from master salt
pub fn derive_session_salt(master_salt: &[u8; 14], ssrc: u32) -> [u8; 14] {
    let mut salt = [0u8; 14];
    salt.copy_from_slice(master_salt);

    // XOR with SSRC
    let ssrc_bytes = ssrc.to_be_bytes();
    for i in 0..4 {
        salt[i] ^= ssrc_bytes[i];
    }

    salt
}

/// Derives authentication key (20 bytes for HMAC-SHA1)
pub fn derive_auth_key(master_key: &[u8; 16], ssrc: u32) -> [u8; 20] {
    let key16 = derive_session_key(master_key, ssrc, 0x01);
    let mut key20 = [0u8; 20];
    key20[..16].copy_from_slice(&key16);
    // Extend to 20 bytes by repeating first 4 bytes
    key20[16..].copy_from_slice(&key16[..4]);
    key20
}

/// Builds IV for CTR mode encryption
pub fn build_iv(salt: &[u8; 14], _ssrc: u32, seq: u16) -> [u8; 16] {
    let mut iv = [0u8; 16];

    // Copy salt
    iv[..14].copy_from_slice(salt);

    // XOR with packet index
    let seq_bytes = seq.to_be_bytes();
    iv[4] ^= seq_bytes[0];
    iv[5] ^= seq_bytes[1];

    iv
}

/// Encrypts payload using AES-CTR
pub fn encrypt_payload(payload: &mut [u8], key: &[u8; 16], iv: &[u8; 16]) {
    if !payload.is_empty() {
        let mut cipher = Aes128Ctr::new(key.into(), iv.into());
        cipher.apply_keystream(payload);
    }
}

/// Computes HMAC-SHA1 authentication tag (truncated to 10 bytes)
pub fn compute_auth_tag(key: &[u8; 20], data: &[u8]) -> Result<[u8; 10]> {
    let mut mac = HmacSha1::new_from_slice(key)
        .map_err(|e| NetworkError::CryptoError(format!("HMAC init failed: {}", e)))?;

    mac.update(data);

    let result = mac.finalize();
    let tag_bytes = result.into_bytes();

    // Truncate to 10 bytes per RFC 3711
    let mut tag = [0u8; 10];
    tag.copy_from_slice(&tag_bytes[..10]);

    Ok(tag)
}
