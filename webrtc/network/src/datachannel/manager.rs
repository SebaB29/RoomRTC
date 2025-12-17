//! Data channel manager
//!
//! Manages multiple data channels over a single SCTP association.

use super::channel::{DataChannel, DataChannelConfig};
use crate::sctp::{
    ChannelType, DataChannelAck, DataChannelOpen, SctpAssociation, SctpPacket, ppid,
};
use std::collections::HashMap;

/// Events emitted by the data channel manager
#[derive(Debug, Clone)]
pub enum DataChannelEvent {
    /// A new channel was opened by the remote peer
    ChannelOpened { id: u16, label: String },
    /// A channel was closed
    ChannelClosed { id: u16 },
    /// Data received on a channel
    DataReceived { id: u16, data: Vec<u8> },
    /// Error occurred
    Error { message: String },
}

/// Manages multiple data channels
#[derive(Debug)]
pub struct DataChannelManager {
    /// SCTP association
    association: SctpAssociation,
    /// Active data channels (by stream ID)
    channels: HashMap<u16, DataChannel>,
    /// Next stream ID to allocate (for initiator, use even; for responder, use odd)
    next_stream_id: u16,
    // /// Whether we are the DTLS client -> If DTLS wrapping is necessary
    // is_client: bool,
    /// Pending events
    events: Vec<DataChannelEvent>,
}

impl DataChannelManager {
    /// Create a new data channel manager
    pub fn new(association: SctpAssociation, is_client: bool) -> Self {
        // Client uses even stream IDs, server uses odd
        let next_stream_id = if is_client { 0 } else { 1 };

        Self {
            association,
            channels: HashMap::new(),
            next_stream_id,
            events: Vec::new(),
        }
    }

    /// Check if the association is established
    pub fn is_established(&self) -> bool {
        self.association.is_established()
    }

