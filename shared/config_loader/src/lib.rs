//! # Config Loader
//!
//! Una librería genérica para cargar archivos de configuración desde disco.
//!
//! ```no_run
//! use config_loader::{load_config_file, find_config_file};
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Opción 1: Buscar automáticamente
//!     let path = find_config_file("config.toml")?;
//!     let content = load_config_file(&path)?;
//!
//!     // Opción 2: Path específico
//!     let content = load_config_file("./config/my_config.json")?;
//!
//!     // El consumidor decide cómo parsear
//!     // Ejemplo: usar json_parser
//!     // let json_value = json_parser::parse(&content)?;
//!     // let my_config = MyConfig::from_json(json_value)?;
//!
//!     Ok(())
//! }
//! ```

pub mod error;

pub use error::{ConfigError, Result};

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Carga el contenido de un archivo de configuración.
///
/// Lee el archivo especificado y retorna su contenido como String.
/// No parsea ni valida el contenido - eso es responsabilidad del consumidor.
///
/// # Ejemplos
///
/// ```no_run
/// use config_loader::load_config_file;
///
/// let content = load_config_file("./config/config.json")?;
/// println!("Config content: {}", content);
/// # Ok::<(), config_loader::ConfigError>(())
/// ```
pub fn load_config_file<P: AsRef<Path>>(path: P) -> Result<String> {
    let path = path.as_ref();

    if !path.exists() {
        return Err(ConfigError::FileNotFound(path.display().to_string()));
    }

    fs::read_to_string(path).map_err(|e| ConfigError::ReadError(e.to_string()))
}

/// Busca un archivo de configuración en ubicaciones comunes.
///
/// Busca en el siguiente orden:
/// 1. Variable de entorno `CONFIG_PATH` (si existe)
/// 2. `./config/{filename}`
/// 3. `./{filename}`
///
/// # Ejemplos
///
/// ```no_run
/// use config_loader::find_config_file;
///
/// // Busca config.json en ubicaciones comunes
/// let path = find_config_file("config.json")?;
/// println!("Found config at: {}", path.display());
/// # Ok::<(), config_loader::ConfigError>(())
/// ```
pub fn find_config_file(filename: &str) -> Result<PathBuf> {
    // 1. Check environment variable
    if let Ok(path) = env::var("CONFIG_PATH") {
        let path_buf = PathBuf::from(&path);
        if path_buf.exists() {
            return Ok(path_buf);
        }
    }

    // 2. Check ./config/{filename}
    let config_dir = PathBuf::from("./config").join(filename);
    if config_dir.exists() {
        return Ok(config_dir);
    }

    // 3. Check ./{filename}
    let current_dir = PathBuf::from("./").join(filename);
    if current_dir.exists() {
        return Ok(current_dir);
    }

    Err(ConfigError::FileNotFound(format!(
        "No se encontró '{}'. Buscado en: CONFIG_PATH env var, ./config/{}, ./{}",
        filename, filename, filename
    )))
}

/// Busca y carga un archivo de configuración automáticamente.
///
/// Combina `find_config_file` y `load_config_file` en un solo paso.
///
/// # Ejemplos
///
/// ```no_run
/// use config_loader::find_and_load;
///
/// let content = find_and_load("config.json")?;
/// // Ahora parsea el contenido según tu necesidad
/// # Ok::<(), config_loader::ConfigError>(())
/// ```
pub fn find_and_load(filename: &str) -> Result<String> {
    let path = find_config_file(filename)?;
    load_config_file(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_nonexistent_file() {
        let result = load_config_file("/path/that/does/not/exist.json");
        assert!(result.is_err());
        assert!(matches!(result, Err(ConfigError::FileNotFound(_))));
    }

    #[test]
    fn test_find_nonexistent_file() {
        let result = find_config_file("file_that_definitely_does_not_exist_12345.json");
        assert!(result.is_err());
    }
}
