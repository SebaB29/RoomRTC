//! File transfer protocol over WebRTC data channels
//!
//! Implements reliable file transfer with chunking, progress tracking,
//! and accept/reject flow.

mod event;
mod incoming;
mod message;
mod outgoing;
mod state;

pub use event::FileTransferEvent;
pub use incoming::IncomingTransfer;
pub use message::FileTransferMessage;
pub use outgoing::OutgoingTransfer;
pub use state::FileTransferState;
