# Logger Library Structure

This document describes the internal architecture and organization of the logger library.

## Directory Structure

```
logger/
├── Cargo.toml          # Package configuration
├── README.md           # User-facing documentation
├── STRUCTURE.md        # This file - Architecture documentation
└── src/
    ├── lib.rs          # Public API exports
    ├── error.rs        # Error types and handling
    ├── log_level.rs    # Log level definitions
    ├── log_message.rs  # Internal message structure
    ├── log_writer.rs   # Asynchronous file writer
    └── logger.rs       # Main logger implementation
```

## Data Flow

```
1. User calls logger.info("message")
   ↓
2. Logger checks if Info >= minimum level
   ↓
3. Create LogMessage with timestamp
   ↓
4. Send through channel (non-blocking)
   ↓
5. Writer thread receives message
   ↓
6. Format and write to file
   ↓
7. Flush to ensure persistence
```

## Thread Model

```
Main Thread                  Writer Thread
-----------                  -------------
Logger::new()
    ├─ channel()
    └─ spawn() ────────────> run()
                                 ├─ open file
                                 └─ loop:
                                     ├─ receive msg
                                     ├─ write to file
                                     └─ flush

logger.info()
    ├─ check level
    ├─ create message
    └─ send() ──────────────> format()
                              write_all()
                              flush()

logger.clone()
(shares channel)
```
