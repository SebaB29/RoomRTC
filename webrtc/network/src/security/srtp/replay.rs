//! Replay attack protection for SRTP

use std::collections::HashSet;

/// Replay attack protection using sliding window
pub struct ReplayWindow {
    /// Highest sequence number received
    highest_seq: u64,
    /// Bitmap of recently received sequence numbers
    window: HashSet<u64>,
    window_size: u64,
}

impl ReplayWindow {
    pub fn new(window_size: u64) -> Self {
        Self {
            highest_seq: 0,
            window: HashSet::new(),
            window_size,
        }
    }

    /// Checks if packet is a replay, updates window if valid
    ///
    /// Returns true if packet is valid (not a replay)
    pub fn check_and_update(&mut self, seq: u64) -> bool {
        if self.window.is_empty() {
            return self.handle_first_packet(seq);
        }

        if self.is_too_old(seq) || self.is_duplicate(seq) {
            return false;
        }

        self.add_packet(seq);
        true
    }

    fn handle_first_packet(&mut self, seq: u64) -> bool {
        self.highest_seq = seq;
        self.window.insert(seq);
        true
    }

    fn is_too_old(&self, seq: u64) -> bool {
        seq + self.window_size < self.highest_seq
    }

    fn is_duplicate(&self, seq: u64) -> bool {
        self.window.contains(&seq)
    }

    fn add_packet(&mut self, seq: u64) {
        self.window.insert(seq);

        if seq > self.highest_seq {
            self.highest_seq = seq;
            self.clean_old_entries();
        }
    }

    fn clean_old_entries(&mut self) {
        self.window
            .retain(|&s| s + self.window_size >= self.highest_seq);
    }
}
