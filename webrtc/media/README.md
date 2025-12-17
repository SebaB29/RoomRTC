# Media - Video Capture and Codec Library

High-performance media capture and codec module for WebRTC applications.

## Overview

This module provides a complete media pipeline for WebRTC, including camera detection, video capture, and codec operations. Built with OpenCV for capture and FFmpeg for encoding/decoding, it offers cross-platform support and automatic device detection.

## Features

- ✅ Automatic camera detection and enumeration
- ✅ Cross-platform camera capture (Linux, macOS, Windows)
- ✅ H.264 and VP8 encoding/decoding
- ✅ Configurable bitrate and FPS
- ✅ YUV ↔ BGR color space conversion
- ✅ Thread-safe operations
- ✅ Type-safe error handling

## Quick Start

### List Available Cameras

```rust
use media::{CameraDetection, Camera};
use logging::{Logger, LogLevel};

let logger = Logger::new("media.log".into(), LogLevel::Info)?;

// Detect all available cameras
let cameras = CameraDetection::list_devices(&logger)?;
for camera in cameras {
    println!("Device {}: {} - {}x{}", 
        camera.device_id, 
        camera.name,
        camera.max_width, 
        camera.max_height
    );
}
```

### Auto-Detect and Capture

```rust
use media::Camera;

// Automatically select first available camera at maximum resolution
let mut camera = Camera::new_auto(30.0, logger)?;

// Capture a frame
let frame = camera.capture_frame()?;
println!("Captured {}x{} frame", frame.width(), frame.height());
```

### Manual Camera Configuration

```rust
use media::{Camera, CameraConfig};

let config = CameraConfig::new(0, 30.0)?
    .with_resolution(1920, 1080)?;

let mut camera = Camera::new(config, logger)?;
let frame = camera.capture_frame()?;
```

### Encode with H.264

```rust
use media::{H264Encoder, VideoEncoder};

let mut encoder = H264Encoder::new(
    1920,       // width
    1080,       // height
    2_000_000,  // bitrate (2 Mbps)
    60,         // keyframe interval (GOP size)
    30.0,       // fps
    logger
)?;

// Encode a frame
let packets = encoder.encode(&frame)?;
for packet in packets {
    println!("Encoded packet: {} bytes", packet.len());
}

// Or use the trait for polymorphism
let encoded_data = VideoEncoder::encode(&mut encoder, &frame)?;
```

### Encode with VP8

```rust
use media::{VP8Encoder, VideoEncoder};

let mut encoder = VP8Encoder::new(
    1920, 1080, 2_000_000, 60, 30.0, logger
)?;

let encoded = encoder.encode(&frame)?;
```

### Decode Video

```rust
use media::{H264Decoder, VP8Decoder, VideoDecoder};

// H.264 decoding
let mut h264_decoder = H264Decoder::new(logger)?;
let frame = h264_decoder.decode(&h264_data)?;

// VP8 decoding
let mut vp8_decoder = VP8Decoder::new(logger)?;
let frame = vp8_decoder.decode(&vp8_data)?;
```

## Architecture

### Media Pipeline

```
┌──────────────────┐
│ CameraDetection  │ → Enumerate available cameras
└──────────────────┘
         │
         ▼
┌──────────────────┐
│      Camera      │ → Capture BGR frames (OpenCV)
└──────────────────┘
         │
         ▼
┌──────────────────┐
│    VideoFrame    │ → Raw frame with metadata
└──────────────────┘
         │
         ▼
┌──────────────────┐
│     Encoder      │ → BGR → YUV → H.264/VP8 (FFmpeg)
│  (H264 or VP8)   │
└──────────────────┘
         │
         ▼
  [Compressed data]
         │
         ▼
┌──────────────────┐
│     Decoder      │ → H.264/VP8 → YUV → BGR (FFmpeg)
│  (H264 or VP8)   │
└──────────────────┘
         │
         ▼
┌──────────────────┐
│   VideoFrame     │ → Display or processing
└──────────────────┘
```

