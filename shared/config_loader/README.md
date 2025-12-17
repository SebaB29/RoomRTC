# Config Loader

Una librería **genérica** para cargar archivos de configuración desde disco. No asume ninguna estructura específica - el consumidor decide cómo parsear y validar.

## 🎯 Filosofía

- **Genérica**: No impone structs ni formatos específicos
- **Zero dependencies**: Solo Rust std library + json_parser propio
- **Flexible**: El consumidor decide cómo parsear (JSON, TOML-like, custom)
- **Simple**: Lee archivo, devuelve String - nada más
- **Reutilizable**: Sirve para cualquier tipo de configuración

## 📦 Instalación

Agrega a tu `Cargo.toml`:

```toml
[dependencies]
config_loader = { path = "../shared/config_loader" }
json_parser = { path = "../shared/json_parser" }  # o el parser que prefieras
```

## 🚀 Uso Rápido

### 1. Carga el archivo

```rust
use config_loader::{find_and_load, load_config_file};

// Opción 1: Búsqueda automática (busca en ./config/, ./, CONFIG_PATH)
let content = find_and_load("config.json")?;

// Opción 2: Path específico
let content = load_config_file("./my_config.json")?;
```

### 2. Parsea según tu necesidad

**Ejemplo con JSON:**

```rust
use config_loader::find_and_load;
use json_parser::{parse, Value};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Carga el archivo
    let content = find_and_load("config.json")?;
    
    // 2. Parsea con json_parser
    let json_value = parse(&content)?;
    
    // 3. Extrae valores según tu necesidad
    if let Value::Object(obj) = json_value {
        if let Some(Value::String(host)) = obj.get("server_host") {
            println!("Host: {}", host);
        }
        if let Some(Value::Number(port)) = obj.get("server_port") {
            println!("Port: {}", port);
        }
    }
    
    Ok(())
}
```

**Ejemplo con struct personalizado:**

```rust
use config_loader::find_and_load;
use json_parser::{parse, Value};

// Define TU propia estructura de config
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub debug: bool,
}

impl ServerConfig {
    // TU decides cómo parsear
    pub fn from_json(value: Value) -> Result<Self, Box<dyn std::error::Error>> {
        let obj = match value {
            Value::Object(o) => o,
            _ => return Err("Expected object".into()),
        };
        
        let host = match obj.get("host") {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("Missing 'host' field".into()),
        };
        
        let port = match obj.get("port") {
            Some(Value::Number(n)) => *n as u16,
            _ => return Err("Missing 'port' field".into()),
        };
        
        let debug = match obj.get("debug") {
            Some(Value::Boolean(b)) => *b,
            _ => false, // default
        };
        
        Ok(ServerConfig { host, port, debug })
    }
    
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let content = find_and_load("config.json")?;
        let json_value = parse(&content)?;
        Self::from_json(json_value)
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::load()?;
    println!("Server: {}:{}", config.host, config.port);
    Ok(())
}
```

## 📖 API Completa

### Funciones de Carga

#### `load_config_file(path)`
Carga el contenido de un archivo específico.

```rust
use config_loader::load_config_file;

let content = load_config_file("./config/app.json")?;
let content = load_config_file("/etc/myapp/config.json")?;
```

**Retorna**: `Result<String, ConfigError>`
- `Ok(String)`: Contenido del archivo
- `Err(ConfigError::FileNotFound)`: Archivo no existe
- `Err(ConfigError::ReadError)`: Error al leer

#### `find_config_file(filename)`
Busca un archivo en ubicaciones comunes.

Busca en orden:
1. Variable de entorno `CONFIG_PATH`
2. `./config/{filename}`
3. `./{filename}`

```rust
use config_loader::find_config_file;

let path = find_config_file("config.json")?;
println!("Found at: {}", path.display());
```

**Retorna**: `Result<PathBuf, ConfigError>`

#### `find_and_load(filename)`
Combina búsqueda + carga en un solo paso.

```rust
use config_loader::find_and_load;

let content = find_and_load("config.json")?;
// Ahora parsea como quieras
```

**Retorna**: `Result<String, ConfigError>`

## 🔧 Características Avanzadas

### Variables de Entorno

Especifica la ruta exacta via environment variable:

```bash
export CONFIG_PATH=/path/to/my/config.json
cargo run
```

```rust
// Automáticamente usará CONFIG_PATH si existe
let content = find_and_load("config.json")?;
```

### Múltiples Formatos

La librería no asume ningún formato - tú decides:

**JSON con json_parser:**
```rust
let content = find_and_load("config.json")?;
let value = json_parser::parse(&content)?;
```

**TOML-like con config_manager:**
```rust
let content = load_config_file("config.conf")?;
let mut manager = ConfigManager::new();
manager.parse_string(&content)?;
```

