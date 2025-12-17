//! SCTP association state machine
//!
//! An SCTP association represents a connection between two endpoints.
//! This implements the minimal state machine needed for WebRTC data channels.

use super::chunk::{DataChunk, InitChunk, SackChunk, SctpChunk};
use super::packet::SctpPacket;
use std::collections::{BTreeMap, VecDeque};
use std::time::Instant;

/// SCTP association states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationState {
    /// Initial state, no association
    Closed,
    /// INIT sent, waiting for INIT-ACK
    CookieWait,
    /// COOKIE-ECHO sent, waiting for COOKIE-ACK
    CookieEchoed,
    /// Association established
    Established,
    /// Shutdown initiated
    ShutdownPending,
    /// Shutdown sent
    ShutdownSent,
    /// Shutdown received
    ShutdownReceived,
    /// Shutdown acknowledged
    ShutdownAckSent,
}

/// Configuration for SCTP association
#[derive(Debug, Clone)]
pub struct AssociationConfig {
    /// Local SCTP port
    pub local_port: u16,
    /// Remote SCTP port
    pub remote_port: u16,
    /// Maximum number of outbound streams
    pub max_outbound_streams: u16,
    /// Maximum number of inbound streams
    pub max_inbound_streams: u16,
    /// Receiver window size
    pub recv_window: u32,
}

impl Default for AssociationConfig {
    fn default() -> Self {
        Self {
            local_port: 5000,
            remote_port: 5000,
            max_outbound_streams: 65535,
            max_inbound_streams: 65535,
            recv_window: 131072, // 128KB
        }
    }
}

/// Represents an SCTP association (connection)
#[derive(Debug)]
pub struct SctpAssociation {
    /// Current state
    state: AssociationState,
    /// Configuration
    config: AssociationConfig,
    /// Our verification tag (sent by peer in INIT-ACK)
    local_verification_tag: u32,
    /// Peer's verification tag (from INIT)
    peer_verification_tag: u32,
    /// Next TSN to send
    next_tsn: u32,
    /// Cumulative TSN acknowledged by peer
    cumulative_tsn_ack: u32,
    /// Expected next TSN from peer
    peer_last_tsn: u32,
    /// Number of outbound streams negotiated
    num_outbound_streams: u16,
    /// Number of inbound streams negotiated
    num_inbound_streams: u16,
    /// Outbound send queue
    send_queue: VecDeque<DataChunk>,
    /// Chunks waiting for acknowledgment
    in_flight: BTreeMap<u32, DataChunk>,
    /// Received chunks buffer (for reordering)
    receive_buffer: BTreeMap<u32, DataChunk>,
    /// Stream sequence numbers for outbound
    outbound_stream_seq: Vec<u16>,
    // /// Stream sequence numbers for inbound
    // //Webrtc uses this but i could make it work without what is the neccesity level? Maca remember to ask!
    // inbound_stream_seq: Vec<u16>,
    /// Cookie for handshake
    cookie: Vec<u8>,
    /// Last activity time
    last_activity: Instant,
}

impl SctpAssociation {
    /// Create new association
    pub fn new(config: AssociationConfig) -> Self {
        let local_verification_tag: u32 = rand::random();
        let initial_tsn: u32 = rand::random();

        Self {
            state: AssociationState::Closed,
            config,
            local_verification_tag,
            peer_verification_tag: 0,
            next_tsn: initial_tsn,
            cumulative_tsn_ack: initial_tsn.wrapping_sub(1),
            peer_last_tsn: 0,
            num_outbound_streams: 0,
            num_inbound_streams: 0,
            send_queue: VecDeque::new(),
            in_flight: BTreeMap::new(),
            receive_buffer: BTreeMap::new(),
            outbound_stream_seq: vec![0; 65536],
            // inbound_stream_seq: vec![0; 65536],
            cookie: Vec::new(),
            last_activity: Instant::now(),
        }
    }

    /// Get current state
    pub fn state(&self) -> AssociationState {
        self.state
    }

    /// Check if association is established
    pub fn is_established(&self) -> bool {
        self.state == AssociationState::Established
    }

