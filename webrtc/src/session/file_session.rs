//! File session - separate session for file transfers sharing DTLS transport
//!
//! FileSession provides file transfer capabilities over an existing DTLS connection,
//! running parallel to video streaming without interference.

use super::file_channel::FileChannel;
use super::file_transfer::FileTransferEvent;
use network::datachannel::{DataChannelEvent, DataChannelManager};
use network::sctp::{AssociationConfig, SctpAssociation, SctpPacket};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// File session for managing file transfers
///
/// Runs alongside the video session, sharing the same DTLS transport.
pub struct FileSession {
    /// Data channel manager over SCTP
    channel_manager: Arc<Mutex<DataChannelManager>>,
    /// File channel for high-level file transfer
    file_channel: Arc<Mutex<Option<FileChannel>>>,
    /// File transfer channel IDs (both local and remote)
    file_channel_ids: Arc<Mutex<Vec<u16>>>,
    /// Whether the session is established
    established: Arc<AtomicBool>,
}

impl FileSession {
    /// Create a new file session
    ///
    /// # Arguments
    /// * `is_client` - True if we initiated the connection (affects stream ID allocation)
    pub fn new(is_client: bool) -> Self {
        let config = AssociationConfig::default();
        let association = SctpAssociation::new(config);
        let channel_manager = DataChannelManager::new(association, is_client);

        Self {
            channel_manager: Arc::new(Mutex::new(channel_manager)),
            file_channel: Arc::new(Mutex::new(None)),
            file_channel_ids: Arc::new(Mutex::new(Vec::new())),
            established: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Check if the file session is ready
    ///
    /// This directly checks the DataChannelManager for any open file-transfer channel,
    /// bypassing the event queue which may have delays.
    pub fn is_established(&self) -> bool {
        // Directly check manager for ANY open file-transfer channel
        // This bypasses the event queue and checks the actual channel state
        if let Ok(manager) = self.channel_manager.lock()
            && manager.is_established()
                && let Some(open_channel_id) = manager.find_open_file_channel() {
                    // Found an open file-transfer channel!
                    // Make sure it's tracked in file_channel_ids
                    if let Ok(mut ids) = self.file_channel_ids.lock()
                        && !ids.contains(&open_channel_id) {
                            ids.push(open_channel_id);
                        }

                    // Set established flag
                    if !self.established.load(Ordering::SeqCst) {
                        self.established.store(true, Ordering::SeqCst);
                    }
                }

        self.established.load(Ordering::SeqCst)
    }

    /// Establish the SCTP association
    ///
    /// Call this after DTLS handshake is complete
    /// Returns the SCTP INIT packet to send
    pub fn establish(&mut self) -> Result<Vec<u8>, &'static str> {
        let mut manager = self.channel_manager.lock().map_err(|_| "Lock error")?;

        // Initialize SCTP association (sends INIT)
        let init_packet = manager.init_association()?;

        Ok(init_packet)
    }

    /// Create file channel after SCTP association is established
    fn create_file_channel(&mut self) -> Result<(), &'static str> {
        // Check if we already have a file channel
        {
            let fc = self.file_channel.lock().map_err(|_| "Lock error")?;
            if fc.is_some() {
                return Ok(()); // Already created
            }
        }

        let mut manager = self.channel_manager.lock().map_err(|_| "Lock error")?;

        let is_established = manager.is_established();

        if !is_established {
            return Err("SCTP association not established");
        }

        // Create file transfer channel
        let channel_id = manager.create_file_channel()?;

        // Track this channel ID
        if let Ok(mut ids) = self.file_channel_ids.lock() {
            ids.push(channel_id);
        }

        let mut fc = self.file_channel.lock().map_err(|_| "Lock error")?;
        *fc = Some(FileChannel::new(channel_id));

        Ok(())
    }

    /// Send a file
    ///
    /// If no channel is ready yet, this will retry for up to 5 seconds.
    pub fn send_file(&self, path: &Path) -> Result<u64, String> {
        // Wait for channel to become ready (DCEP handshake can take 1-3 seconds)
        let mut retries = 0;
        let max_retries = 100; // 100 * 50ms = 5 seconds max

        let open_channel_id = loop {
            // Find ANY open file-transfer channel directly from the manager
            let found = if let Ok(manager) = self.channel_manager.lock() {
                manager.find_open_file_channel()
            } else {
                None
            };

            if let Some(id) = found {
                break id;
            }

            retries += 1;
            if retries > max_retries {
                return Err(
                    "DataChannel not open yet. Please wait for 'Ready to transfer files' message."
                        .to_string(),
                );
            }

            std::thread::sleep(std::time::Duration::from_millis(50));
        };

        // Now we have an open channel, proceed with send
        let mut fc = self.file_channel.lock().map_err(|e| e.to_string())?;

        if fc.is_none() {
            // Create FileChannel if it doesn't exist yet
            *fc = Some(super::file_channel::FileChannel::new(open_channel_id));
        } else {
            // Update FileChannel to use the open channel ID if different
            let file_channel = fc.as_mut().unwrap();
            if file_channel.channel_id() != open_channel_id {
                *fc = Some(super::file_channel::FileChannel::new(open_channel_id));
            }
        }

        let file_channel = fc.as_mut().unwrap();
        file_channel.send_file(path).map_err(|e| e.to_string())
    }

