use std::fmt;

/// Tipo de resultado usado en toda la librería
pub type Result<T> = std::result::Result<T, ConfigError>;

/// Errores que pueden ocurrir durante la carga de configuración
#[derive(Debug)]
pub enum ConfigError {
    /// El archivo de configuración no fue encontrado
    FileNotFound(String),

    /// Error al leer el archivo
    ReadError(String),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::FileNotFound(path) => {
                write!(f, "Archivo de configuración no encontrado: {}", path)
            }
            ConfigError::ReadError(msg) => {
                write!(f, "Error al leer archivo de configuración: {}", msg)
            }
        }
    }
}

impl std::error::Error for ConfigError {}