    /// Create INIT packet to start association
    pub fn create_init(&mut self) -> SctpPacket {
        self.state = AssociationState::CookieWait;

        let init = InitChunk::new(self.local_verification_tag, self.next_tsn);

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            0, // Verification tag is 0 for INIT
        );
        packet.add_chunk(SctpChunk::Init(init));
        packet
    }

    /// Process received packet
    pub fn process_packet(&mut self, packet: &SctpPacket) -> Vec<SctpPacket> {
        self.last_activity = Instant::now();
        let mut responses = Vec::new();

        for chunk in &packet.chunks {
            if let Some(response) = self.process_chunk(chunk, packet.verification_tag) {
                responses.push(response);
            }
        }

        responses
    }

    /// Process a single chunk
    fn process_chunk(&mut self, chunk: &SctpChunk, _vtag: u32) -> Option<SctpPacket> {
        match chunk {
            SctpChunk::Init(init) => self.handle_init(init),
            SctpChunk::InitAck(init_ack) => self.handle_init_ack(init_ack),
            SctpChunk::CookieEcho(cookie) => self.handle_cookie_echo(cookie),
            SctpChunk::CookieAck => self.handle_cookie_ack(),
            SctpChunk::Data(data) => self.handle_data(data),
            SctpChunk::Sack(sack) => self.handle_sack(sack),
            SctpChunk::Shutdown { cumulative_tsn } => self.handle_shutdown(*cumulative_tsn),
            SctpChunk::ShutdownAck => self.handle_shutdown_ack(),
            _ => None,
        }
    }

    /// Handle INIT chunk (we are the server)
    fn handle_init(&mut self, init: &InitChunk) -> Option<SctpPacket> {
        self.peer_verification_tag = init.initiate_tag;
        self.peer_last_tsn = init.initial_tsn.wrapping_sub(1);
        self.num_outbound_streams = init
            .num_inbound_streams
            .min(self.config.max_outbound_streams);
        self.num_inbound_streams = init
            .num_outbound_streams
            .min(self.config.max_inbound_streams);

        // Generate cookie (simple: just random bytes)
        self.cookie = (0..32).map(|_| rand::random::<u8>()).collect();

        // Create INIT-ACK
        let init_ack = InitChunk {
            initiate_tag: self.local_verification_tag,
            a_rwnd: self.config.recv_window,
            num_outbound_streams: self.num_outbound_streams,
            num_inbound_streams: self.num_inbound_streams,
            initial_tsn: self.next_tsn,
        };

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );
        packet.add_chunk(SctpChunk::InitAck(init_ack));
        // Note: In real SCTP, cookie would be sent as a parameter within INIT-ACK
        // For simplicity, we send just INIT-ACK and the client will echo back our stored cookie

        Some(packet)
    }

    /// Handle INIT-ACK chunk (we are the client)
    fn handle_init_ack(&mut self, init_ack: &InitChunk) -> Option<SctpPacket> {
        if self.state != AssociationState::CookieWait {
            return None;
        }

        self.peer_verification_tag = init_ack.initiate_tag;
        self.peer_last_tsn = init_ack.initial_tsn.wrapping_sub(1);
        self.num_outbound_streams = init_ack
            .num_inbound_streams
            .min(self.config.max_outbound_streams);
        self.num_inbound_streams = init_ack
            .num_outbound_streams
            .min(self.config.max_inbound_streams);

        // Store cookie for COOKIE-ECHO
        // (In real implementation, cookie comes as parameter in INIT-ACK)
        self.cookie = (0..32).map(|_| rand::random::<u8>()).collect();
        self.state = AssociationState::CookieEchoed;

        // Send COOKIE-ECHO
        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );
        packet.add_chunk(SctpChunk::CookieEcho(self.cookie.clone()));

        Some(packet)
    }

    /// Handle COOKIE-ECHO chunk
    fn handle_cookie_echo(&mut self, _cookie: &[u8]) -> Option<SctpPacket> {
        // Verify cookie (simplified: accept any valid-looking cookie)
        self.state = AssociationState::Established;

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );
        packet.add_chunk(SctpChunk::CookieAck);

        Some(packet)
    }

    /// Handle COOKIE-ACK chunk
    fn handle_cookie_ack(&mut self) -> Option<SctpPacket> {
        if self.state == AssociationState::CookieEchoed {
            self.state = AssociationState::Established;
        }
        None
    }

    /// Handle DATA chunk
    fn handle_data(&mut self, data: &DataChunk) -> Option<SctpPacket> {
        if self.state != AssociationState::Established {
            return None;
        }

        // Skip duplicate/retransmitted packets that have already been delivered
        // A packet with TSN <= peer_last_tsn has already been cumulatively acknowledged
        let is_gt = tsn_gt(data.tsn, self.peer_last_tsn);
        let is_in_buffer = self.receive_buffer.contains_key(&data.tsn);

        if !is_gt && !is_in_buffer {
            // Return None - the normal SACK flow will handle acknowledgments
            return None;
        }

        // Store in receive buffer
        self.receive_buffer.insert(data.tsn, data.clone());

        // Update cumulative TSN
        while self
            .receive_buffer
            .contains_key(&self.peer_last_tsn.wrapping_add(1))
        {
            self.peer_last_tsn = self.peer_last_tsn.wrapping_add(1);
        }

        // Send SACK with gap blocks for out-of-order packets
        let mut sack = SackChunk::new(self.peer_last_tsn, self.config.recv_window);

        // Build gap ack blocks for packets received beyond cumulative TSN
        let mut gap_blocks = Vec::new();
        if !self.receive_buffer.is_empty() {
            let mut sorted_tsns: Vec<u32> = self.receive_buffer.keys().copied().collect();
            sorted_tsns.sort();

            let mut gap_start: Option<u32> = None;
            let mut gap_end: Option<u32> = None;

            for &tsn in &sorted_tsns {
                if tsn_gt(tsn, self.peer_last_tsn) {
                    // TSN is beyond cumulative, include in gap blocks
                    let offset = tsn.wrapping_sub(self.peer_last_tsn);

                    if let Some(end) = gap_end {
                        let prev_offset = end;
                        if offset == prev_offset + 1 {
                            // Continuous, extend current gap
                            gap_end = Some(offset);
                        } else {
                            // New gap, save previous and start new
                            if let Some(start) = gap_start {
                                gap_blocks.push((start as u16, end as u16));
                            }
                            gap_start = Some(offset);
                            gap_end = Some(offset);
                        }
                    } else {
                        // First gap
                        gap_start = Some(offset);
                        gap_end = Some(offset);
                    }
                }
            }

            // Add final gap block
            if let (Some(start), Some(end)) = (gap_start, gap_end) {
                gap_blocks.push((start as u16, end as u16));
            }
        }

        sack.gap_ack_blocks = gap_blocks;

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );
        packet.add_chunk(SctpChunk::Sack(sack));

        Some(packet)
    }

    /// Handle SACK chunk
    fn handle_sack(&mut self, sack: &SackChunk) -> Option<SctpPacket> {
        // Update cumulative TSN ack
        self.cumulative_tsn_ack = sack.cumulative_tsn;

        // Remove all chunks up to and including cumulative TSN
        self.in_flight
            .retain(|tsn, _| tsn_gt(*tsn, self.cumulative_tsn_ack));

        // Process gap blocks: mark selectively acknowledged chunks
        for (start_offset, end_offset) in &sack.gap_ack_blocks {
            let start_tsn = sack.cumulative_tsn.wrapping_add(*start_offset as u32);
            let end_tsn = sack.cumulative_tsn.wrapping_add(*end_offset as u32);

            // Remove acknowledged chunks in this gap
            self.in_flight
                .retain(|tsn, _| !(*tsn >= start_tsn && *tsn <= end_tsn));
        }

        // Retransmit missing packets (those still in in_flight after gap processing)
        if !self.in_flight.is_empty() {
            // Get the first missing chunk to retransmit
            if let Some((&_tsn, chunk)) = self.in_flight.iter().next() {
                let mut packet = SctpPacket::new(
                    self.config.local_port,
                    self.config.remote_port,
                    self.peer_verification_tag,
                );
                packet.add_chunk(SctpChunk::Data(chunk.clone()));
                return Some(packet);
            }
        }

        None
    }

    /// Handle SHUTDOWN chunk
    fn handle_shutdown(&mut self, _cumulative_tsn: u32) -> Option<SctpPacket> {
        self.state = AssociationState::ShutdownReceived;

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );
        packet.add_chunk(SctpChunk::ShutdownAck);

        Some(packet)
    }

    /// Handle SHUTDOWN-ACK chunk
    fn handle_shutdown_ack(&mut self) -> Option<SctpPacket> {
        self.state = AssociationState::Closed;

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );
        packet.add_chunk(SctpChunk::ShutdownComplete);

        Some(packet)
    }

    /// Queue data for sending on a stream
    pub fn send(&mut self, stream_id: u16, ppid: u32, data: Vec<u8>) -> Result<(), &'static str> {
        if self.state != AssociationState::Established {
            return Err("Association not established");
        }

        let stream_seq = self.outbound_stream_seq[stream_id as usize];
        self.outbound_stream_seq[stream_id as usize] = stream_seq.wrapping_add(1);

        let chunk = DataChunk::new(self.next_tsn, stream_id, stream_seq, ppid, data);
        self.next_tsn = self.next_tsn.wrapping_add(1);

        self.send_queue.push_back(chunk);
        Ok(())
    }

    /// Get next packet to send (if any)
    pub fn poll_send(&mut self) -> Option<SctpPacket> {
        if self.send_queue.is_empty() {
            return None;
        }

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );

        // Add chunks up to MTU
        // Simplified: only one chunk per packet for now
        if let Some(chunk) = self.send_queue.pop_front() {
            self.in_flight.insert(chunk.tsn, chunk.clone());
            packet.add_chunk(SctpChunk::Data(chunk));
        }

        Some(packet)
    }

    /// Receive data from a stream
    pub fn recv(&mut self) -> Option<(u16, u32, Vec<u8>)> {
        // Find all TSNs that are <= peer_last_tsn (in-order packets)
        // Return the lowest one to maintain order
        let in_order_tsns: Vec<u32> = self
            .receive_buffer
            .keys()
            .copied()
            .filter(|&tsn| !tsn_gt(tsn, self.peer_last_tsn))
            .collect();

        if let Some(&min_tsn) = in_order_tsns.iter().min()
            && let Some(chunk) = self.receive_buffer.remove(&min_tsn)
        {
            return Some((chunk.stream_id, chunk.ppid, chunk.data));
        }

        None
    }

    /// Initiate shutdown
    pub fn shutdown(&mut self) -> Option<SctpPacket> {
        if self.state != AssociationState::Established {
            return None;
        }

        self.state = AssociationState::ShutdownPending;

        let mut packet = SctpPacket::new(
            self.config.local_port,
            self.config.remote_port,
            self.peer_verification_tag,
        );
        packet.add_chunk(SctpChunk::Shutdown {
            cumulative_tsn: self.peer_last_tsn,
        });

        self.state = AssociationState::ShutdownSent;
        Some(packet)
    }
}

