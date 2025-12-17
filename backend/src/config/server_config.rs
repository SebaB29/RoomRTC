use json_parser::impl_json;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub bind_address: String,
    pub port: u32,
    pub max_connections: usize,
    pub enable_tls: bool,
    pub pkcs12_path: Option<String>,
    pub pkcs12_password: Option<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            bind_address: "127.0.0.1".to_string(),
            port: 8080,
            max_connections: 100,
            enable_tls: false,
            pkcs12_path: None,
            pkcs12_password: None,
        }
    }
}

impl_json! {
    ServerConfig {
        bind_address: String,
        port: u32,
        max_connections: usize,
        enable_tls: bool,
        pkcs12_path: Option<String>,
        pkcs12_password: Option<String>,
    }
}