## Supported Codecs

### H.264 (AVC)
- Industry standard, excellent compression
- Wide hardware support
- Best for cross-platform compatibility

### VP8
- Open-source codec
- WebRTC standard
- Good compression with lower complexity than H.264

## Performance

Approximate performance on modern CPU (i7/M1):

| Operation | Time | Notes |
|-----------|------|-------|
| Camera Detection (Linux) | ~50-100ms | Scans /dev efficiently |
| Camera Detection (macOS) | ~100-200ms | Tests device IDs |
| Frame Capture | ~5-10ms | Camera dependent |
| H.264 Encoding | ~20-30ms | 1080p @ 30fps |
| H.264 Decoding | ~15-25ms | 1080p @ 30fps |
| VP8 Encoding | ~25-35ms | 1080p @ 30fps |
| VP8 Decoding | ~20-30ms | 1080p @ 30fps |

**Memory:** ~3MB per uncompressed frame (1080p), ~10-50KB compressed

## Error Handling

All operations return `Result<T, MediaError>` for type-safe error handling:

```rust
use media::MediaError;

match camera.capture_frame() {
    Ok(frame) => { /* use frame */ },
    Err(MediaError::Camera(msg)) => eprintln!("Camera error: {}", msg),
    Err(MediaError::Codec(msg)) => eprintln!("Codec error: {}", msg),
    Err(MediaError::Config(msg)) => eprintln!("Configuration error: {}", msg),
    Err(MediaError::Processing(msg)) => eprintln!("Processing error: {}", msg),
    Err(e) => eprintln!("Other error: {}", e),
}
```

### Error Types

- **`MediaError::Config`**: Invalid configuration parameters (e.g., non-finite FPS, invalid resolution)
- **`MediaError::Camera`**: Camera access or capture failures
- **`MediaError::Codec`**: Encoding/decoding errors
- **`MediaError::Processing`**: Frame processing errors
- **`MediaError::Io`**: File I/O errors

All constructors and methods validate inputs and return proper errors instead of panicking.

## Camera Detection

The module provides efficient automatic camera detection with platform-specific optimizations:

### Platform-Specific Behavior

- **Linux**: Scans `/dev` directory for video devices. Only checks even-numbered devices (video0, video2, etc.) to avoid metadata node duplicates. Reads friendly device names from `/sys/class/video4linux/` when available.
  
- **macOS**: Tests common device IDs (0-3) with quick availability checks. Uses system-provided device naming.

- **Windows**: Tests standard device IDs (0-3) for camera enumeration.

### Auto-Selection Strategy

When using `Camera::new_auto()`, the module:
1. Scans all available cameras
2. Prioritizes cameras with higher resolution (larger pixel area)
3. Automatically configures maximum resolution
4. Logs detailed selection information

## Dependencies

### Required Libraries

- **OpenCV 4.x**: Camera capture and image processing
- **FFmpeg 4.x+**: Video encoding/decoding
- **libx264**: H.264 codec implementation
- **libvpx**: VP8/VP9 codec implementation

### Installation

**Ubuntu/Debian:**
```bash
sudo apt-get install libopencv-dev libavcodec-dev libavformat-dev \
                     libavutil-dev libswscale-dev libx264-dev \
                     libvpx-dev clang pkg-config
```

**macOS:**
```bash
brew install opencv ffmpeg pkg-config
```

**Windows:**
- Download OpenCV from https://opencv.org/releases/
- Download FFmpeg from https://ffmpeg.org/download.html
- Ensure development libraries are in your PATH

## References

- [ITU-T H.264 Specification](https://www.itu.int/rec/T-REC-H.264)
- [VP8 Data Format and Decoding Guide](https://datatracker.ietf.org/doc/html/rfc6386)
- [FFmpeg Documentation](https://ffmpeg.org/documentation.html)
- [OpenCV VideoCapture](https://docs.opencv.org/4.x/d8/dfe/classcv_1_1VideoCapture.html)