    /// Initialize SCTP association (send INIT packet)
    ///
    /// Call this after DTLS handshake to start SCTP association
    /// Returns the INIT packet bytes to send
    pub fn init_association(&mut self) -> Result<Vec<u8>, &'static str> {
        let init_packet = self.association.create_init();
        Ok(init_packet.to_bytes())
    }

    /// Create a new data channel
    pub fn create_channel(&mut self, config: DataChannelConfig) -> Result<u16, &'static str> {
        if !self.association.is_established() {
            return Err("SCTP association not established");
        }

        let stream_id = self.allocate_stream_id();
        let channel = DataChannel::new(stream_id, config.clone());

        // Send DATA_CHANNEL_OPEN message
        let open_msg = DataChannelOpen {
            channel_type: if config.ordered {
                ChannelType::Reliable
            } else {
                ChannelType::ReliableUnordered
            },
            priority: 0,
            reliability_param: 0,
            label: config.label,
            protocol: config.protocol,
        };

        self.association
            .send(stream_id, ppid::DCEP, open_msg.to_bytes())?;

        self.channels.insert(stream_id, channel);
        Ok(stream_id)
    }

    /// Create a file transfer channel
    pub fn create_file_channel(&mut self) -> Result<u16, &'static str> {
        self.create_channel(DataChannelConfig::file_transfer())
    }

    /// Send data on a channel
    pub fn send(&mut self, channel_id: u16, data: &[u8]) -> Result<(), &'static str> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or("Channel not found")?;

        if !channel.is_open() {
            return Err("Channel not open");
        }

        // Determine PPID (binary for file data)
        let ppid_value = if data.is_empty() {
            ppid::BINARY_EMPTY
        } else {
            ppid::BINARY
        };

        self.association
            .send(channel_id, ppid_value, data.to_vec())?;
        Ok(())
    }

    /// Send string data on a channel
    pub fn send_string(&mut self, channel_id: u16, data: &str) -> Result<(), &'static str> {
        let channel = self
            .channels
            .get_mut(&channel_id)
            .ok_or("Channel not found")?;

        if !channel.is_open() {
            return Err("Channel not open");
        }

        let ppid_value = if data.is_empty() {
            ppid::STRING_EMPTY
        } else {
            ppid::STRING
        };

        self.association
            .send(channel_id, ppid_value, data.as_bytes().to_vec())?;
        Ok(())
    }

    /// Process incoming SCTP packet
    pub fn process_packet(&mut self, packet: &SctpPacket) -> Vec<SctpPacket> {
        let responses = self.association.process_packet(packet);

        // Process any received data
        while let Some((stream_id, ppid_value, data)) = self.association.recv() {
            self.handle_received_data(stream_id, ppid_value, data);
        }

        responses
    }

    /// Handle received data
    fn handle_received_data(&mut self, stream_id: u16, ppid_value: u32, data: Vec<u8>) {
        match ppid_value {
            ppid::DCEP => self.handle_dcep_message(stream_id, &data),
            ppid::STRING | ppid::STRING_EMPTY | ppid::BINARY | ppid::BINARY_EMPTY => {
                self.handle_user_data(stream_id, data);
            }
            _ => {
                self.events.push(DataChannelEvent::Error {
                    message: format!("Unknown PPID: {}", ppid_value),
                });
            }
        }
    }

    /// Handle DCEP message
    fn handle_dcep_message(&mut self, stream_id: u16, data: &[u8]) {
        if data.is_empty() {
            return;
        }

        match data[0] {
            0x03 => {
                // DATA_CHANNEL_OPEN
                if let Ok(open) = DataChannelOpen::from_bytes(data) {
                    self.handle_channel_open(stream_id, open);
                }
            }
            0x02 => {
                // DATA_CHANNEL_ACK
                self.handle_channel_ack(stream_id);
            }
            _ => {}
        }
    }

    /// Handle DATA_CHANNEL_OPEN from remote
    fn handle_channel_open(&mut self, stream_id: u16, open: DataChannelOpen) {
        // Create channel for the incoming request
        let config = DataChannelConfig {
            label: open.label.clone(),
            ordered: open.channel_type.is_ordered(),
            max_retransmits: None,
            max_packet_lifetime: None,
            negotiated: false,
            id: Some(stream_id),
            protocol: open.protocol,
        };

        let mut channel = DataChannel::new(stream_id, config);
        channel.on_open(); // Immediately open for incoming channels

        self.channels.insert(stream_id, channel);

        // Send ACK
        let ack = DataChannelAck;
        let _ = self.association.send(stream_id, ppid::DCEP, ack.to_bytes());

        self.events.push(DataChannelEvent::ChannelOpened {
            id: stream_id,
            label: open.label,
        });
    }

    /// Handle DATA_CHANNEL_ACK from remote
    fn handle_channel_ack(&mut self, stream_id: u16) {
        if let Some(channel) = self.channels.get_mut(&stream_id) {
            channel.on_open();

            // Emit ChannelOpened event for locally-initiated channels
            self.events.push(DataChannelEvent::ChannelOpened {
                id: stream_id,
                label: channel.label().to_string(),
            });
        }
    }

    /// Handle user data
    fn handle_user_data(&mut self, stream_id: u16, data: Vec<u8>) {
        if let Some(channel) = self.channels.get_mut(&stream_id) {
            channel.on_data(data.clone());
            self.events.push(DataChannelEvent::DataReceived {
                id: stream_id,
                data,
            });
        }
    }

    /// Poll for events
    pub fn poll_event(&mut self) -> Option<DataChannelEvent> {
        if self.events.is_empty() {
            None
        } else {
            Some(self.events.remove(0))
        }
    }

    /// Get all pending events
    pub fn drain_events(&mut self) -> Vec<DataChannelEvent> {
        std::mem::take(&mut self.events)
    }

    /// Get channel by ID
    pub fn get_channel(&self, id: u16) -> Option<&DataChannel> {
        self.channels.get(&id)
    }

    /// Get mutable channel by ID
    pub fn get_channel_mut(&mut self, id: u16) -> Option<&mut DataChannel> {
        self.channels.get_mut(&id)
    }

    /// Get packet to send (if any)
    pub fn poll_send(&mut self) -> Option<SctpPacket> {
        self.association.poll_send()
    }

    /// Find any open file-transfer channel
    ///
    /// This searches ALL channels in the manager, not just tracked IDs,
    /// to handle the case where the channel is open but the event hasn't been processed yet.
    pub fn find_open_file_channel(&self) -> Option<u16> {
        for (&id, channel) in self.channels.iter() {
            if channel.is_open() && channel.label() == "file-transfer" {
                return Some(id);
            }
        }
        None
    }

    /// Close a channel
    pub fn close_channel(&mut self, channel_id: u16) -> Result<(), &'static str> {
        if let Some(channel) = self.channels.get_mut(&channel_id) {
            channel.close();
            // Note: SCTP doesn't have per-stream close, just stop using it
            Ok(())
        } else {
            Err("Channel not found")
        }
    }

    /// Allocate next stream ID
    fn allocate_stream_id(&mut self) -> u16 {
        let id = self.next_stream_id;
        self.next_stream_id += 2; // Skip by 2 (even/odd separation)
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sctp::AssociationConfig;

    #[test]
    fn test_manager_creation() {
        let config = AssociationConfig::default();
        let assoc = SctpAssociation::new(config);
        let manager = DataChannelManager::new(assoc, true);

        assert!(!manager.is_established());
    }

    #[test]
    fn test_stream_id_allocation() {
        let config = AssociationConfig::default();
        let assoc = SctpAssociation::new(config);
        let mut manager = DataChannelManager::new(assoc, true);

        // Client starts with even IDs
        assert_eq!(manager.allocate_stream_id(), 0);
        assert_eq!(manager.allocate_stream_id(), 2);
        assert_eq!(manager.allocate_stream_id(), 4);
    }

    #[test]
    fn test_server_stream_ids() {
        let config = AssociationConfig::default();
        let assoc = SctpAssociation::new(config);
        let mut manager = DataChannelManager::new(assoc, false);

        // Server starts with odd IDs
        assert_eq!(manager.allocate_stream_id(), 1);
        assert_eq!(manager.allocate_stream_id(), 3);
    }
}