**Formato custom:**
```rust
let content = find_and_load("config.txt")?;
let config = MyCustomParser::parse(&content)?;
```

## 💡 Patrones de Uso

### Patrón 1: Struct con método `load()`

```rust
pub struct AppConfig {
    pub server_host: String,
    pub server_port: u16,
    // ... más campos
}

impl AppConfig {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let content = config_loader::find_and_load("config.json")?;
        let json = json_parser::parse(&content)?;
        Self::from_json(json)
    }
    
    fn from_json(value: Value) -> Result<Self, Box<dyn std::error::Error>> {
        // Tu lógica de parseo
    }
}

// Uso:
let config = AppConfig::load()?;
```

### Patrón 2: Factory function

```rust
pub fn load_app_config() -> Result<AppConfig, Box<dyn std::error::Error>> {
    let content = config_loader::find_and_load("config.json")?;
    let json = json_parser::parse(&content)?;
    parse_app_config(json)
}

fn parse_app_config(value: Value) -> Result<AppConfig, Box<dyn std::error::Error>> {
    // Tu lógica de parseo
}
```

### Patrón 3: Validación separada

```rust
pub struct Config {
    // campos...
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let content = config_loader::find_and_load("config.json")?;
        let config = Self::parse(&content)?;
        config.validate()?;
        Ok(config)
    }
    
    fn parse(content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // parseo
    }
    
    fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        if self.port == 0 {
            return Err("Port must be > 0".into());
        }
        Ok(())
    }
}
```

## 🧪 Testing

La librería incluye tests básicos:

```bash
cd shared/config_loader
cargo test
```

Para testear tu configuración:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use config_loader::load_config_file;

    #[test]
    fn test_load_test_config() {
        let content = load_config_file("./tests/test_config.json").unwrap();
        let config = MyConfig::parse(&content).unwrap();
        assert_eq!(config.port, 8080);
    }
}
```

## 📚 Ejemplo Completo: Backend Server

```rust
// backend/server/src/config.rs
use config_loader::find_and_load;
use json_parser::{parse, Value};
use std::error::Error;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub timeout_seconds: u64,
    pub max_connections: usize,
    pub debug: bool,
}

impl ServerConfig {
    pub fn load() -> Result<Self, Box<dyn Error>> {
        // 1. Carga el archivo (busca automáticamente)
        let content = find_and_load("server_config.json")?;
        
        // 2. Parsea JSON
        let json = parse(&content)?;
        
        // 3. Extrae valores
        Self::from_json(json)
    }
    
    fn from_json(value: Value) -> Result<Self, Box<dyn Error>> {
        let obj = match value {
            Value::Object(o) => o,
            _ => return Err("Expected JSON object".into()),
        };
        
        let host = Self::get_string(&obj, "host")?;
        let port = Self::get_number(&obj, "port")? as u16;
        let timeout_seconds = Self::get_number(&obj, "timeout_seconds")
            .unwrap_or(30.0) as u64;
        let max_connections = Self::get_number(&obj, "max_connections")
            .unwrap_or(100.0) as usize;
        let debug = Self::get_bool(&obj, "debug").unwrap_or(false);
        
        Ok(ServerConfig {
            host,
            port,
            timeout_seconds,
            max_connections,
            debug,
        })
    }
    
    fn get_string(obj: &std::collections::HashMap<String, Value>, key: &str) 
        -> Result<String, Box<dyn Error>> 
    {
        match obj.get(key) {
            Some(Value::String(s)) => Ok(s.clone()),
            _ => Err(format!("Missing or invalid field: {}", key).into()),
        }
    }
    
    fn get_number(obj: &std::collections::HashMap<String, Value>, key: &str) 
        -> Result<f64, Box<dyn Error>> 
    {
        match obj.get(key) {
            Some(Value::Number(n)) => Ok(*n),
            _ => Err(format!("Missing or invalid field: {}", key).into()),
        }
    }
    
    fn get_bool(obj: &std::collections::HashMap<String, Value>, key: &str) 
        -> Result<bool, Box<dyn Error>> 
    {
        match obj.get(key) {
            Some(Value::Boolean(b)) => Ok(*b),
            _ => Err(format!("Missing or invalid field: {}", key).into()),
        }
    }
    
    pub fn validate(&self) -> Result<(), Box<dyn Error>> {
        if self.host.is_empty() {
            return Err("Host cannot be empty".into());
        }
        if self.port == 0 {
            return Err("Port must be > 0".into());
        }
        Ok(())
    }
}

