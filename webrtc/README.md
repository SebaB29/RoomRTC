# 🌐 WebRTC - Modular Implementation in Rust

Simplified WebRTC implementation in pure Rust, designed with a modular plug-and-play architecture.

## 📚 Table of Contents

- [📋 Overview](#overview)
- [🚀 Quick Start](#quick-start)
  - [📦 Installation](#installation)
  - [🛠️ Prerequisites](#prerequisites)
  - [💡 Basic Usage](#basic-usage)
- [🧩 Modules](#modules)
- [✨ Features](#features)
- [🏗️ Architecture](#architecture)
  - [🔧 Core Components](#core-components)
  - [🔄 P2P Session Pipeline](#p2p-session-pipeline)
- [📖 Usage Examples](#usage-examples)
  - [📹 Basic P2P Video Call](#basic-p2p-video-call)
  - [🎥 Camera Discovery and Selection](#camera-discovery-and-selection)
  - [🎤 Audio Device Discovery and Capture](#audio-device-discovery-and-capture)
  - [🎬 Combined Audio + Video Capture](#combined-audio--video-capture)
  - [⚙️ Manual Connection Setup](#manual-connection-setup-advanced)
  - [💬 Control Messages and State Sync](#control-messages-and-state-sync)
  - [🔌 Using Individual Modules](#using-individual-modules)
- [📚 Public API](#public-api)
- [📦 Dependencies](#dependencies)
- [📖 References](#references)

## 📋 Overview

This library implements WebRTC components following RFC standards, with each module independently usable. The architecture emphasizes simplicity, type safety, and clear separation of concerns.

**Key Highlights:**
- 🎯 **Simple API**: `WebRtcConnection` provides everything needed in one interface
- 🧩 **Modular**: Use individual components (ICE, SDP, RTP, etc.) independently
- 🦀 **Pure Rust**: Safe Rust with comprehensive error handling
- ⚡ **Low Latency**: Multi-threaded pipeline with minimal buffering
- 📹 **Camera Support**: Built-in camera discovery and capture
- � **Audio Support**: Audio device detection and capture
- �🎬 **H.264/VP8**: Hardware-accelerated encoding via FFmpeg

## 🚀 Quick Start

### 📦 Installation

Add to your `Cargo.toml`:
```toml
[dependencies]
webrtc = { path = "path/to/webrtc" }
logging = { path = "path/to/logging" }
```

### 🛠️ Prerequisites

**System Dependencies:**
- **OpenCV** (4.x) - For camera capture
- **FFmpeg** (4.x or 5.x) - For H.264/VP8 codec support

**macOS:**
```bash
brew install opencv ffmpeg
```

**Ubuntu/Debian:**
```bash
sudo apt install libopencv-dev libavcodec-dev libavformat-dev libavutil-dev libswscale-dev
```

### 💡 Basic Usage

```rust
use webrtc::WebRtcConnection;
use logging::Logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new();
    
    // Offerer: Create connection and offer
    let (mut conn, offer) = WebRtcConnection::create_offer_from_new(logger)?;
    
    // Exchange SDP with peer via signaling...
    // Then set remote answer:
    conn.set_remote_answer(&answer)?;
    
    // Establish connection
    conn.establish_connection()?;
    
    // Start camera and stream
    conn.start_camera_auto(30.0)?;
    
    loop {
        // Send local video and get RGB preview
        let (w, h, rgb) = conn.capture_and_send()?;
        
        // Receive remote video
        if let Some((w, h, rgb)) = conn.receive_frame()? {
            // Render remote video...
        }
    }
}
```

See [Usage Examples](#usage-examples) for more detailed examples.

## 🧩 Modules

The library is organized into independent, reusable modules:

### 🧊 ICE (Interactive Connectivity Establishment)
**Location:** `webrtc/ice/`

Handles network connectivity and NAT traversal:
- Candidate generation (Host, Srflx, Relay, Prflx types)
- Connectivity checks and state management
- Candidate pair formation and prioritization (RFC 5245, RFC 8445)
- UDP socket binding and management

**Key Types:** `IceAgent`, `Candidate`, `ConnectionState`

### 📋 SDP (Session Description Protocol)
**Location:** `webrtc/sdp/`

Session negotiation and description:
- Offer/Answer generation and parsing (RFC 4566)
- Type-safe builder pattern
- Format validation with detailed errors
- ICE attributes integration

**Key Types:** `SessionDescription`, `SdpType`, `MediaDescription`

### 🎥 Media
**Location:** `webrtc/media/`

Video capture and codec support:
- Webcam capture via OpenCV (BGR format)
- H.264 and VP8 encoding/decoding via FFmpeg
- Frame processing and color conversion (BGR ↔ RGB)
- Camera device detection and configuration
- Resolution and FPS management

**Key Types:** `Camera`, `H264Encoder`, `H264Decoder`, `VideoFrame`

### 🌐 Network
**Location:** `webrtc/network/`

RTP transport and packetization:
- RTP packet construction/parsing (RFC 3550)
- H.264 over RTP with FU-A fragmentation (RFC 6184)
- VP8 RTP packetization
- UDP transport with non-blocking I/O
- Automatic packet reassembly

**Key Types:** `UdpTransport`, `H264RtpPacketizer`, `H264RtpDepacketizer`, `RtpPacket`

### 🔌 STUN (Session Traversal Utilities for NAT)
**Location:** `webrtc/stun/`

NAT discovery and public IP resolution:
- STUN client implementation (RFC 5389)
- Public IP discovery via STUN servers
- Binding Request/Response protocol
- Message attribute parsing
- Transaction ID management

**Key Types:** `StunClient`, `StunMessage`, `StunAttribute`

### 🔄 TURN (Traversal Using Relays around NAT)
**Location:** `webrtc/turn/`

Relay allocation for NAT traversal when direct P2P fails:
- TURN client implementation (RFC 5766)
- Relay address allocation
- Permission management
- Channel binding for optimized data transfer
- Allocation refresh and lifetime management
- Support for UDP transport

**Key Types:** `TurnClient`, `TurnMessage`, `TurnMessageType`, `TurnAttributeType`

### 📞 Signaling
**Status:** ⚠️ **Implemented in Backend Server**

SDP exchange is handled by the RoomRTC signaling server (`backend/`) using a custom binary TCP protocol. See `backend/PROTOCOL.md` for details.

## ✨ Features

### ✅ Implemented

- **ICE (Interactive Connectivity Establishment)**
  - Host candidate gathering with automatic port selection
  - Candidate pair formation and prioritization (RFC 5245)
  - Connection state management
  - Local IP detection via STUN or fallback methods

- **SDP (Session Description Protocol)**
  - Type-safe offer/answer generation (RFC 4566)
  - SDP parsing with comprehensive validation
  - Builder pattern for construction
  - ICE candidate integration in SDP attributes

- **Media Pipeline**
  - H.264 and VP8 video encoding/decoding via FFmpeg
  - Camera capture with OpenCV (BGR format)
  - Audio device detection and capture
  - Automatic resolution and FPS detection
  - Fast device discovery (UI-safe, non-blocking)
  - Frame conversion utilities (BGR ↔ RGB)
  - Audio capture with configurable sample rate and channels

- **STUN Client**
  - RFC 5389 compliant implementation
  - Public IP discovery via STUN servers
  - Reflexive candidate gathering
  - Multiple server fallback support
  - Binding Request/Response handling
  - Transaction ID management

- **TURN Client**
  - RFC 5766 compliant relay allocation
  - Allocation and refresh mechanisms
  - Permission and channel binding
  - UDP transport support
  - Authentication via username/password
  - Relay candidate gathering for ICE

- **Network Layer (RTP/RTCP)**
  - RTP packetization (RFC 3550)
  - H.264 over RTP with FU-A fragmentation (RFC 6184)
  - VP8 RTP packetization
  - UDP transport with non-blocking I/O
  - Automatic RTP depacketization and reassembly
  - Packet loss detection and statistics
  - RTCP sender/receiver reports

- **Security (DTLS/SRTP)**
  - DTLS handshake for secure key exchange
  - SRTP encryption/decryption for media streams
  - Certificate fingerprint validation
  - Master key derivation

- **Media Processing**
  - Jitter buffer with adaptive playout delay
  - Packet reordering and duplicate detection
  - Frame synchronization
  - Statistics collection (packet loss, jitter, latency)

- **P2P Session Management**
  - Multi-threaded send/receive pipeline
  - Control message protocol (camera state, participant info)
  - Participant name exchange
  - Graceful disconnect notifications
  - Camera on/off signaling

### 🚧 Limitations

- **Signaling**: Not implemented - applications must provide their own SDP exchange mechanism
- **ICE**: Only host candidates (no STUN/TURN relay candidates yet)
- **NAT Traversal**: Works on local networks or with port forwarding
- **DTLS**: Not implemented (no encryption)
- **SRTP**: Not implemented (RTP only, no encryption)
- **Audio**: Not implemented (video only)
- **Data Channels**: Not implemented
- **Simulcast/SVC**: Not supported
- **Multiple Streams**: Single video stream only

### 📋 Roadmap

- [ ] Signaling module implementation (HTTP, WebSocket transports)
- [ ] STUN server reflexive candidates (Srflx)
- [ ] TURN relay candidates for NAT traversal
- [ ] DTLS handshake for secure connections
- [ ] SRTP for encrypted media
- [ ] Audio support (Opus codec)
- [ ] Multi-party calls (SFU architecture)
- [ ] Adaptive bitrate based on network conditions
- [ ] VP9 codec support

## 🏗️ Architecture

### High-Level Components

The library is structured in layers, from high-level API to low-level protocols:

```text
┌─────────────────────────────────────────────────────────────┐
│                    WebRtcConnection                         │
│          (Main public API for all WebRTC operations)        │
├─────────────────────────────────────────────────────────────┤
│  PeerConnection  │  P2PSession  │  CameraManager            │
│  (ICE/SDP)       │   (Media)    │    (Camera)               │
├─────────────────────────────────────────────────────────────┤
│  ICE   │   SDP   │  Network     │    Media    │    STUN     │
│        │         │  (RTP/UDP)   │  (H264/VP8) │             │
└─────────────────────────────────────────────────────────────┘
```

### 🔧 Core Components

#### **WebRtcConnection** (`webrtc_connection.rs`)
Main entry point providing a simplified API that coordinates all WebRTC operations:
- **Connection Setup**: Creates offers/answers with automatic port selection
- **Camera Management**: Discovery, initialization, and frame capture
- **Media Streaming**: Send/receive video frames with automatic encoding/decoding
- **State Management**: Tracks connection state and handles lifecycle

**Key Methods:**
- `create_offer_from_new()` / `create_answer_from_new()` - One-shot connection setup
- `start_camera()` / `stop_camera()` - Camera lifecycle control
- `capture_and_send()` - Captures frame, sends it, returns RGB for preview
- `receive_frame()` - Receives decoded frame as RGB
- `send_control_message()` / `receive_control_message()` - Camera state sync

#### **PeerConnection** (`peer_connection.rs`)
Manages ICE candidate gathering and SDP negotiation:
- Generates host candidates on specified UDP ports
- Creates/parses SDP offers and answers
- Imports remote ICE candidates from SDP
- Validates connection readiness

#### **P2PSession** (`p2p/session.rs`)
Complete media pipeline with multi-threaded encoding/decoding:
- **Send Thread**: Captures → Encodes H.264 → Packetizes RTP → Sends UDP
- **Receive Thread**: Receives UDP → Depacketizes RTP → Decodes H.264 → Delivers frames
- Automatic SPS/PPS parameter set delivery
- Packet loss detection and statistics
- Ultra-low latency with sync channels (buffer=1)

#### **CameraManager** (`camera_manager.rs`)
Camera device lifecycle and frame capture:
- Fast device detection (`list_camera_ids_fast()`)
- Auto-detection with fallback (`start_camera_auto()`)
- Resolution and FPS configuration
- Hardware resource management

#### **AudioManager** (`audio_manager.rs`)
Audio device lifecycle and audio capture:
- Audio device detection and enumeration
- Auto-detection with default device selection
- Configurable sample rate (8kHz-192kHz) and channels (mono/stereo)
- 16-bit PCM audio capture
- Buffer size configuration for latency control

### 🔄 P2P Session Pipeline

The media pipeline implements efficient multi-threaded streaming:

```text
Send Pipeline:
VideoFrame (BGR) → H264Encoder → RtpPacketizer → UdpTransport → Network

Receive Pipeline:
Network → UdpTransport → RtpDepacketizer → H264Decoder → VideoFrame (BGR)
```

**Threading Model:**
- **Send Thread**: Encodes frames and packetizes to RTP
- **Receive Thread**: Depacketizes RTP and decodes frames
- **Sync Channels**: Ultra-low latency with minimal buffering (capacity=1)
- **Control Messages**: Separate channel for camera state synchronization

## 📖 Usage Examples

### 📹 Basic P2P Video Call

**Offerer (Initiator):**
```rust
use webrtc::{WebRtcConnection, CameraInfo};
use logging::Logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new();
    
    // Create connection and generate offer
    let (mut conn, offer_sdp) = WebRtcConnection::create_offer_from_new(logger)?;
    
    // Send offer_sdp to peer via signaling (HTTP, WebSocket, etc.)
    // ... signaling logic ...
    
    // Receive answer from peer
    let answer_sdp = receive_answer_from_peer()?;
    conn.set_remote_answer(&answer_sdp)?;
    
    // Establish connection
    conn.establish_connection()?;
    
    // Start camera
    conn.start_camera_auto(30.0)?;
    
    // Main streaming loop
    loop {
        // Capture, encode, send, and get RGB for local preview
        let (width, height, rgb_pixels) = conn.capture_and_send()?;
        render_local_preview(width, height, &rgb_pixels);
        
        // Receive remote peer's video
        if let Some((width, height, rgb_pixels)) = conn.receive_frame()? {
            render_remote_video(width, height, &rgb_pixels);
        }
        
        // Check for camera state changes from peer
        if let Some(msg) = conn.receive_control_message()? {
            handle_control_message(msg);
        }
    }
}
```

**Answerer (Responder):**
```rust
use webrtc::WebRtcConnection;
use logging::Logger;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new();
    
    // Receive offer from peer
    let offer_sdp = receive_offer_from_peer()?;
    
    // Create connection and generate answer
    let (mut conn, answer_sdp) = 
        WebRtcConnection::create_answer_from_new(&offer_sdp, logger)?;
    
    // Send answer_sdp to peer
    send_answer_to_peer(&answer_sdp)?;
    
    // Establish connection
    conn.establish_connection()?;
    
    // Start camera and stream
    conn.start_camera_auto(30.0)?;
    
    loop {
        let (width, height, rgb_pixels) = conn.capture_and_send()?;
        render_local_preview(width, height, &rgb_pixels);
        
        if let Some((width, height, rgb_pixels)) = conn.receive_frame()? {
            render_remote_video(width, height, &rgb_pixels);
        }
    }
}
```

### 🎥 Camera Discovery and Selection

```rust
use webrtc::{WebRtcConnection, CameraInfo};
use logging::Logger;

fn discover_and_select_camera() -> Result<i32, Box<dyn std::error::Error>> {
    let logger = Logger::new();
    let mut conn = WebRtcConnection::new(None, logger)?;
    
    // Fast check if camera is available (UI-safe, no blocking)
    if !WebRtcConnection::is_camera_available() {
        println!("No camera detected on system");
        return Err("No camera available".into());
    }
    
    // List device IDs quickly (no hardware probing)
    let device_ids = WebRtcConnection::list_camera_ids_fast();
    println!("Found {} potential camera device(s)", device_ids.len());
    
    // Full camera discovery with detailed info (expensive operation)
    let cameras = conn.discover_cameras()?;
    
    for camera in &cameras {
        println!("Camera {}: {}", camera.device_id, camera.name);
        println!("  Resolution: {}", camera.resolution_string());
        println!("  Supported FPS: {:?}", camera.supported_fps);
        println!("  Recommended FPS: {:.1}", camera.recommended_fps());
    }
    
    // Select first available camera
    let camera_id = cameras.first()
        .map(|c| c.device_id)
        .ok_or("No cameras found")?;
    
    Ok(camera_id)
}

fn start_specific_camera() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new();
    let mut conn = WebRtcConnection::new(None, logger)?;
    
    let camera_id = discover_and_select_camera()?;
    
    // Start with specific device and FPS
    conn.start_camera(camera_id, 30.0)?;
    
    println!("Camera {} started successfully", camera_id);
    Ok(())
}
```

### 🎤 Audio Device Discovery and Capture

```rust
use webrtc::{AudioManager, AudioInfo};
use logging::Logger;

fn discover_and_start_audio() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new("audio_example");
    let mut audio_manager = AudioManager::new(logger);
    
    // Discover available audio input devices
    let devices = audio_manager.discover_devices()?;
    
    for device in &devices {
        println!("Audio Device {}: {}", device.device_id, device.name);
        println!("  Max Sample Rate: {}", device.sample_rate_string());
        println!("  Supported Channels: {:?}", device.supported_channels);
        println!("  Recommended: {} Hz, {} channels",
            device.recommended_sample_rate(),
            device.recommended_channels()
        );
    }
    
    // Start with auto-detected device (48kHz, stereo)
    let settings = audio_manager.start_audio_auto(48000, 2)?;
    println!("Audio started: {} Hz, {} channels, buffer: {} frames",
        settings.sample_rate,
        settings.channels,
        settings.buffer_size
    );
    
    // Capture audio frames
    for _ in 0..100 {
        let frame = audio_manager.capture_frame()?;
        println!("Captured {} samples, duration: {:.2}ms",
            frame.samples.len(),
            frame.duration_ms()
        );
        // Process audio here...
    }
    
    // Stop when done
    audio_manager.stop_audio();
    Ok(())
}

fn start_specific_audio_device() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new("audio");
    let mut audio_manager = AudioManager::new(logger);
    
    // Start specific device (device_id: 1, 44.1kHz, mono)
    let settings = audio_manager.start_audio(Some(1), 44100, 1)?;
    println!("Audio device 1 started: {:?}", settings);
    
    Ok(())
}
```

### 🎬 Combined Audio + Video Capture

```rust
use webrtc::{AudioManager, CameraManager};
use logging::Logger;

fn audio_video_capture() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new("av_capture");
    
    // Initialize camera and audio managers
    let mut camera_manager = CameraManager::new(logger.clone());
    let mut audio_manager = AudioManager::new(logger.clone());
    
    // Start camera (auto-detect, 30 fps)
    let camera_res = camera_manager.start_camera_auto(30.0)?;
    println!("Camera: {}x{} @ {:.1} fps",
        camera_res.width, camera_res.height, camera_res.fps);
    
    // Start audio (48kHz, stereo)
    let audio_settings = audio_manager.start_audio_auto(48000, 2)?;
    println!("Audio: {} Hz, {} channels",
        audio_settings.sample_rate, audio_settings.channels);
    
    // Capture loop
    loop {
        // Capture video frame
        let video_frame = camera_manager.capture_frame()?;
        
        // Capture audio frame
        let audio_frame = audio_manager.capture_frame()?;
        
        // Encode and transmit both...
        // encode_and_send(video_frame, audio_frame)?;
        
        break; // Remove in production
    }
    
    // Cleanup
    camera_manager.stop_camera();
    audio_manager.stop_audio();
    
    Ok(())
}
```

### ⚙️ Manual Connection Setup (Advanced)

```rust
use webrtc::WebRtcConnection;
use logging::Logger;

fn manual_setup() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new();
    
    // Create connection with specific port
    let mut conn = WebRtcConnection::new(Some(5000), logger)?;
    
    // Create offer manually
    let offer = conn.create_offer()?;
    println!("Generated offer:\n{}", offer);
    
    // Exchange SDP via custom signaling...
    let remote_answer = exchange_sdp_via_custom_signaling(&offer)?;
    
    // Set remote SDP (auto-detects type)
    conn.set_remote_sdp(&remote_answer)?;
    
    // Establish connection
    conn.establish_connection()?;
    
    println!("Connected! Local port: {}, Remote port: {}", 
             conn.local_port(), conn.remote_port());
    
    Ok(())
}
```

### 💬 Control Messages and State Sync

```rust
use webrtc::{WebRtcConnection, ControlMessage};
use logging::Logger;

fn handle_peer_communication() -> Result<(), Box<dyn std::error::Error>> {
    let logger = Logger::new();
    let mut conn = WebRtcConnection::new(None, logger)?;
    
    // ... setup connection ...
    
    // Send participant name to peer
    conn.send_participant_name("Alice")?;
    
    loop {
        // Process control messages from peer
        if let Some(msg) = conn.receive_control_message()? {
            match msg {
                ControlMessage::CameraOn => {
                    println!("Peer turned camera ON");
                }
                ControlMessage::CameraOff => {
                    println!("Peer turned camera OFF");
                }
                ControlMessage::ParticipantName(name) => {
                    println!("Peer name: {}", name);
                }
                ControlMessage::ParticipantDisconnected => {
                    println!("Peer disconnected");
                    break;
                }
                ControlMessage::OwnerDisconnected => {
                    println!("Room owner disconnected");
                    break;
                }
            }
        }
        
        // ... video streaming logic ...
    }
    
    // Graceful shutdown - notify peer before disconnecting
    conn.send_disconnect_message(false)?; // false = not room owner
    
    Ok(())
}
```

### 🔌 Using Individual Modules

The library's modular design allows using components independently:

#### ICE Only
```rust
use ice::IceAgent;

fn ice_example() -> Result<(), Box<dyn std::error::Error>> {
    let mut agent = IceAgent::new();
    agent.gather_host_candidates(5000)?;
    
    for candidate in &agent.local_candidates {
        println!("Candidate: {}", candidate);
    }
    
    Ok(())
}
```

#### SDP Only
```rust
use sdp::{SessionDescription, SdpType, Origin, Timing};

fn sdp_example() -> Result<(), Box<dyn std::error::Error>> {
    let sdp = SessionDescription::builder(SdpType::Offer)
        .origin(Origin::default())
        .session_name("Example Session")
        .timing(Timing::default())
        .build()?;
    
    println!("{}", sdp.to_string());
    Ok(())
}
```

#### RTP Packetization Only
```rust
use network::{H264RtpPacketizer, RtpPacketizer};

fn rtp_example() {
    let mut packetizer = H264RtpPacketizer::new(96, 1200, 30.0);
    let h264_data = vec![0x00, 0x00, 0x00, 0x01, 0x67, /* ... */];
    
    let rtp_packets = packetizer.packetize(&h264_data);
    println!("Generated {} RTP packets", rtp_packets.len());
}
```

## 📚 Public API

External users should **only** use these public types from the `webrtc` crate:

### High-Level API
- **`WebRtcConnection`** - Main interface for all WebRTC operations
- **`RgbFrame`** - Type alias for RGB frame data: `(width, height, pixel_data)`
- **`CameraInfo`** - Camera device information
- **`CameraManager`** - Camera lifecycle management
- **`CameraResolution`** - Camera resolution information
- **`AudioInfo`** - Audio device information
- **`AudioManager`** - Audio lifecycle management
- **`AudioSettings`** - Audio configuration information
- **`AudioFrame`** - Audio sample data (16-bit PCM)
- **`ControlMessage`** - Control message types (CameraOn, CameraOff, ParticipantDisconnected, etc.)

### ICE/STUN/TURN API (for signaling servers)
- **`IceAgent`** - ICE candidate gathering and management
- **`Candidate`** - ICE candidate representation
- **`CandidateType`** - Candidate types (Host, Srflx, Relay)
- **`StunClient`** - STUN client for NAT discovery
- **`TurnClient`** - TURN client for relay allocation

### SDP API (for signaling)
- **`SessionDescription`** - Parsed SDP structure
- **`SessionDescriptionBuilder`** - Fluent SDP builder
- **`MediaDescription`** - SDP media line representation

All other types are internal implementation details and subject to change.

For detailed audio usage, see [AUDIO.md](AUDIO.md).

## 📦 Dependencies

- **OpenCV** (`opencv` crate) - Camera capture and image processing
- **FFmpeg** (`ffmpeg-next` crate) - H.264/VP8 encoding and decoding
- Custom implementations of ICE, SDP, RTP, STUN (no external WebRTC libraries)

## 📖 References

- [RFC 5389 - STUN](https://datatracker.ietf.org/doc/html/rfc5389)
- [RFC 4566 - SDP](https://datatracker.ietf.org/doc/html/rfc4566)
- [RFC 8445 - ICE](https://datatracker.ietf.org/doc/html/rfc8445)
- [RFC 5245 - ICE (Legacy)](https://datatracker.ietf.org/doc/html/rfc5245)
- [RFC 3550 - RTP](https://datatracker.ietf.org/doc/html/rfc3550)
- [RFC 6184 - H.264 over RTP](https://datatracker.ietf.org/doc/html/rfc6184)
