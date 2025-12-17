//! Outgoing file transfer (sender side)

use super::state::FileTransferState;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::PathBuf;

/// Outgoing file transfer (sender side) with adaptive chunking
#[derive(Debug)]
pub struct OutgoingTransfer {
    /// Unique transfer ID
    pub id: u64,
    /// File path
    pub path: PathBuf,
    /// Total file size
    pub total_size: u64,
    /// Bytes sent so far
    pub bytes_sent: u64,
    /// Current state
    pub state: FileTransferState,
    /// Current adaptive chunk size
    chunk_size: usize,
    /// Number of successful chunks sent
    successful_chunks: u64,
    /// Number of failed/slow chunks
    failed_chunks: u64,
}

impl OutgoingTransfer {
    // Chunk sizes MUST fit within DTLS/UDP MTU (~1400 bytes usable)
    // SCTP implementation doesn't fragment
    const MIN_CHUNK_SIZE: usize = 512; // 0.5KB minimum
    const MAX_CHUNK_SIZE: usize = 1200; // ~1.2KB max (MTU-safe)
    const INITIAL_CHUNK_SIZE: usize = 1024; // 1KB start

    /// Create new outgoing transfer
    pub fn new(id: u64, path: PathBuf, size: u64) -> Self {
        Self {
            id,
            path,
            total_size: size,
            bytes_sent: 0,
            state: FileTransferState::Pending,
            chunk_size: Self::INITIAL_CHUNK_SIZE,
            successful_chunks: 0,
            failed_chunks: 0,
        }
    }

    /// Mark as accepted and start transferring
    pub fn accept(&mut self) {
        if self.state == FileTransferState::Pending {
            self.state = FileTransferState::Transferring;
        }
    }

    /// Mark as rejected
    pub fn reject(&mut self) {
        self.state = FileTransferState::Cancelled;
    }

    /// Adapt chunk size based on success rate (like TCP slow start)
    pub fn adapt_chunk_size(&mut self, success: bool) {
        if success {
            self.successful_chunks += 1;
            if self.successful_chunks.is_multiple_of(10) && self.chunk_size < Self::MAX_CHUNK_SIZE {
                self.chunk_size = (self.chunk_size * 3 / 2).min(Self::MAX_CHUNK_SIZE);
            }
        } else {
            self.failed_chunks += 1;
            self.chunk_size = (self.chunk_size / 2).max(Self::MIN_CHUNK_SIZE);
        }
    }

    // /// Get current chunk size
    // pub fn get_chunk_size(&self) -> usize {
    //     self.chunk_size
    // }

    /// Read next chunk from file
    pub fn read_next_chunk(&mut self) -> io::Result<Option<Vec<u8>>> {
        if self.state != FileTransferState::Transferring {
            return Ok(None);
        }
        if self.bytes_sent >= self.total_size {
            return Ok(None);
        }

        let mut file = std::fs::File::open(&self.path)?;
        file.seek(SeekFrom::Start(self.bytes_sent))?;

        let remaining = (self.total_size - self.bytes_sent) as usize;
        let chunk_size = remaining.min(self.chunk_size);
        let mut buffer = vec![0u8; chunk_size];

        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            return Ok(None);
        }

        buffer.truncate(bytes_read);
        self.bytes_sent += bytes_read as u64;

        if self.bytes_sent >= self.total_size {
            self.state = FileTransferState::Completed;
        }

        Ok(Some(buffer))
    }
}
