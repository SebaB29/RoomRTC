# 🏗️ Monorepo Structure - Roome

Project structure with clear separation of responsibilities.

---

## 📁 Current Structure

```
25C2-rusty-coders/
├── 🌐 webrtc/                      # Independent WebRTC library
│   ├── Cargo.toml
│   ├── README.md
│   ├── src/                        # Main integration layer
│   │   ├── lib.rs                  # Public API
│   │   ├── camera_manager.rs       # Camera lifecycle management
│   │   ├── camera_info.rs          # Camera types and info
│   │   ├── connection/             # WebRTC connection implementations
│   │   └── session/                # P2P session and media pipeline
│   ├── ice/                        # ICE connectivity (RFC 5245/8445)
│   ├── sdp/                        # Session Description (RFC 4566)
│   ├── media/                      # Camera capture & H.264/VP8 codecs
│   ├── network/                    # RTP/RTCP, DTLS/SRTP encryption
│   ├── stun/                       # STUN client (RFC 5389)
│   └── turn/                       # TURN client (RFC 5766)
│
├── 🔧 shared/                      # Shared utilities
│   ├── logger/                     # Thread-safe async logger
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   ├── STRUCTURE.md
│   │   └── src/
│   │       ├── lib.rs              # Public API
│   │       ├── logger.rs           # Logger implementation
│   │       ├── log_message.rs      # Message types
│   │       ├── log_level.rs        # Log levels
│   │       ├── log_writer.rs       # File writer
│   │       └── error.rs            # Error types
│   ├── json_parser/                # RFC 8259 JSON parser
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/                    # Parser implementation
│   └── config_loader/              # Configuration file loader
│       ├── Cargo.toml
│       ├── README.md
│       └── src/                    # Config loader
│
├── 🚀 backend/                     # Signaling server
│   ├── Cargo.toml
│   ├── README.md
│   ├── CONFIG.md                   # Configuration reference
│   ├── PROTOCOL.md                 # Binary TCP protocol spec
│   ├── server_config.json          # Server configuration
│   └── src/
│       ├── main.rs                 # Server entry point
│       ├── lib.rs                  # Library exports
│       ├── domain/                 # Domain entities (User, Call, States)
│       ├── application/            # Use cases and handlers
│       │   ├── handlers/           # Message handlers
│       │   └── usecases/           # Business logic
│       ├── infrastructure/         # Storage and persistence
│       ├── tcp/                    # TCP server and protocol
│       │   ├── messages/           # Message definitions
│       │   └── tls/                # TLS support (PKCS#12)
│       └── config/                 # Configuration modules
│
├── 🖥️ frontend/                    # UI application (EGUI)
│   ├── Cargo.toml
│   ├── index.html
│   ├── Trunk.toml
│   ├── README.md
│   ├── ARCHITECTURE.md
│   ├── DEVELOPMENT.md
│   └── src/
│       ├── main.rs                 # App entry point
│       ├── storage.rs              # Local storage
│       ├── app/                    # Application core
│       ├── components/             # Reusable UI components
│       ├── context/                # App state management
│       ├── events/                 # Event handling
│       ├── logic/                  # Business logic
│       ├── models/                 # Data models
│       └── pages/                  # UI pages
│
├── Cargo.toml                      # Workspace root
├── README.md                       # Project overview
```

---

## 🎯 Separation of Responsibilities

### 0️⃣ Logging - Shared Utility

**Purpose**: Application-wide logging system, independent and reusable.

**Features**:
- Thread-safe concurrent logging
- File-based persistent logs with rotation
- Configurable log levels (Debug, Info, Warning, Error)
- Used by all components (WebRTC, backend, frontend)

---

### 1️⃣ WebRTC - Independent Library

**Purpose**: Complete WebRTC implementation, reusable in any Rust project.

**Modules**:
- **ice/** - ICE connectivity (RFC 8445/5245)
- **sdp/** - Session Description Protocol (RFC 4566/8866)
- **stun/** - STUN client for NAT discovery (RFC 5389)
- **turn/** - TURN client for relay allocation (RFC 5766)
- **media/** - Camera capture, H.264/VP8 codecs
- **network/** - RTP transport (RFC 3550/6184)
- **stun/** - STUN client (RFC 5389/8489)
- **signaling/** - SDP exchange via WebSocket

**Features**:
- Modular plug-and-play architecture
- Each component works independently
- Production-ready with comprehensive testing

---

### 2️⃣ Backend - Server Application

**Purpose**: HTTP/WebSocket server using the WebRTC library.

**Features**:
- Room management (create, list, delete)
- Signaling server for SDP exchange
- REST API for control
- WebSocket for real-time communication

---

### 3️⃣ Frontend - UI Application

**Purpose**: Cross-platform graphical interface with EGUI.

**Features**:
- Native EGUI interface
- Reusable UI components
- Backend integration via HTTP/WebSocket
- Uses production-ready WebRTC library

---

## 🚀 Quick Start

```bash
# Build all
cargo build --workspace

# Run tests (WebRTC: 359+)
cargo test --workspace

# Quality checks
cargo clippy --workspace
cargo doc --workspace --no-deps --open

# Run applications
cargo run --package server    # Backend
cd frontend && trunk serve    # Frontend
```

---
