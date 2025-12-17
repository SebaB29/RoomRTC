# 🏠 RoomRTC Signaling Server

> A WebRTC signaling server with custom binary TCP protocol, implementing Clean Architecture principles

<a name="overview"></a>
## 📋 Overview

**RoomRTC Signaling Server** is the central coordination component of the RoomRTC video conferencing system. This server handles user authentication, presence management, and real-time WebRTC signaling over persistent TCP connections.

### Key Characteristics

- 🦀 **Pure Rust Implementation**: Custom binary protocol using standard library
- 🔐 **Security First**: Optional TLS encryption (PKCS#12) and bcrypt password hashing
- 🏗️ **Clean Architecture**: Layered design - Domain → Application → Infrastructure → TCP  
- 📡 **Efficient Protocol**: Binary format with 4-byte length + 1-byte type + JSON payload
- 🔄 **Real-time Updates**: Live user state broadcasts to all connected clients
- 🧵 **Thread-per-Connection**: Simple, reliable concurrency model
- 🧪 **Testing Suite**: Automated test scripts for protocol validation

This server enables peer-to-peer WebRTC connections by facilitating SDP/ICE exchange between clients while maintaining user state and call management.

## 📚 Table of Contents

- [📋 Overview](#overview)
- [✨ Features](#features)
- [🏗️ Architecture](#architecture)
- [🚀 Quick Start](#quick-start)
- [🧪 Testing](#testing)
- [📖 Documentation](#documentation)

<a name="features"></a>
## ✨ Features

#### User Management 👥
- User registration with bcrypt password hashing
- Login/logout with persistent TCP connections
- Three-state model (Disconnected, Available, Busy)
- Persistent storage in `users.txt`

#### Call Management 📞
- Peer-to-peer call initiation and acceptance/decline
- Automatic state transitions (Available ↔ Busy)
- Automatic cleanup on disconnect

#### WebRTC Signaling 🔄
- SDP offer/answer forwarding
- ICE candidate exchange
- Real-time signaling over persistent TCP

#### Security 🔐
- Optional TLS encryption (PKCS#12)
- bcrypt password hashing
- Compatible with TLS-terminating proxies

<a name="architecture"></a>
## 🏗️ Architecture

### System Overview

```
┌──────────────────────────────────────────────────────────────┐
│                  RoomRTC Signaling Server                    │
├──────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────┐  TCP/TLS ┌────────────────┐      ┌──────────┐  │
│  │ Client A │◄────────►│   TCP Server   │◄─────│ Client B │  │
│  └──────────┘          │ (optional TLS) │      └──────────┘  │
│                        └────────┬───────┘                    │
│                                 │                            │
│                        ┌────────▼────────┐                   │
│                        │ Client Handler  │                   │
│                        │(per connection) │                   │
│                        └────────┬────────┘                   │
│                                 │                            │
│                        ┌────────▼────────┐                   │
│                        │ Message Handler │                   │
│                        │  (routes msgs)  │                   │
│                        └────────┬────────┘                   │
│                                 │                            │
│          ┌──────────────────────┼───────────────────┐        │
│          │                      │                   │        │
│     ┌────▼─────┐        ┌───────▼──────┐      ┌─────▼────┐   │
│     │   Auth   │        │     Call     │      │   User   │   │
│     │  UseCase │        │    UseCase   │      │  UseCase │   │
│     └────┬─────┘        └───────┬──────┘      └─────┬────┘   │
│          │                      │                   │        │
│          └──────────────────────┼───────────────────┘        │
│                                 │                            │
│                        ┌────────▼────────┐                   │
│                        │     Storage     │                   │
│                        │  (Arc<Mutex>)   │                   │
│                        └────────┬────────┘                   │
│                                 │                            │
│                        ┌────────▼────────┐                   │
│                        │  Persistence    │                   │
│                        │  (users.txt)    │                   │
│                        └─────────────────┘                   │
└──────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Protocol**: Custom binary TCP (length + type + JSON)
- **Transport**: `std::net::TcpListener` + optional TLS (PKCS#12)
- **Storage**: Thread-safe `Arc<Mutex<HashMap>>`
- **Persistence**: Plain text file (`users.txt`)
- **Concurrency**: `std::thread` (one per connection)
- **Logging**: Custom component-based logger

### Design Principles

- Clean Architecture (Domain → Application → Infrastructure → TCP)
- Single Responsibility (one struct per file)
- Dependency Injection
- Thread-safe shared state

### Module Structure (Clean Architecture)

```
backend/src/
├── main.rs              # Entry point & initialization
├── domain/              # Entities (User, Call, States)
├── application/         # Use Cases (Auth, Call, Signaling, User)
├── infrastructure/      # Storage & Persistence
├── tcp/                 # Protocol, Server, Handlers, TLS
└── config/              # Configuration management
```

**Layer Dependencies**: TCP → Infrastructure → Application → Domain

<a name="quick-start"></a>
## 🚀 Quick Start

### 1. Build

```bash
cd backend
cargo build --release
```

### 2. Configure

Create `server_config.json`:

```json
{
  "server": {
    "bind_address": "127.0.0.1",
    "port": 8080,
    "enable_tls": false
  }
}
```

See [CONFIG.md](CONFIG.md) for all options.

### 3. Run

```bash
cargo run --release
```

### 4. Test

```bash
./test_server.sh  # or test_server.ps1 on Windows
```

<a name="testing"></a>
## 🧪 Testing

Run automated test scripts with running server in the background:

```bash
# Unix/Linux/macOS
./test_server.sh

# Windows PowerShell
.\test_server.ps1
```

Or run integration tests directly:
```bash
cargo test --test integration_test -- --nocapture --test-threads=1
```

<a name="documentation"></a>
## 📖 Documentation

| Document | Description |
|----------|-------------|
| **[README.md](README.md)** | Overview and quick start (this file) |
| **[CONFIG.md](CONFIG.md)** | Complete configuration reference |
| **[PROTOCOL.md](PROTOCOL.md)** | Binary protocol specification with examples |

---

**Developed with 🦀 by Rusty Coders | Backend Server | Taller de Programación I - FIUBA - 2025**
