//! Audio processing module
//!
//! Handles audio capture, encoding, decoding, and playback.

pub mod capture;
pub mod codecs;
pub mod config;
pub mod detection;
pub mod device;
pub mod frame;
pub mod info;
pub mod playback;
pub mod traits;

pub use capture::AudioCapture;
pub use codecs::{OpusDecoder, OpusEncoder};
pub use config::AudioConfig;
pub use detection::AudioDetection;
pub use device::Audio;
pub use frame::{AudioFrame, AudioSample};
pub use info::AudioInfo;
pub use playback::AudioPlayback;
pub use traits::{AudioDecoder, AudioEncoder};
