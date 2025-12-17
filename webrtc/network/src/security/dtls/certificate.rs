//! DTLS certificate generation and management

use openssl::x509::X509;

/// Compute SHA-256 fingerprint of certificate
pub fn compute_fingerprint(cert: &X509) -> Result<String, String> {
    let der = cert
        .to_der()
        .map_err(|e| format!("Failed to encode certificate: {}", e))?;

    let digest = openssl::sha::sha256(&der);

    // Format as colon-separated hex (without algorithm prefix)
    let fingerprint = digest
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<_>>()
        .join(":");

    Ok(fingerprint)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_generation() {
        let result = dimpl::certificate::generate_self_signed_certificate();
        assert!(result.is_ok());

        if let Ok(cert) = result {
            let x509 =
                openssl::x509::X509::from_der(&cert.certificate).expect("Invalid DER certificate");
            let fingerprint = compute_fingerprint(&x509);
            assert!(fingerprint.is_ok());
        }
    }

    #[test]
    fn test_fingerprint_format() {
        let cert = dimpl::certificate::generate_self_signed_certificate().unwrap();
        let x509 =
            openssl::x509::X509::from_der(&cert.certificate).expect("Invalid DER certificate");
        let fp = compute_fingerprint(&x509).unwrap();

        // Should be "XX:XX:XX:..." (32 bytes for SHA-256)§
        let bytes: Vec<&str> = fp.split(':').collect();
        assert_eq!(bytes.len(), 32); // SHA-256 = 32 bytes

        // Each byte should be 2 hex digits
        for byte in bytes {
            assert_eq!(byte.len(), 2);
            assert!(byte.chars().all(|c| c.is_ascii_hexdigit()));
        }
    }
}
