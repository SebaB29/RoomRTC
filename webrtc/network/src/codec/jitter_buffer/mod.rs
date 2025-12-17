//! Enhanced Jitter Buffer - Timestamp-based Playout

mod config;
mod stats;

pub use config::JitterBufferConfig;
pub use stats::JitterBufferStats;

use crate::codec::rtp::RtpPacket;
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Buffered packet with metadata
struct TimestampedPacket {
    packet: RtpPacket,
    playout_time: Instant,
}

/// Enhanced jitter buffer using timestamps for playout
pub struct JitterBuffer {
    config: JitterBufferConfig,
    buffer: BTreeMap<u16, TimestampedPacket>,
    next_sequence: Option<u16>,
    base_timestamp: Option<u32>,
    base_arrival: Option<Instant>,
    playout_delay_units: u32,
    jitter: f64,
    prev_arrival: Option<Instant>,
    prev_timestamp: Option<u32>,
    stats: JitterBufferStats,
    last_frame_duration: Option<u32>,
}

impl JitterBuffer {
    pub fn new() -> Self {
        Self::with_config(JitterBufferConfig::default())
    }

    pub fn with_config(config: JitterBufferConfig) -> Self {
        let initial_delay = config.min_delay_frames * (config.clock_rate / 30);

        Self {
            config,
            buffer: BTreeMap::new(),
            next_sequence: None,
            base_timestamp: None,
            base_arrival: None,
            playout_delay_units: initial_delay,
            jitter: 0.0,
            prev_arrival: None,
            prev_timestamp: None,
            stats: JitterBufferStats::default(),
            last_frame_duration: None,
        }
    }

    pub fn push(&mut self, packet: RtpPacket) {
        let arrival_time = Instant::now();
        let timestamp = packet.header.timestamp;
        let sequence = packet.header.sequence_number;

        self.initialize_base_if_needed(timestamp, sequence, arrival_time);

        if self.is_duplicate(sequence) {
            return;
        }

        self.update_jitter_estimate(arrival_time, timestamp);

        if self.is_too_late(arrival_time, timestamp) {
            return;
        }

        self.add_to_buffer(packet, sequence, timestamp);
        self.detect_frame_rate_if_needed();
        self.adapt_playout_delay();
    }

    pub fn pop(&mut self) -> Option<RtpPacket> {
        if self.config.ultra_low_latency {
            self.pop_ultra_low_latency()
        } else {
            self.pop_normal_mode()
        }
    }

    pub fn peek(&self) -> Option<&RtpPacket> {
        let next_seq = self.next_sequence?;
        self.buffer.get(&next_seq).map(|b| &b.packet)
    }

    pub fn is_ready(&self) -> bool {
        if let Some(next_seq) = self.next_sequence
            && let Some(buffered) = self.buffer.get(&next_seq)
        {
            return Instant::now() >= buffered.playout_time;
        }
        false
    }

    pub fn stats(&self) -> &JitterBufferStats {
        &self.stats
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        self.next_sequence = None;
        self.base_timestamp = None;
        self.base_arrival = None;
        self.prev_arrival = None;
        self.prev_timestamp = None;
        self.stats.buffer_size = 0;
    }

    fn initialize_base_if_needed(&mut self, timestamp: u32, sequence: u16, arrival_time: Instant) {
        if self.base_timestamp.is_none() {
            self.base_timestamp = Some(timestamp);
            self.base_arrival = Some(arrival_time);
            self.next_sequence = Some(sequence);
        }
    }

    fn is_duplicate(&mut self, sequence: u16) -> bool {
        if self.buffer.contains_key(&sequence) {
            self.stats.packets_duplicate += 1;
            true
        } else {
            false
        }
    }

    fn update_jitter_estimate(&mut self, arrival_time: Instant, timestamp: u32) {
        if let (Some(prev_arrival), Some(prev_ts)) = (self.prev_arrival, self.prev_timestamp) {
            let arrival_delta = arrival_time.duration_since(prev_arrival).as_secs_f64();
            let timestamp_delta =
                timestamp.wrapping_sub(prev_ts) as f64 / self.config.clock_rate as f64;
            let d = (arrival_delta - timestamp_delta).abs();

            self.jitter += (d - self.jitter) / 16.0;
            self.stats.jitter_ms = self.jitter * 1000.0;
        }

        self.prev_arrival = Some(arrival_time);
        self.prev_timestamp = Some(timestamp);
    }

    fn is_too_late(&mut self, arrival_time: Instant, timestamp: u32) -> bool {
        if self.config.ultra_low_latency {
            return false;
        }

        let playout_time = self.calculate_playout_time(timestamp);
        if arrival_time > playout_time {
            self.stats.packets_late += 1;
            true
        } else {
            false
        }
    }

    fn calculate_playout_time(&self, timestamp: u32) -> Instant {
        let base_arrival = self.base_arrival.expect("Base arrival initialized");
        let base_ts = self.base_timestamp.expect("Base timestamp initialized");
        let ts_delta = timestamp.wrapping_sub(base_ts) as u64;
        let playout_delay_ms =
            (self.playout_delay_units as u64 * 1000) / self.config.clock_rate as u64;

        base_arrival
            + Duration::from_millis(
                playout_delay_ms + (ts_delta * 1000) / self.config.clock_rate as u64,
            )
    }

