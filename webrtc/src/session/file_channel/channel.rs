//! File channel implementation

use super::mime::guess_mime_type;
use crate::session::file_transfer::{
    FileTransferEvent, FileTransferMessage, FileTransferState, IncomingTransfer, OutgoingTransfer,
};
use std::collections::HashMap;
use std::io;
use std::path::Path;

/// File channel for managing file transfers
#[derive(Debug)]
pub struct FileChannel {
    channel_id: u16,
    next_transfer_id: u64,
    outgoing: HashMap<u64, OutgoingTransfer>,
    incoming: HashMap<u64, IncomingTransfer>,
    events: Vec<FileTransferEvent>,
    send_queue: Vec<FileTransferMessage>,
    /// Current buffered amount (bytes pending in SCTP buffer)
    buffered_amount: usize,
    /// Max buffered amount before pausing (1MB)
    max_buffered_amount: usize,
}

impl FileChannel {
    /// Create new file channel
    pub fn new(channel_id: u16) -> Self {
        Self {
            channel_id,
            next_transfer_id: rand::random::<u64>() & 0x0000_FFFF_FFFF_FFFF,
            outgoing: HashMap::new(),
            incoming: HashMap::new(),
            events: Vec::new(),
            send_queue: Vec::new(),
            buffered_amount: 0,
            max_buffered_amount: 1024 * 1024, // 1MB - supports adaptive chunking
        }
    }

    /// Get the data channel ID
    pub fn channel_id(&self) -> u16 {
        self.channel_id
    }

    /// Notify that bytes were successfully sent (reduces buffered amount)
    pub fn on_bytes_sent(&mut self, bytes: usize) {
        self.buffered_amount = self.buffered_amount.saturating_sub(bytes);

        // Adaptive feedback: successful send, increase chunk size
        for transfer in self.outgoing.values_mut() {
            if transfer.state == FileTransferState::Transferring {
                transfer.adapt_chunk_size(true);
                break; // Only adapt active transfer
            }
        }
    }

    /// Check if we can send more data (flow control)
    pub fn can_send(&self) -> bool {
        self.buffered_amount < self.max_buffered_amount
    }

    /// Start sending a file
    pub fn send_file(&mut self, path: &Path) -> io::Result<u64> {
        let metadata = std::fs::metadata(path)?;
        if !metadata.is_file() {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Not a file"));
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        let size = metadata.len();
        let mime_type = guess_mime_type(&filename);

        let id = self.next_transfer_id;
        self.next_transfer_id += 1;

        let transfer = OutgoingTransfer::new(id, path.to_path_buf(), size);
        self.outgoing.insert(id, transfer);

        self.send_queue.push(FileTransferMessage::Offer {
            id,
            filename,
            size,
            mime_type,
        });

        Ok(id)
    }

    /// Accept an incoming file transfer
    pub fn accept_transfer(&mut self, id: u64, save_path: &Path) -> Result<(), &'static str> {
        let transfer = self.incoming.get_mut(&id).ok_or("Transfer not found")?;
        if transfer.state != FileTransferState::Pending {
            return Err("Transfer not pending");
        }
        transfer.accept(save_path.to_path_buf());
        self.send_queue.push(FileTransferMessage::Accept { id });
        Ok(())
    }

