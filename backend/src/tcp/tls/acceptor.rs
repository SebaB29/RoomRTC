//! TLS acceptor configuration and loading.

use native_tls::{Identity, TlsAcceptor};
use std::fs::File;
use std::io::Read;
use std::sync::Arc;

use super::error::TlsError;

/// Load TLS server configuration from PKCS#12 file
///
/// # Arguments
/// * `pkcs12_path` - Path to the PKCS#12 certificate file
/// * `password` - Password to decrypt the certificate
///
/// # Returns
/// Arc-wrapped TLS acceptor ready to accept connections
pub fn load_tls_acceptor(pkcs12_path: &str, password: &str) -> Result<Arc<TlsAcceptor>, TlsError> {
    // Read PKCS#12 file
    let mut file = File::open(pkcs12_path)
        .map_err(|e| TlsError::InvalidCertificate(format!("Cannot open {}: {}", pkcs12_path, e)))?;

    let mut identity_data = Vec::new();
    file.read_to_end(&mut identity_data)?;

    // Validate certificate data
    if identity_data.is_empty() {
        return Err(TlsError::InvalidCertificate(
            "Certificate file is empty".to_string(),
        ));
    }

    // Parse identity
    let identity = Identity::from_pkcs12(&identity_data, password)
        .map_err(|e| TlsError::InvalidCertificate(format!("Invalid PKCS#12 format: {}", e)))?;

    // Build TLS acceptor
    let acceptor = TlsAcceptor::new(identity)?;

    Ok(Arc::new(acceptor))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent_cert() {
        let result = load_tls_acceptor("nonexistent.pfx", "password");
        assert!(result.is_err());
    }

    #[test]
    fn test_empty_path() {
        let result = load_tls_acceptor("", "password");
        assert!(result.is_err());
    }
}
