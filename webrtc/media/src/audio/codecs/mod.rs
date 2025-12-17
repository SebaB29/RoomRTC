//! Audio codecs module
//!
//! Audio encoding and decoding implementations.

pub mod opus;

pub use opus::{OpusDecoder, OpusEncoder};