// backend/server/src/main.rs
use config::ServerConfig;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig::load()?;
    config.validate()?;
    
    println!("🚀 Starting server on {}:{}", config.host, config.port);
    
    // ... tu lógica de servidor
    
    Ok(())
}
```

**config/server_config.json:**
```json
{
  "host": "0.0.0.0",
  "port": 8080,
  "timeout_seconds": 30,
  "max_connections": 100,
  "debug": false
}
```

## 🆚 Ventajas de este Enfoque

| Característica | config_loader (genérico) | Alternativas con serde |
|----------------|--------------------------|------------------------|
| **Dependencies** | ✅ Solo std + propias | ❌ serde + formato-específico |
| **Flexibilidad** | ✅ Total control | ⚠️ Limitado a serde |
| **Tamaño binario** | ✅ Mínimo | ❌ +200KB |
| **Compile time** | ✅ Rápido | ⚠️ Macros lentas |
| **Learning curve** | ✅ Simple | ⚠️ Requiere aprender serde |
| **Control** | ✅ 100% tuyo | ⚠️ Delegado a serde |

## 📄 Licencia

Este código es parte del proyecto RoomRTC y sigue la misma licencia del workspace.

## 🎯 Filosofía

- **Type-safe**: Todo se valida en compile-time
- **Simple**: Un solo `load_config()` call
- **Mínimas dependencias**: Solo serde + toml
- **Reutilizable**: Define tu struct una vez, úsalo en todo el proyecto
- **Auto-descubrimiento**: Busca automáticamente el archivo de configuración

## 📦 Instalación

Agrega a tu `Cargo.toml`:

```toml
[dependencies]
config_loader = { path = "../shared/config_loader" }
```

## 🚀 Uso Rápido

### 1. Crea tu archivo de configuración

`config/config.toml`:
```toml
[server]
host = "0.0.0.0"
port = 8080
timeout_seconds = 30
max_connections = 100

[app]
name = "RoomRTC"
debug = false
log_level = "info"

[storage]
data_dir = "./data"
backup_enabled = true
```

### 2. Carga en tu aplicación

```rust
use config_loader::{load_config, Config};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Carga automática (busca en ./config/config.toml, ./config.toml, etc.)
    let config = load_config()?;

    println!("🚀 Starting {} on {}", 
        config.app.name, 
        config.server_address()
    );

    // Usa la configuración...
    start_server(&config)?;

    Ok(())
}

fn start_server(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    println!("Listening on {}:{}", config.server.host, config.server.port);
    
    if config.app.debug {
        println!("Debug mode enabled!");
    }
    
    Ok(())
}
```

## 📖 API Completa

### Funciones de Carga

#### `load_config()` / `load_config_auto()`
Busca y carga automáticamente la configuración en el siguiente orden:
1. Variable de entorno `CONFIG_PATH`
2. `./config/config.toml`
3. `./config.toml`

```rust
let config = load_config()?;
```

#### `load_config_from(path)`
Carga desde una ruta específica:

```rust
let config = load_config_from("./custom_config.toml")?;
let config = load_config_from("/etc/myapp/config.toml")?;
```

### Estructura de Config

```rust
pub struct Config {
    pub server: ServerConfig,
    pub app: AppConfig,
    pub storage: StorageConfig,
}

pub struct ServerConfig {
    pub host: String,                    // ej: "0.0.0.0"
    pub port: u16,                       // ej: 8080
    pub timeout_seconds: u64,            // default: 30
    pub max_connections: usize,          // default: 100
}

pub struct AppConfig {
    pub name: String,                    // ej: "RoomRTC"
    pub debug: bool,                     // default: false
    pub log_level: String,               // "debug"|"info"|"warn"|"error"
}

pub struct StorageConfig {
    pub data_dir: String,                // ej: "./data"
    pub backup_enabled: bool,            // default: false
}
```

### Métodos Útiles

```rust
// Obtener dirección completa del servidor
let addr = config.server_address(); // "0.0.0.0:8080"

// Validación manual (se llama automáticamente al cargar)
config.validate()?;
```

## 🔧 Características Avanzadas

### Valores por Defecto

Algunos campos tienen valores por defecto si no se especifican:

```toml
[server]
host = "0.0.0.0"
port = 8080
# timeout_seconds = 30  (default si se omite)
# max_connections = 100  (default si se omite)

[app]
name = "MyApp"
# debug = false  (default si se omite)
# log_level = "info"  (default si se omite)