    /// Reject an incoming file transfer
    pub fn reject_transfer(&mut self, id: u64, reason: &str) -> Result<(), &'static str> {
        let transfer = self.incoming.get_mut(&id).ok_or("Transfer not found")?;
        if transfer.state != FileTransferState::Pending {
            return Err("Transfer not pending");
        }
        transfer.reject();
        self.send_queue.push(FileTransferMessage::Reject {
            id,
            reason: reason.to_string(),
        });
        Ok(())
    }

    /// Cancel an ongoing transfer
    pub fn cancel_transfer(&mut self, id: u64, reason: &str) -> Result<(), &'static str> {
        let found = self.outgoing.remove(&id).is_some() || self.incoming.remove(&id).is_some();

        if !found {
            return Err("Transfer not found");
        }

        // Send cancel message to peer
        self.send_queue.push(FileTransferMessage::Cancel {
            id,
            reason: reason.to_string(),
        });

        // Generate failed event locally
        self.events.push(FileTransferEvent::Failed {
            id,
            reason: format!("Cancelled: {}", reason),
        });

        Ok(())
    }

    /// Process incoming data from data channel
    pub fn on_data(&mut self, data: &[u8]) {
        let msg = match FileTransferMessage::from_bytes(data) {
            Ok(m) => m,
            Err(_) => {
                return;
            }
        };
        match msg {
            FileTransferMessage::Offer {
                id,
                filename,
                size,
                mime_type: _,
            } => {
                let transfer = IncomingTransfer::new(size);
                self.incoming.insert(id, transfer);
                self.events
                    .push(FileTransferEvent::IncomingOffer { id, filename, size });
            }
            FileTransferMessage::Accept { id } => {
                if let Some(transfer) = self.outgoing.get_mut(&id) {
                    transfer.accept();
                    self.events.push(FileTransferEvent::Accepted { id });
                }
            }
            FileTransferMessage::Reject { id, reason } => {
                if let Some(transfer) = self.outgoing.get_mut(&id) {
                    transfer.reject();
                    self.events.push(FileTransferEvent::Rejected { id, reason });
                }
            }
            FileTransferMessage::Data { id, offset, data } => {
                if let Some(transfer) = self.incoming.get_mut(&id) {
                    let _ = transfer.receive_chunk(offset, data);
                    self.events.push(FileTransferEvent::Progress {
                        id,
                        bytes: transfer.bytes_received,
                        total: transfer.total_size,
                    });
                }
            }
            FileTransferMessage::Complete { id, .. } => {
                if let Some(transfer) = self.incoming.get_mut(&id) {
                    match transfer.finalize() {
                        Ok(path) => {
                            self.events.push(FileTransferEvent::Completed {
                                id,
                                path: path.clone(),
                            });
                            // Remove completed transfer from map
                            self.incoming.remove(&id);
                        }
                        Err(e) => {
                            self.events.push(FileTransferEvent::Failed {
                                id,
                                reason: e.to_string(),
                            });
                            // Remove failed transfer from map
                            self.incoming.remove(&id);
                        }
                    }
                }
            }
            FileTransferMessage::Cancel { id, reason } => {
                if let Some(t) = self.outgoing.get_mut(&id) {
                    t.state = FileTransferState::Cancelled;
                }
                if let Some(t) = self.incoming.get_mut(&id) {
                    t.state = FileTransferState::Cancelled;
                }
                // Remove cancelled transfers from maps
                self.outgoing.remove(&id);
                self.incoming.remove(&id);
                self.events.push(FileTransferEvent::Failed { id, reason });
            }
        }
    }

    /// Poll for next message to send
    pub fn poll_send(&mut self) -> Option<Vec<u8>> {
        // First, check send queue for control messages (Complete, etc.)
        if let Some(msg) = self.send_queue.pop() {
            // Check if this is a Complete message - if so, generate completion event for sender
            if let FileTransferMessage::Complete { id, .. } = &msg {
                if let Some(transfer) = self.outgoing.get(id) {
                    let path = transfer.path.clone();
                    self.events
                        .push(FileTransferEvent::Completed { id: *id, path });
                }
                // Remove the completed transfer
                self.outgoing.remove(id);
            }
            let bytes = msg.to_bytes();
            return Some(bytes);
        }

        // Check flow control before sending data chunks
        if !self.can_send() {
            return None; // Buffer full, wait for on_bytes_sent() callback
        }

        // Then, check for data to send from active transfers
        // Apply flow control: only read next chunk if buffer has space
        if self.buffered_amount >= self.max_buffered_amount {
            return None; // Buffer full, wait for data to be sent
        }

        let mut to_remove = Vec::new();
        let mut result = None;

        for transfer in self.outgoing.values_mut() {
            if transfer.state == FileTransferState::Transferring {
                match transfer.read_next_chunk() {
                    Ok(Some(data)) => {
                        let offset = transfer.bytes_sent - data.len() as u64;
                        let transfer_id = transfer.id;

                        // Generate progress event
                        self.events.push(FileTransferEvent::Progress {
                            id: transfer_id,
                            bytes: transfer.bytes_sent,
                            total: transfer.total_size,
                        });

                        // Check if transfer just completed
                        if transfer.state == FileTransferState::Completed {
                            // Queue Complete message (will be sent next iteration)
                            self.send_queue.push(FileTransferMessage::Complete {
                                id: transfer_id,
                                checksum: 0,
                            });
                        }

                        // Create message bytes
                        let msg_bytes = FileTransferMessage::Data {
                            id: transfer_id,
                            offset,
                            data,
                        }
                        .to_bytes();

                        // Track buffered amount for flow control
                        self.buffered_amount += msg_bytes.len();

                        result = Some(msg_bytes);
                        break;
                    }
                    Ok(None) => {
                        // Transfer finished but no data to send - shouldn't happen normally
                        // as we queue Complete message when last chunk is sent
                    }
                    Err(e) => {
                        // File read error
                        let transfer_id = transfer.id;
                        self.events.push(FileTransferEvent::Failed {
                            id: transfer_id,
                            reason: format!("File read error: {}", e),
                        });
                        to_remove.push(transfer_id);
                    }
                }
            }
        }

        // Clean up failed transfers
        for id in to_remove {
            self.outgoing.remove(&id);
        }

        result
    }

    /// Poll for next event
    pub fn poll_event(&mut self) -> Option<FileTransferEvent> {
        if self.events.is_empty() {
            None
        } else {
            let event = self.events.remove(0);
            Some(event)
        }
    }

    /// Get transfer progress (bytes_transferred, total_bytes)
    pub fn get_progress(&self, id: u64) -> Option<(u64, u64)> {
        self.outgoing
            .get(&id)
            .map(|t| (t.bytes_sent, t.total_size))
            .or_else(|| {
                self.incoming
                    .get(&id)
                    .map(|t| (t.bytes_received, t.total_size))
            })
    }
}
