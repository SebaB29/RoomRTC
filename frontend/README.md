# RoomRTC Frontend

WebRTC video conferencing desktop application built with Rust and EGUI.

## Technology Stack

- **Framework**: EGUI (Immediate mode GUI)
- **Target**: Native desktop (Linux, macOS, Windows)
- **Language**: Rust
- **WebRTC**: Custom implementation from `../webrtc`
- **Backend**: TCP signaling server (`../backend`)

## Prerequisites

- Rust toolchain (latest stable)

## Development

### Run locally
```bash
cargo run
```

The application will start as a native desktop window.

### Build for production
```bash
cargo build --release
```

Binary will be available at `target/release/frontend`

## Features

- **User Authentication**: Register and login via signaling server
- **User Lobby**: View online users and their availability status
- **Peer-to-Peer Video Calls**: Direct WebRTC connections between users
- **Real-time Signaling**: TCP-based binary protocol for SDP/ICE exchange
- **Camera Control**: Toggle camera on/off during calls
- **Responsive UI**: Clean, modern interface with dark theme
- **State Management**: MVC architecture with separation of concerns

## Architecture

The frontend follows an **MVC pattern** with clear separation:

- **Model**: Application state (`app/state.rs`)
- **View**: Pages and components (pure rendering functions)
- **Controller**: Event handlers in `app/handlers/`
- **Logic Thread**: Background WebRTC and media processing

For detailed architecture documentation, see:
- **[ARCHITECTURE.md](ARCHITECTURE.md)** - Architecture patterns and design principles
- **[DEVELOPMENT.md](DEVELOPMENT.md)** - Development guides and how-to instructions
- **[LOGGING.md](LOGGING.md)** - Logging prefix reference

## Project Structure

```
src/
├── app/                    # Application core (MVC Controller)
├── pages/                  # UI pages (MVC View)
├── components/             # Reusable UI components
├── events/                 # Event system (commands & events)
├── logic/                  # Background WebRTC thread
├── infrastructure/         # Server connection
├── models/                 # Data structures
├── context/                # User session state
└── config/                 # Configuration
```

---

**Part of RoomRTC | Rusty Coders | Taller de Programación I - FIUBA - 2025**
