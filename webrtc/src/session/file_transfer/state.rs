//! File transfer state enum

/// File transfer state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileTransferState {
    /// Waiting for acceptance
    Pending,
    /// Transfer in progress
    Transferring,
    /// Transfer completed
    Completed,
    /// Transfer cancelled or rejected
    Cancelled,
}
