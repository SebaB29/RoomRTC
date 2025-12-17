//! Packet Loss and Reordering Handler

use std::collections::{BTreeMap, HashSet};

/// Packet loss and reordering statistics
#[derive(Debug, Clone, Default)]
pub struct PacketStats {
    /// Total packets received
    pub packets_received: u64,
    /// Total packets lost (gaps in sequence)
    pub packets_lost: u64,
    /// Packets received out of order
    pub packets_reordered: u64,
    /// Duplicate packets received
    pub packets_duplicate: u64,
    /// Current loss rate (0.0 - 1.0)
    pub loss_rate: f64,
}

/// Packet handler for loss detection and reordering
pub struct PacketHandler {
    /// Expected next sequence number
    expected_seq: Option<u16>,
    /// Highest sequence number seen
    highest_seq: u16,
    /// Number of sequence number cycles (for wrap-around)
    seq_cycles: u32,
    /// Set of received sequence numbers (for duplicate detection)
    received_seqs: HashSet<u16>,
    /// Out-of-order buffer (sequence -> payload)
    reorder_buffer: BTreeMap<u16, Vec<u8>>,
    /// Maximum reorder buffer size
    max_reorder_buffer: usize,
    /// Statistics
    stats: PacketStats,
}

impl PacketHandler {
    /// Create a new packet handler
    pub fn new() -> Self {
        Self::with_capacity(100)
    }

    /// Create with custom reorder buffer capacity
    pub fn with_capacity(max_reorder_buffer: usize) -> Self {
        Self {
            expected_seq: None,
            highest_seq: 0,
            seq_cycles: 0,
            received_seqs: HashSet::new(),
            reorder_buffer: BTreeMap::new(),
            max_reorder_buffer,
            stats: PacketStats::default(),
        }
    }

    /// Process incoming packet
    /// Returns: (is_new, is_in_order, missing_count)
    pub fn process_packet(&mut self, seq: u16) -> (bool, bool, u64) {
        // Check for duplicate
        if self.received_seqs.contains(&seq) {
            self.stats.packets_duplicate += 1;
            return (false, false, 0);
        }

        // Initialize on first packet
        if self.expected_seq.is_none() {
            self.expected_seq = Some(seq.wrapping_add(1));
            self.highest_seq = seq;
            self.received_seqs.insert(seq);
            self.stats.packets_received += 1;
            return (true, true, 0);
        }

        let expected = self
            .expected_seq
            .expect("Expected sequence should be initialized after first packet");
        let mut is_in_order = false;
        let mut missing_count = 0u64;

        // Check if this is the expected packet
        if seq == expected {
            is_in_order = true;
            self.expected_seq = Some(seq.wrapping_add(1));

            // Update highest sequence if needed
            if self.is_newer(seq, self.highest_seq) {
                self.highest_seq = seq;
            }
        } else {
            // Out of order packet
            if self.is_newer(seq, expected) {
                // Future packet - we've detected loss
                missing_count = self.seq_distance(expected, seq);
                self.stats.packets_lost += missing_count;
                self.expected_seq = Some(seq.wrapping_add(1));

                if self.is_newer(seq, self.highest_seq) {
                    // Check for sequence wrap-around
                    if (seq as i32 - self.highest_seq as i32) < 0 {
                        self.seq_cycles += 1;
                    }
                    self.highest_seq = seq;
                }
            } else {
                // Old packet - reordered
                self.stats.packets_reordered += 1;
            }
        }

        self.received_seqs.insert(seq);
        self.stats.packets_received += 1;

        // Update loss rate
        let total = self.stats.packets_received + self.stats.packets_lost;
        if total > 0 {
            self.stats.loss_rate = self.stats.packets_lost as f64 / total as f64;
        }

        // Cleanup old sequence numbers (keep last 1000)
        if self.received_seqs.len() > 1000 {
            let old_seqs: Vec<u16> = self
                .received_seqs
                .iter()
                .take(self.received_seqs.len() - 1000)
                .copied()
                .collect();
            for old_seq in old_seqs {
                self.received_seqs.remove(&old_seq);
            }
        }

        (true, is_in_order, missing_count)
    }

