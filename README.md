# 🏠 RoomRTC — Rusty Coders

> A complete WebRTC video conferencing system in Rust for Taller de Programación I (75.42) at FIUBA

## 📋 Overview

**RoomRTC** is a comprehensive WebRTC implementation developed by the Rusty Coders team. This project provides a complete peer-to-peer video conferencing solution with a central signaling server, emphasizing:

- 🦀 **Pure Rust Implementation**: Custom WebRTC stack built from scratch
- 🎥 **Real-Time Video**: H.264 encoding with low-latency optimizations
- 🔐 **Security**: DTLS/SRTP encryption for secure communications
- 🏗️ **Robust Architecture**: Clean, modular, and maintainable design
- 📋 **Comprehensive Testing**: Unit and integration test coverage
- 📚 **Team Collaboration**: Agile development with code reviews

This repository contains all source code, documentation, and configuration files needed to build, test, and run the RoomRTC system.

## 📚 Table of Contents

- [📋 Overview](#overview)
- [👥 Team Members](#team)
- [ℹ️ About the Project](#about)
- [✨ Features](#features)
- [🏗️ Architecture](#architecture)
- [🛠️ Prerequisites](#prerequisites)
- [🚀 Quick Start](#quick-start)
- [⚙️ Configuration](#configuration)
- [🗂️ Project Structure](#project-structure)
- [🧪 Testing](#testing)
- [📖 Documentation](#documentation)
- [🤝 Contributing](#contributing)
- [📄 License](#license)

<a name="team"></a>
## 👥 Team Members

| StudentID | Name |
| :-------: | :------ |
| 103384 | Adriana Macarena Iglesias Tripodi |
| 105288 | Sebastián Brizuela |
| 105400 | Franco Altieri Lamas |
| 105907 | Nicolás Chen |

<a name="about"></a>
## ℹ️ About the Project

RoomRTC is a complete video conferencing system that implements the WebRTC protocol stack in Rust. The project consists of three main components:

1. **WebRTC Library**: A standalone, plug-and-play library implementing ICE, SDP, STUN, RTP/RTCP, and H.264 codec
2. **Signaling Server**: Central server handling user authentication, discovery, and SDP/ICE exchange
3. **Client Application**: GUI application with camera capture and video display

The system enables direct peer-to-peer video calls between users after initial connection establishment through the signaling server.

<a name="features"></a>
## ✨ Features

### Implemented ✅

- **SDP (Session Description Protocol)**: Offer/answer generation and parsing
- **ICE (Interactive Connectivity Establishment)**: Host candidate gathering and connectivity checks
- **STUN Client**: RFC 5389 compliant for NAT traversal
- **RTP/RTCP**: Video transport with H.264 packetization
- **H.264 Codec**: Hardware-accelerated encoding/decoding with FFmpeg
- **Camera Capture**: OpenCV-based HD video capture (1280x720 @ 30 FPS)
- **P2P Session**: Direct peer-to-peer video streaming
- **Frontend GUI**: egui-based interface with video display
- **Logging System**: Structured logging with configurable levels
- **Configuration**: TOML-based .conf files for all parameters

### In Progress 🔨

- **DTLS/SRTP Encryption**: Secure media transport (implemented in network layer)
- **Advanced NAT Traversal**: Enhanced TURN relay support
- **Production Deployment**: Docker containerization and deployment scripts
- **Performance Optimization**: Reduced latency and improved bandwidth usage

<a name="architecture"></a>
## 🏗️ Architecture

```
┌────────────────────────────────────────────────────────────────┐
│                      RoomRTC System                            │
├────────────────────────────────────────────────────────────────┤
│                                                                │
│  ┌─────────────┐         Signaling            ┌─────────────┐  │
│  │  Client A   │◄────────────────────────────►│  Client B   │  │
│  │             │                              │             │  │
│  │  ┌───────┐  │                              │  ┌───────┐  │  │
│  │  │  GUI  │  │       ┌──────────────┐       │  │  GUI  │  │  │
│  │  └───┬───┘  │       │   Signaling  │       │  └───┬───┘  │  │
│  │      │      │◄─────►│    Server    │◄─────►│      │      │  │
│  │  ┌───▼───┐  │   TCP │              │  TCP  │  ┌───▼───┐  │  │
│  │  │WebRTC │  │       │ - Auth       │       │  │WebRTC │  │  │
│  │  │Manager│  │       │ - Users      │       │  │Manager│  │  │
│  │  └───┬───┘  │       │ - SDP Relay  │       │  └───┬───┘  │  │
│  │      │      │       └──────────────┘       │      │      │  │
│  │  ┌───▼───┐  │                              │  ┌───▼───┐  │  │
│  │  │Camera │  │         P2P Media            │  │Camera │  │  │
│  │  └───┬───┘  │◄────────────────────────────►│  └───┬───┘  │  │
│  │      │      │       DTLS/SRTP (RTP)        │      │      │  │
│  │  ┌───▼───┐  │                              │  ┌───▼───┐  │  │
│  │  │H.264  │  │                              │  │H.264  │  │  │
│  │  │Encoder│  │                              │  │Decoder│  │  │
│  │  └───────┘  │                              │  └───────┘  │  │
│  └─────────────┘                              └─────────────┘  │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

### Component Overview

- **WebRTC Library** (`webrtc/`): Core protocol implementations
  - `ice/`: ICE candidate gathering and connectivity checks
  - `sdp/`: SDP parsing and generation
  - `stun/`: STUN client for NAT discovery and reflexive candidates
  - `turn/`: TURN client for relay allocation (RFC 5766)
  - `network/`: RTP/RTCP transport, DTLS/SRTP encryption
  - `media/`: Camera capture and H.264/VP8 codec support
  
- **Signaling Server** (`backend/server/`): Central coordination
  - User authentication (register/login)
  - User directory and presence
  - SDP offer/answer relay
  - ICE candidate exchange
  
- **Client** (`frontend/`): User interface
  - Video display (local and remote)
  - Camera controls
  - Connection management
  - User login/selection

<a name="about"></a>
## ℹ️ About the Project (Detailed)

RoomRTC demonstrates a complete understanding of WebRTC by implementing all major components from scratch

<a name="prerequisites"></a>
## 🛠️ Prerequisites

TBD

<a name="quick-start"></a>
## 🚀 Quick Start

1. **Clone the repository:**

```bash
git clone https://github.com/taller-1-fiuba-rust/25C2-rusty-coders.git
cd 25C2-rusty-coders
```

2. **Build the project:**

```bash
cargo build --release
```

3. **Run the application:**

```bash
cargo run --release
# or, if you built the release binary directly:
./target/release/roome
```

4. **Run tests:**

```bash
cargo test
```

5. **Run tests with coverage:**

```bash
cargo test -- --test-threads=1 --nocapture
```

> **Note:** If this repository is not a Rust project, please substitute the appropriate build and test commands (e.g., `npm install` / `npm test`, `mvn test`, `python -m pytest`, etc.).

<a name="project-structure"></a>
## 🗂️ Project Structure

```
TBD
```

<a name="contributing"></a>
## 🤝 Contributing

We welcome contributions from all team members! Please follow these guidelines:

1. **Create an issue** describing the feature, enhancement, or bug fix
2. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. **Make your changes** following Rust best practices and coding standards
4. **Add tests** to ensure your changes work as expected
5. **Run the test suite** to verify everything passes:
   ```bash
   cargo test
   cargo clippy -- -D warnings  # Check for common mistakes
   cargo fmt -- --check         # Verify formatting
   ```
6. **Commit your changes** with clear, descriptive messages
7. **Push your branch** and open a pull request against `main`
8. **Request review** from at least one team member

### Code Style

- Follow Rust's official [style guidelines](https://doc.rust-lang.org/style-guide/)
- Use `cargo fmt` before committing
- Ensure `cargo clippy` passes without warnings
- Write clear documentation comments (`///`) for public APIs

<a name="license"></a>
## 📄 License

This repository includes a `LICENSE` file. Please refer to it for detailed license information.

---

**Developed with 🦀 by Rusty Coders | Taller de Programación I - FIUBA - 2025**