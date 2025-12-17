//! WebRTC connection module
//!
//! Modularized WebRTC connection implementation

mod audio;
mod camera;
mod ice;
mod sdp;
mod webrtc_connection;

pub use webrtc_connection::{RgbFrame, WebRtcConnection};