[storage]
data_dir = "./data"
# backup_enabled = false  (default si se omite)
```

### Validación Automática

La configuración se valida automáticamente al cargar:

- `server.host` no puede estar vacío
- `server.port` debe ser > 0
- `app.name` no puede estar vacío
- `app.log_level` debe ser: "debug", "info", "warn", o "error"
- `storage.data_dir` no puede estar vacío

Si la validación falla, recibes un error descriptivo:

```rust
match load_config() {
    Ok(config) => println!("✓ Config cargada correctamente"),
    Err(e) => eprintln!("✗ Error: {}", e),
}
```

### Variables de Entorno

Puedes especificar la ruta del config via environment variable:

```bash
export CONFIG_PATH=/path/to/custom/config.toml
cargo run
```

```rust
// Automáticamente usará CONFIG_PATH si existe
let config = load_config()?;
```

## 🆚 Comparación: config_loader vs config_manager

| Característica | config_loader | config_manager |
|----------------|---------------|----------------|
| **Type Safety** | ✅ Compile-time | ❌ Runtime |
| **Autocompletado IDE** | ✅ Perfecto | ❌ No disponible |
| **Validación** | ✅ Automática | ⚠️ Manual |
| **Dependencies** | serde + toml | Zero (solo std) |
| **Flexibilidad** | ⚠️ Requiere recompilación | ✅ Dinámico |
| **Documentación** | ✅ El struct ES la docs | ⚠️ Dispersa en código |
| **Errores** | ✅ En compile-time | ⚠️ En runtime |
| **Refactoring** | ✅ Seguro | ⚠️ Propenso a errores |
| **Complejidad** | Simple | Media |

### ¿Cuándo usar cada uno?

**Usa `config_loader` cuando:**
- Estás en producción y quieres máxima seguridad
- Quieres autocompletado y type safety
- Tu configuración es relativamente estable
- Prefieres detectar errores en compile-time

**Usa `config_manager` cuando:**
- Necesitas configuración muy dinámica
- No quieres dependencias externas
- Estás prototipando rápido
- Necesitas merge de múltiples configs en runtime

## 💡 Ejemplo Completo: Backend Server

```rust
use config_loader::{load_config, Config};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    // Cargar configuración
    let config = load_config()?;

    // Inicializar logger con nivel de config
    init_logger(&config.app.log_level)?;

    // Crear directorio de datos si no existe
    std::fs::create_dir_all(&config.storage.data_dir)?;

    // Iniciar servidor
    println!("🚀 Starting {} v1.0", config.app.name);
    println!("📍 Listening on {}", config.server_address());
    println!("📁 Data directory: {}", config.storage.data_dir);
    
    if config.app.debug {
        println!("⚠️  DEBUG MODE ENABLED");
    }

    start_http_server(&config)?;

    Ok(())
}

fn start_http_server(config: &Config) -> Result<(), Box<dyn Error>> {
    // Tu lógica de servidor aquí...
    // Usa config.server.host, config.server.port, etc.
    Ok(())
}

fn init_logger(level: &str) -> Result<(), Box<dyn Error>> {
    // Tu lógica de logging aquí...
    Ok(())
}
```

## 🎨 Extendiendo la Configuración

Para agregar nuevos campos, simplemente edita `src/config.rs`:

```rust
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub timeout_seconds: u64,
    pub max_connections: usize,
    
    // ¡Nuevo campo!
    #[serde(default = "default_enable_cors")]
    pub enable_cors: bool,
}

fn default_enable_cors() -> bool {
    true
}
```

Y actualiza tu `config.toml`:

```toml
[server]
host = "0.0.0.0"
port = 8080
enable_cors = true  # ¡Nuevo!
```

## 🧪 Testing

La librería incluye tests para validación:

```bash
cd shared/config_loader
cargo test
```

Para testear tu aplicación con diferentes configs:

```rust
#[cfg(test)]
mod tests {
    use config_loader::*;

    #[test]
    fn test_load_test_config() {
        let config = load_config_from("./tests/test_config.toml").unwrap();
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.app.name, "Test App");
    }
}
```

## 📚 Recursos

- [Serde Documentation](https://serde.rs/)
- [TOML Specification](https://toml.io/)
- Ejemplo completo: Ver `backend/server/src/main.rs` (próximamente)

## 🤝 Comparación con Go

Si vienes de Go (como pareces indicar), esto es equivalente a:

**Go (Viper o similar):**
```go
type Config struct {
    Server ServerConfig `mapstructure:"server"`
    App    AppConfig    `mapstructure:"app"`
}

func LoadConfig() (*Config, error) {
    viper.SetConfigName("config")
    viper.AddConfigPath("./config")
    
    if err := viper.ReadInConfig(); err != nil {
        return nil, err
    }
    
    var config Config
    if err := viper.Unmarshal(&config); err != nil {
        return nil, err
    }
    
    return &config, nil
}
```

**Rust (config_loader):**
```rust
// ¡Es más simple!
let config = load_config()?;
```

La diferencia clave: **En Rust tienes compile-time type safety**, mientras que en Go los errores aparecen en runtime.

## 📄 Licencia

Este código es parte del proyecto RoomRTC y sigue la misma licencia del workspace.