/// Compare TSNs accounting for wraparound (a > b in serial number arithmetic)
/// Per RFC 1982, a > b if (a - b) mod 2^32 is in range (0, 2^31)
fn tsn_gt(a: u32, b: u32) -> bool {
    // Handle wraparound: a is greater if (a - b) is a small positive number
    // In serial arithmetic, this means the difference should be in (0, 2^31)
    let diff = a.wrapping_sub(b);
    diff > 0 && diff < 0x8000_0000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_association_creation() {
        let config = AssociationConfig::default();
        let assoc = SctpAssociation::new(config);
        assert_eq!(assoc.state(), AssociationState::Closed);
    }

    #[test]
    fn test_init_creates_packet() {
        let config = AssociationConfig::default();
        let mut assoc = SctpAssociation::new(config);
        let packet = assoc.create_init();

        assert_eq!(packet.chunks.len(), 1);
        assert_eq!(assoc.state(), AssociationState::CookieWait);
    }

    #[test]
    fn test_tsn_comparison() {
        // Normal cases
        assert!(tsn_gt(2, 1)); // 2 > 1
        assert!(!tsn_gt(1, 2)); // 1 < 2
        assert!(!tsn_gt(1, 1)); // Equal

        // Wraparound cases
        assert!(tsn_gt(0, u32::MAX)); // 0 > MAX (wraparound: 0 just after MAX)
        assert!(!tsn_gt(u32::MAX, 0)); // MAX < 0 (wraparound)

        // Edge cases near half-range (per RFC 1982, exactly at 2^31 is undefined/ambiguous)
        // Our implementation treats the boundary as "not greater than"
        assert!(!tsn_gt(0x8000_0000, 0));
        assert!(!tsn_gt(0x8000_0001, 0)); // Past half-range
    }
}
