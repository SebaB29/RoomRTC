//! RTCP statistics tracking

use std::time::SystemTime;

/// Statistics tracker for RTCP
#[derive(Debug, Clone)]
pub struct RtcpStats {
    /// Our SSRC
    pub ssrc: u32,

    // Sender stats
    pub packets_sent: u32,
    pub bytes_sent: u64,
    pub last_rtp_timestamp: u32,

    // Receiver stats
    pub packets_received: u32,
    pub bytes_received: u64,
    pub packets_lost: u32,
    pub highest_seq_received: u16,
    pub seq_cycles: u32,
    pub jitter: f64,
    pub last_sr_timestamp: u32,
    pub last_sr_received_at: Option<SystemTime>,

    // Jitter calculation state
    pub(super) last_packet_arrival: Option<SystemTime>,
    pub(super) last_packet_timestamp: Option<u32>,

    // Timing
    pub last_sr_sent_at: Option<SystemTime>,
    pub last_rr_sent_at: Option<SystemTime>,
}

impl RtcpStats {
    /// Create new RTCP statistics tracker
    pub fn new(ssrc: u32) -> Self {
        Self {
            ssrc,
            packets_sent: 0,
            bytes_sent: 0,
            last_rtp_timestamp: 0,
            packets_received: 0,
            bytes_received: 0,
            packets_lost: 0,
            highest_seq_received: 0,
            seq_cycles: 0,
            jitter: 0.0,
            last_sr_timestamp: 0,
            last_sr_received_at: None,
            last_packet_arrival: None,
            last_packet_timestamp: None,
            last_sr_sent_at: None,
            last_rr_sent_at: None,
        }
    }

    /// Update sender statistics
    pub fn update_sender(&mut self, packet_size: usize, rtp_timestamp: u32) {
        self.packets_sent += 1;
        self.bytes_sent += packet_size as u64;
        self.last_rtp_timestamp = rtp_timestamp;
    }

    /// Update receiver statistics
    pub fn update_receiver(
        &mut self,
        packet_size: usize,
        seq_num: u16,
        timestamp: u32,
        arrival_time: SystemTime,
    ) {
        self.packets_received += 1;
        self.bytes_received += packet_size as u64;

        self.update_sequence_tracking(seq_num);
        self.update_jitter(timestamp, arrival_time);
    }

    fn update_sequence_tracking(&mut self, seq_num: u16) {
        let extended_seq = self.seq_cycles << 16 | seq_num as u32;
        let expected_seq = self.seq_cycles << 16 | self.highest_seq_received as u32;

        if seq_num > self.highest_seq_received {
            if (seq_num as i32 - self.highest_seq_received as i32) < 0 {
                self.seq_cycles += 1;
            }
            self.highest_seq_received = seq_num;
        }

        let expected = extended_seq.saturating_sub(expected_seq);
        if expected > 0 {
            self.packets_lost += expected.saturating_sub(1);
        }
    }

    fn update_jitter(&mut self, timestamp: u32, arrival_time: SystemTime) {
        if let (Some(last_arrival), Some(last_ts)) =
            (self.last_packet_arrival, self.last_packet_timestamp)
        {
            let arrival_diff = arrival_time
                .duration_since(last_arrival)
                .unwrap_or_default()
                .as_secs_f64()
                * 90000.0;

            let ts_diff = timestamp.wrapping_sub(last_ts) as f64;
            let d = (arrival_diff - ts_diff).abs();
            self.jitter += (d - self.jitter) / 16.0;
        }

        self.last_packet_arrival = Some(arrival_time);
        self.last_packet_timestamp = Some(timestamp);
    }

    /// Calculate fraction lost (0-255)
    pub fn calculate_fraction_lost(&self) -> u8 {
        if self.packets_received == 0 {
            return 0;
        }
        let expected = self.packets_received + self.packets_lost;
        let fraction = (self.packets_lost as f64 / expected as f64 * 256.0) as u32;
        fraction.min(255) as u8
    }

    /// Calculate RTT from SR/RR exchange
    pub fn calculate_rtt(&self) -> Option<f64> {
        let last_sr_sent = self.last_sr_sent_at?;
        let now = SystemTime::now();
        let rtt = now.duration_since(last_sr_sent).ok()?;
        Some(rtt.as_secs_f64())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fraction_lost_calculation() {
        let mut stats = RtcpStats::new(12345);
        stats.packets_received = 90;
        stats.packets_lost = 10;

        let fraction = stats.calculate_fraction_lost();
        assert!((24..=26).contains(&fraction));
    }
}
