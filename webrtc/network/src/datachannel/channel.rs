//! Individual data channel implementation
//!
//! A data channel represents a bidirectional data stream within an SCTP association.

use std::collections::VecDeque;

/// Data channel state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataChannelState {
    /// Channel is being opened (waiting for ACK)
    Connecting,
    /// Channel is open and ready for data
    Open,
    /// Channel is closing
    Closing,
    /// Channel is closed
    Closed,
}

/// Configuration for a data channel
#[derive(Debug, Clone)]
pub struct DataChannelConfig {
    /// Human-readable label for the channel
    pub label: String,
    /// Whether messages are delivered in order
    pub ordered: bool,
    /// Maximum number of retransmissions (None = reliable)
    pub max_retransmits: Option<u16>,
    /// Maximum lifetime in milliseconds (None = reliable)
    pub max_packet_lifetime: Option<u16>,
    /// Negotiated channel (both sides agree on ID)
    pub negotiated: bool,
    /// Channel ID (if negotiated)
    pub id: Option<u16>,
    /// Sub-protocol
    pub protocol: String,
}

impl Default for DataChannelConfig {
    fn default() -> Self {
        Self {
            label: String::new(),
            ordered: true,
            max_retransmits: None,
            max_packet_lifetime: None,
            negotiated: false,
            id: None,
            protocol: String::new(),
        }
    }
}

impl DataChannelConfig {
    /// Create configuration for a reliable ordered channel
    pub fn reliable(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            ordered: true,
            max_retransmits: None,
            max_packet_lifetime: None,
            negotiated: false,
            id: None,
            protocol: String::new(),
        }
    }

    /// Create configuration for a file transfer channel
    pub fn file_transfer() -> Self {
        Self::reliable("file-transfer")
    }
}

/// Represents a single data channel
#[derive(Debug)]
pub struct DataChannel {
    /// Channel ID (SCTP stream ID)
    id: u16,
    /// Configuration
    config: DataChannelConfig,
    /// Current state
    state: DataChannelState,
    /// Buffered outgoing messages
    send_buffer: VecDeque<Vec<u8>>,
    /// Buffered incoming messages
    recv_buffer: VecDeque<Vec<u8>>,
    /// Bytes sent
    bytes_sent: u64,
    /// Bytes received
    bytes_received: u64,
}

impl DataChannel {
    /// Create a new data channel
    pub fn new(id: u16, config: DataChannelConfig) -> Self {
        Self {
            id,
            config,
            state: DataChannelState::Connecting,
            send_buffer: VecDeque::new(),
            recv_buffer: VecDeque::new(),
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Get channel ID
    pub fn id(&self) -> u16 {
        self.id
    }

    /// Get channel label
    pub fn label(&self) -> &str {
        &self.config.label
    }

    /// Get current state
    pub fn state(&self) -> DataChannelState {
        self.state
    }

    /// Check if channel is open
    pub fn is_open(&self) -> bool {
        self.state == DataChannelState::Open
    }

    /// Check if channel is ordered
    pub fn is_ordered(&self) -> bool {
        self.config.ordered
    }

    /// Get bytes sent
    pub fn bytes_sent(&self) -> u64 {
        self.bytes_sent
    }

    /// Get bytes received
    pub fn bytes_received(&self) -> u64 {
        self.bytes_received
    }

    /// Queue data for sending
    pub fn send(&mut self, data: Vec<u8>) -> Result<(), &'static str> {
        if self.state != DataChannelState::Open {
            return Err("Channel not open");
        }

        self.bytes_sent += data.len() as u64;
        self.send_buffer.push_back(data);
        Ok(())
    }

    /// Get next message to send
    pub fn poll_send(&mut self) -> Option<Vec<u8>> {
        self.send_buffer.pop_front()
    }

    /// Receive a message
    pub fn recv(&mut self) -> Option<Vec<u8>> {
        self.recv_buffer.pop_front()
    }

    /// Check if there's data to receive
    pub fn has_data(&self) -> bool {
        !self.recv_buffer.is_empty()
    }

    /// Called when ACK is received
    pub(crate) fn on_open(&mut self) {
        self.state = DataChannelState::Open;
    }

    /// Called when data is received
    pub(crate) fn on_data(&mut self, data: Vec<u8>) {
        self.bytes_received += data.len() as u64;
        self.recv_buffer.push_back(data);
    }

    /// Close the channel
    pub fn close(&mut self) {
        match self.state {
            DataChannelState::Open | DataChannelState::Connecting => {
                self.state = DataChannelState::Closing;
            }
            _ => {}
        }
    }

    // /// Mark channel as fully closed
    // TODO: I should close the channel when the SCTP association is closed at the very list when dropping the connection
    // pub(crate) fn on_close(&mut self) {
    //     self.state = DataChannelState::Closed;
    // }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let config = DataChannelConfig::reliable("test");
        let channel = DataChannel::new(0, config);

        assert_eq!(channel.id(), 0);
        assert_eq!(channel.label(), "test");
        assert_eq!(channel.state(), DataChannelState::Connecting);
    }

    #[test]
    fn test_channel_send_recv() {
        let config = DataChannelConfig::reliable("test");
        let mut channel = DataChannel::new(0, config);

        // Can't send when not open
        assert!(channel.send(vec![1, 2, 3]).is_err());

        // Open the channel
        channel.on_open();
        assert!(channel.is_open());

        // Now we can send
        assert!(channel.send(vec![1, 2, 3]).is_ok());
        assert_eq!(channel.bytes_sent(), 3);

        // Receive data
        channel.on_data(vec![4, 5, 6]);
        assert_eq!(channel.bytes_received(), 3);
        assert_eq!(channel.recv(), Some(vec![4, 5, 6]));
    }

    #[test]
    fn test_file_transfer_config() {
        let config = DataChannelConfig::file_transfer();
        assert_eq!(config.label, "file-transfer");
        assert!(config.ordered);
    }
}
