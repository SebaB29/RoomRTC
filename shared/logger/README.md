# Logger - Thread-Safe Asynchronous Logger

Thread-safe logging with non-blocking writes and zero global state.

## Overview

Production-ready logger for multi-threaded applications. Logger calls are non-blocking (< 1μs) with writes handled asynchronously in a dedicated thread.

## Features

- ✅ Thread-safe without global state
- ✅ Non-blocking log calls via channels
- ✅ Cloneable for sharing across threads
- ✅ Multiple log levels (Debug, Info, Warn, Error)
- ✅ Millisecond-precision timestamps
- ✅ Automatic file flushing

## Quick Start

```rust
use logger::{Logger, LogLevel};

// Create logger
let logger = Logger::new("app.log".into(), LogLevel::Info)?;

// Log messages
logger.info("Application started");
logger.warn("Low memory warning");
logger.error("Connection failed");
logger.debug("Not logged (level is Info)");

// Share across threads
let logger_clone = logger.clone();
std::thread::spawn(move || {
    logger_clone.info("Hello from thread!");
});
```

## Architecture

```
Thread 1 ─┐
Thread 2 ─┼─> Channel ──> Writer Thread ──> log.txt
Thread 3 ─┘              (non-blocking)
```

Log calls send messages through a channel to a dedicated writer thread, ensuring zero blocking.

## API

```rust
// Create logger
Logger::new(path: PathBuf, level: LogLevel) -> Result<Logger>

// Log methods
logger.debug(message: &str)  // Only if level >= Debug
logger.info(message: &str)   // Only if level >= Info
logger.warn(message: &str)   // Only if level >= Warn
logger.error(message: &str)  // Always logged

// Log levels (Debug < Info < Warn < Error)
pub enum LogLevel { Debug, Info, Warn, Error }
```

## Log Format
```
[2024-11-01 14:32:10.123] INFO: Application started
[2024-11-01 14:32:10.456] DEBUG: Loading configuration
[2024-11-01 14:32:11.789] WARN: High memory usage: 85%
[2024-11-01 14:32:15.012] ERROR: Database connection failed
```

## Best Practices

**✅ DO:**
- Use appropriate log levels for different situations
- Clone logger instances for multi-threaded use
- Throttle logs in tight loops to avoid disk saturation

**❌ DON'T:**
- Log sensitive data (passwords, tokens, etc.)
- Log in tight loops without throttling

## Example: Multi-threaded Application

```rust
use logger::{Logger, LogLevel};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new("app.log".into(), LogLevel::Info)?;
    logger.info("Application started");
    
    // Clone for threads
    let logger_net = logger.clone();
    let network = std::thread::spawn(move || {
        logger_net.info("Network thread running");
    });
    
    network.join().unwrap();
    logger.info("Application stopped");
    Ok(())
}
```

## Integration Example

```rust
use logger::{Logger, LogLevel};
use std::thread;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new("app.log".into(), LogLevel::Debug)?;
    logger.info("Application started");
    
    // Network thread
    let logger_net = logger.clone();
    let network = thread::spawn(move || {
        logger_net.info("Network thread started");
        // Network operations...
    });
    
    // Media thread
    let logger_media = logger.clone();
    let media = thread::spawn(move || {
        logger_media.info("Media thread started");
        // Media processing...
    });
    
    network.join().unwrap();
    media.join().unwrap();
    
    logger.info("Application stopped");
    Ok(())
}
```

## Dependencies

- **chrono**: ISO 8601 timestamp generation
- **std::sync::mpsc**: Channel-based async communication

## References

- [Rust std::sync::mpsc](https://doc.rust-lang.org/std/sync/mpsc/)
- [Chrono Documentation](https://docs.rs/chrono/)
- [Logging Best Practices](https://www.scalyr.com/blog/logging-best-practices/)