    fn add_to_buffer(&mut self, packet: RtpPacket, sequence: u16, timestamp: u32) {
        if self.buffer.len() >= self.config.max_capacity
            && let Some(&oldest_ts) = self.buffer.keys().next()
        {
            self.buffer.remove(&oldest_ts);
        }

        let playout_time = self.calculate_playout_time(timestamp);

        self.buffer.insert(
            sequence,
            TimestampedPacket {
                packet,
                playout_time,
            },
        );

        self.stats.buffer_size = self.buffer.len();
    }

    fn detect_frame_rate_if_needed(&mut self) {
        if self.buffer.len() == 3 {
            let actual_frame_duration = self.estimate_frame_duration();
            self.playout_delay_units = self.config.min_delay_frames * actual_frame_duration;
        }
    }

    fn pop_ultra_low_latency(&mut self) -> Option<RtpPacket> {
        if let Some(next_seq) = self.next_sequence
            && let Some(buffered) = self.buffer.remove(&next_seq)
        {
            return Some(self.consume_packet(buffered.packet, next_seq.wrapping_add(1)));
        }

        self.pop_first_available_packet()
    }

    fn pop_first_available_packet(&mut self) -> Option<RtpPacket> {
        let (&first_seq, _) = self.buffer.iter().next()?;
        let buffered = self.buffer.remove(&first_seq)?;

        if let Some(expected) = self.next_sequence {
            let skipped = first_seq.wrapping_sub(expected) as u64;
            if skipped > 0 && skipped < 1000 {
                self.stats.underruns += skipped;
            }
        }

        Some(self.consume_packet(buffered.packet, first_seq.wrapping_add(1)))
    }

    fn consume_packet(&mut self, packet: RtpPacket, next_seq: u16) -> RtpPacket {
        self.next_sequence = Some(next_seq);
        self.stats.packets_played += 1;
        self.stats.buffer_size = self.buffer.len();
        packet
    }

    fn pop_normal_mode(&mut self) -> Option<RtpPacket> {
        let next_seq = self.next_sequence?;

        if self.is_packet_ready(next_seq) {
            let packet = self.buffer.remove(&next_seq).expect("Packet exists").packet;
            return Some(self.consume_packet(packet, next_seq.wrapping_add(1)));
        }

        if !self.buffer.contains_key(&next_seq) {
            self.handle_missing_packet(next_seq);
        }

        None
    }

    fn is_packet_ready(&self, sequence: u16) -> bool {
        if let Some(buffered) = self.buffer.get(&sequence) {
            Instant::now() >= buffered.playout_time
        } else {
            false
        }
    }

    fn handle_missing_packet(&mut self, sequence: u16) {
        self.stats.underruns += 1;
        self.next_sequence = Some(sequence.wrapping_add(1));
    }

    fn adapt_playout_delay(&mut self) {
        let jitter_units = (self.jitter * self.config.clock_rate as f64) as u32;
        let target_delay =
            self.config.min_delay_frames * (self.config.clock_rate / 30) + jitter_units * 2;

        let max_delay = self.config.max_delay_frames * (self.config.clock_rate / 30);
        let min_delay = self.config.min_delay_frames * (self.config.clock_rate / 30);
        let target_delay = target_delay.clamp(min_delay, max_delay);

        let adjustment = ((target_delay as i64 - self.playout_delay_units as i64) as f64
            * self.config.adaptation_speed) as i32;

        self.playout_delay_units =
            (self.playout_delay_units as i32 + adjustment).max(min_delay as i32) as u32;

        self.stats.playout_delay_ms =
            (self.playout_delay_units as f64 * 1000.0) / self.config.clock_rate as f64;
    }

    fn estimate_frame_duration(&mut self) -> u32 {
        if self.buffer.len() >= 2 {
            let packets: Vec<&TimestampedPacket> = self.buffer.values().take(2).collect();
            if packets.len() >= 2 {
                let duration = packets[1]
                    .packet
                    .header
                    .timestamp
                    .wrapping_sub(packets[0].packet.header.timestamp);
                if duration > 0 && duration < self.config.clock_rate {
                    self.last_frame_duration = Some(duration);
                    return duration;
                }
            }
        }

        self.last_frame_duration
            .unwrap_or(self.config.clock_rate / 30)
    }
}

impl Default for JitterBuffer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::codec::rtp::RtpHeader;

    fn create_test_packet(timestamp: u32, seq: u16) -> RtpPacket {
        let mut header = RtpHeader::new(96, 12345);
        header.sequence_number = seq;
        header.timestamp = timestamp;

        RtpPacket {
            header,
            payload: vec![1, 2, 3, 4],
        }
    }

    #[test]
    fn test_in_order_packets() {
        let mut jb = JitterBuffer::new();

        jb.push(create_test_packet(1000, 1));
        jb.push(create_test_packet(4000, 2));
        jb.push(create_test_packet(7000, 3));

        assert_eq!(jb.buffer.len(), 3);
        assert_eq!(jb.stats().packets_played, 0);
    }

    #[test]
    fn test_duplicate_detection() {
        let mut jb = JitterBuffer::new();

        jb.push(create_test_packet(1000, 1));
        jb.push(create_test_packet(1000, 1));

        assert_eq!(jb.buffer.len(), 1);
        assert_eq!(jb.stats().packets_duplicate, 1);
    }
}
