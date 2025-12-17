//! Data Channel abstraction over SCTP
//!
//! Provides a high-level API for WebRTC data channels, built on top of SCTP.
//!
//! ## Usage
//!
//! ```ignore
//! let mut manager = DataChannelManager::new(sctp_association);
//! let channel_id = manager.create_channel("file-transfer")?;
//! manager.send(channel_id, &data)?;
//! ```

mod channel;
mod manager;

pub use channel::{DataChannel, DataChannelConfig, DataChannelState};
pub use manager::{DataChannelEvent, DataChannelManager};