    /// Add packet to reorder buffer
    pub fn buffer_packet(&mut self, seq: u16, payload: Vec<u8>) -> bool {
        if self.reorder_buffer.len() >= self.max_reorder_buffer {
            // Remove oldest packet
            if let Some(&oldest_seq) = self.reorder_buffer.keys().next() {
                self.reorder_buffer.remove(&oldest_seq);
            }
        }

        self.reorder_buffer.insert(seq, payload);
        true
    }

    /// Get packet from reorder buffer if available
    pub fn get_buffered_packet(&mut self, seq: u16) -> Option<Vec<u8>> {
        self.reorder_buffer.remove(&seq)
    }

    /// Get all packets in sequence starting from expected
    pub fn drain_in_order(&mut self) -> Vec<(u16, Vec<u8>)> {
        let mut result = Vec::new();

        if let Some(mut expected) = self.expected_seq {
            while let Some(payload) = self.reorder_buffer.remove(&expected) {
                result.push((expected, payload));
                expected = expected.wrapping_add(1);
                self.expected_seq = Some(expected);
            }
        }

        result
    }

    /// Get current statistics
    pub fn stats(&self) -> &PacketStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = PacketStats::default();
    }

    /// Clear all state
    pub fn clear(&mut self) {
        self.expected_seq = None;
        self.highest_seq = 0;
        self.seq_cycles = 0;
        self.received_seqs.clear();
        self.reorder_buffer.clear();
        self.stats = PacketStats::default();
    }

    /// Check if seq1 is newer than seq2 (handles wrap-around)
    fn is_newer(&self, seq1: u16, seq2: u16) -> bool {
        let diff = seq1.wrapping_sub(seq2) as i16;
        diff > 0
    }

    /// Calculate distance between two sequence numbers (handles wrap-around)
    fn seq_distance(&self, from: u16, to: u16) -> u64 {
        if to >= from {
            (to - from) as u64
        } else {
            // Wrap-around case
            (u16::MAX - from + to + 1) as u64
        }
    }

    /// Get extended sequence number (includes cycle count)
    pub fn extended_seq(&self, seq: u16) -> u64 {
        ((self.seq_cycles as u64) << 16) | seq as u64
    }
}

impl Default for PacketHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_in_order_packets() {
        let mut handler = PacketHandler::new();

        for i in 0..10u16 {
            let (is_new, is_in_order, missing) = handler.process_packet(i);
            assert!(is_new);
            assert!(is_in_order || i == 0); // First packet initializes
            assert_eq!(missing, 0);
        }

        assert_eq!(handler.stats().packets_received, 10);
        assert_eq!(handler.stats().packets_lost, 0);
    }

    #[test]
    fn test_packet_loss_detection() {
        let mut handler = PacketHandler::new();

        handler.process_packet(0);
        handler.process_packet(1);
        // Skip 2, 3, 4
        let (_, _, missing) = handler.process_packet(5);

        assert_eq!(missing, 3); // Detected 3 missing packets
        assert_eq!(handler.stats().packets_lost, 3);
    }

    #[test]
    fn test_duplicate_detection() {
        let mut handler = PacketHandler::new();

        handler.process_packet(0);
        let (is_new, _, _) = handler.process_packet(0); // Duplicate

        assert!(!is_new);
        assert_eq!(handler.stats().packets_duplicate, 1);
    }

    #[test]
    fn test_reordering_detection() {
        let mut handler = PacketHandler::new();

        handler.process_packet(0);
        handler.process_packet(2); // Out of order
        handler.process_packet(1); // Late arrival

        assert_eq!(handler.stats().packets_reordered, 1);
    }

    #[test]
    fn test_sequence_wraparound() {
        let mut handler = PacketHandler::new();

        handler.process_packet(65534);
        handler.process_packet(65535);
        let (is_new, is_in_order, missing) = handler.process_packet(0);

        assert!(is_new);
        assert!(is_in_order);
        assert_eq!(missing, 0);
    }

    #[test]
    fn test_reorder_buffer() {
        let mut handler = PacketHandler::new();

        handler.process_packet(0);
        handler.buffer_packet(2, vec![2, 2, 2]);
        handler.buffer_packet(3, vec![3, 3, 3]);
        handler.buffer_packet(1, vec![1, 1, 1]);

        handler.process_packet(1);
        let packets = handler.drain_in_order();

        assert_eq!(packets.len(), 2); // Should get 2 and 3
        assert_eq!(packets[0].0, 2);
        assert_eq!(packets[1].0, 3);
    }
}