    /// Accept an incoming file transfer
    pub fn accept_transfer(&self, id: u64, save_path: &Path) -> Result<(), String> {
        let mut fc = self.file_channel.lock().map_err(|e| e.to_string())?;
        let file_channel = fc.as_mut().ok_or("File channel not established")?;
        file_channel
            .accept_transfer(id, save_path)
            .map_err(|e| e.to_string())
    }

    /// Reject an incoming file transfer
    pub fn reject_transfer(&self, id: u64, reason: &str) -> Result<(), String> {
        let mut fc = self.file_channel.lock().map_err(|e| e.to_string())?;
        let file_channel = fc.as_mut().ok_or("File channel not established")?;
        file_channel
            .reject_transfer(id, reason)
            .map_err(|e| e.to_string())
    }

    /// Cancel an ongoing transfer
    pub fn cancel_transfer(&self, id: u64, reason: &str) -> Result<(), String> {
        let mut fc = self.file_channel.lock().map_err(|e| e.to_string())?;
        let file_channel = fc.as_mut().ok_or("File channel not established")?;
        file_channel
            .cancel_transfer(id, reason)
            .map_err(|e| e.to_string())
    }

    /// Notify that bytes were sent (for flow control)
    pub fn on_bytes_sent(&mut self, bytes: usize) {
        if let Ok(mut fc) = self.file_channel.lock()
            && let Some(channel) = fc.as_mut() {
                channel.on_bytes_sent(bytes);
            }
    }

    /// Process incoming SCTP data (from DTLS)
    /// Returns Vec of response packets to send
    pub fn on_sctp_data(&mut self, data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
        let packet = SctpPacket::from_bytes(data).map_err(|e| e.to_string())?;

        let mut manager = self.channel_manager.lock().map_err(|e| e.to_string())?;
        let responses = manager.process_packet(&packet);

        // Check if association just became established and we haven't created a channel yet
        let is_established = manager.is_established();
        let has_channel = {
            let fc = self.file_channel.lock().map_err(|e| e.to_string())?;
            fc.is_some()
        };

        let should_create = is_established && !has_channel;

        if should_create {
            drop(manager); // Release lock before calling create_file_channel
            // Association is now established, create the file channel
            let _ = self.create_file_channel().is_ok();
            manager = self.channel_manager.lock().map_err(|e| e.to_string())?;
        }

        // Process data channel events
        let events = manager.drain_events();

        // Get mutable access to file channel once for all events
        if let Ok(mut fc_lock) = self.file_channel.lock() {
            for event in events {
                if let DataChannelEvent::ChannelOpened { id, label } = &event {
                    // If this is a file transfer channel from the remote peer, track it
                    if label == "file-transfer" {
                        if let Ok(mut ids) = self.file_channel_ids.lock()
                            && !ids.contains(id) {
                                ids.push(*id);
                            }

                        // If we don't have a file channel yet, create one with remote's ID
                        if fc_lock.is_none() {
                            *fc_lock = Some(FileChannel::new(*id));
                        }

                        // NOW set established flag - channel is open!
                        if !self.established.load(Ordering::SeqCst) {
                            self.established.store(true, Ordering::SeqCst);
                        }
                    }
                }

                // Process data events for file channel
                if let Some(fc) = fc_lock.as_mut()
                    && let DataChannelEvent::DataReceived { id, data } = event {
                        // Check if this data is from a file transfer channel
                        let is_file_channel = if let Ok(ids) = self.file_channel_ids.lock() {
                            ids.contains(&id)
                        } else {
                            false
                        };

                        if is_file_channel {
                            fc.on_data(&data);
                        }
                    }
            }
        }

        // Serialize ALL response packets - each needs to be sent separately
        let response_bytes: Vec<Vec<u8>> = responses.iter().map(|r| r.to_bytes()).collect();

        Ok(response_bytes)
    }

    /// Poll for outgoing SCTP data to send
    pub fn poll_send(&self) -> Option<Vec<u8>> {
        // First check file channel for messages
        if let Ok(mut fc) = self.file_channel.lock()
            && let Some(file_channel) = fc.as_mut()
                && let Some(data) = file_channel.poll_send() {
                    // Send through data channel manager
                    if let Ok(mut manager) = self.channel_manager.lock() {
                        let channel_id = file_channel.channel_id();
                        let _ = manager.send(channel_id, &data);
                    }
                }

        // Then check SCTP association for packets
        if let Ok(mut manager) = self.channel_manager.lock()
            && let Some(packet) = manager.poll_send() {
                return Some(packet.to_bytes());
            }

        None
    }

    /// Poll for file transfer events
    pub fn poll_event(&self) -> Option<FileTransferEvent> {
        if let Ok(mut fc) = self.file_channel.lock()
            && let Some(file_channel) = fc.as_mut() {
                return file_channel.poll_event();
            }
        None
    }

    /// Get transfer progress (bytes_transferred, total_bytes)
    pub fn get_progress(&self, id: u64) -> Option<(u64, u64)> {
        if let Ok(fc) = self.file_channel.lock()
            && let Some(file_channel) = fc.as_ref() {
                return file_channel.get_progress(id);
            }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_session_creation() {
        let session = FileSession::new(true);
        assert!(!session.is_established());
    }
}
