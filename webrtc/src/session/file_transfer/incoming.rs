//! Incoming file transfer (receiver side)

use super::state::FileTransferState;
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;

/// Incoming file transfer (receiver side)
#[derive(Debug)]
pub struct IncomingTransfer {
    /// Total file size in bytes
    pub total_size: u64,
    /// Bytes received so far
    pub bytes_received: u64,
    /// Save path (set when accepted)
    pub save_path: Option<PathBuf>,
    /// Current state
    pub state: FileTransferState,
    /// Received chunks (for reassembly)
    chunks: HashMap<u64, Vec<u8>>,
}

impl IncomingTransfer {
    /// Create new incoming transfer
    pub fn new(size: u64) -> Self {
        Self {
            total_size: size,
            bytes_received: 0,
            save_path: None,
            state: FileTransferState::Pending,
            chunks: HashMap::new(),
        }
    }

    /// Accept the transfer and set save path
    pub fn accept(&mut self, save_path: PathBuf) {
        if self.state == FileTransferState::Pending {
            self.save_path = Some(save_path);
            self.state = FileTransferState::Transferring;
        }
    }

    /// Reject the transfer
    pub fn reject(&mut self) {
        self.state = FileTransferState::Cancelled;
    }

    /// Receive a data chunk
    pub fn receive_chunk(&mut self, offset: u64, data: Vec<u8>) -> io::Result<()> {
        if self.state != FileTransferState::Transferring {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Transfer not in progress",
            ));
        }
        let data_len = data.len() as u64;

        // Check for duplicate or out-of-bounds chunks
        if self.chunks.contains_key(&offset) {
            // Duplicate chunk, skip it
            return Ok(());
        }

        if self.bytes_received + data_len > self.total_size {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Received more data than expected",
            ));
        }

        self.chunks.insert(offset, data);
        self.bytes_received += data_len;
        Ok(())
    }

    /// Finalize the transfer - write all chunks to file
    pub fn finalize(&mut self) -> io::Result<PathBuf> {
        let save_path = self
            .save_path
            .as_ref()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "No save path set"))?;

        let mut file = std::fs::File::create(save_path)?;

        // Sort chunks by offset and write
        let mut offsets: Vec<_> = self.chunks.keys().copied().collect();
        offsets.sort();

        for offset in offsets {
            if let Some(data) = self.chunks.get(&offset) {
                file.write_all(data)?;
            }
        }

        file.flush()?;
        self.state = FileTransferState::Completed;

        Ok(save_path.clone())
    }
}
