//! DTLS (Datagram Transport Layer Security) for WebRTC
//!
//! Provides secure key exchange using DTLS-SRTP for WebRTC connections

mod certificate;
mod dimpl_wrapper;

pub use certificate::compute_fingerprint;
pub use dimpl::DtlsCertificate;
pub use dimpl_wrapper::DtlsEngine;

/// SRTP key material extracted from DTLS
#[derive(Debug, Clone)]
pub struct SrtpKeys {
    pub local_master_key: [u8; 16],
    pub local_master_salt: [u8; 14],
    pub remote_master_key: [u8; 16],
    pub remote_master_salt: [u8; 14],
}

/// DTLS context for WebRTC
pub struct DtlsContext {
    dimpl_cert: dimpl::DtlsCertificate,
    local_fingerprint: String,
}

impl DtlsContext {
    /// Create a new DTLS context with self-signed certificate (uses dimpl for compatibility)
    pub fn new() -> Result<Self, String> {
        // Generate certificate using dimpl (compatible with dimpl's DTLS engine)
        let dimpl_cert = dimpl::certificate::generate_self_signed_certificate()
            .map_err(|e| format!("Failed to generate dimpl certificate: {}", e))?;

        // Compute fingerprint from dimpl's certificate (SHA-256 of DER)
        use openssl::sha::sha256;
        let digest = sha256(&dimpl_cert.certificate);
        let local_fingerprint = digest
            .iter()
            .map(|byte| format!("{:02X}", byte))
            .collect::<Vec<_>>()
            .join(":");

        Ok(DtlsContext {
            dimpl_cert,
            local_fingerprint,
        })
    }

    /// Get the local certificate fingerprint for SDP
    pub fn get_fingerprint(&self) -> &str {
        &self.local_fingerprint
    }

    /// Get dimpl certificate for DTLS engine
    pub fn get_dimpl_certificate(&self) -> &dimpl::DtlsCertificate {
        &self.dimpl_cert
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dtls_context_creation() {
        let ctx = DtlsContext::new();
        assert!(ctx.is_ok());

        if let Ok(ctx) = ctx {
            let fingerprint = ctx.get_fingerprint();
            // Should be hex format: "XX:XX:XX:..."
            assert!(fingerprint.contains(":"));
            assert!(!fingerprint.contains("sha-256")); // No prefix
        }
    }

    #[test]
    fn test_fingerprint_format() {
        let ctx = DtlsContext::new().unwrap();
        let fp = ctx.get_fingerprint();

        // Should be "XX:XX:XX:..." (32 bytes for SHA-256)
        let bytes: Vec<&str> = fp.split(':').collect();
        assert_eq!(bytes.len(), 32); // SHA-256 = 32 bytes

        // Each byte should be 2 hex digits
        for byte in bytes {
            assert_eq!(byte.len(), 2);
            assert!(byte.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
