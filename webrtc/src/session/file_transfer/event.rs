//! File transfer events

use std::path::PathBuf;

/// Events emitted by file transfer system
#[derive(Debug, Clone)]
pub enum FileTransferEvent {
    /// Incoming file offer from peer
    IncomingOffer {
        id: u64,
        filename: String,
        size: u64,
    },
    /// File transfer was accepted by peer
    Accepted { id: u64 },
    /// File transfer was rejected by peer
    Rejected { id: u64, reason: String },
    /// Transfer progress update
    Progress { id: u64, bytes: u64, total: u64 },
    /// Transfer completed successfully
    Completed { id: u64, path: PathBuf },
    /// Transfer failed or was cancelled
    Failed { id: u64, reason: String },
}
