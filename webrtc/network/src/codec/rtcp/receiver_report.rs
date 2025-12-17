//! RTCP Receiver Report implementation

use super::RtcpPacketType;
use super::sender_report::ReportBlock;
use super::stats::RtcpStats;
use std::time::SystemTime;

/// RTCP Receiver Report
#[derive(Debug, Clone)]
pub struct ReceiverReport {
    /// SSRC of receiver
    pub ssrc: u32,
    /// Receiver report blocks (one per source)
    pub report_blocks: Vec<ReportBlock>,
}

impl ReceiverReport {
    /// Create a new Receiver Report
    pub fn new(stats: &RtcpStats, remote_ssrc: u32) -> Self {
        let block = create_report_block(stats, remote_ssrc);

        Self {
            ssrc: stats.ssrc,
            report_blocks: vec![block],
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8 + self.report_blocks.len() * 24);

        super::write_rtcp_header(&mut bytes, RtcpPacketType::RR, self.report_blocks.len());
        bytes.extend_from_slice(&self.ssrc.to_be_bytes());

        for block in &self.report_blocks {
            bytes.extend_from_slice(&block.to_bytes());
        }

        bytes
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 8 {
            return Err("RR packet too short".to_string());
        }

        let rc = data[0] & 0x1F;
        let ssrc = crate::codec::rtp::parse_u32_be(data, 4);

        let report_blocks = parse_report_blocks(data, rc)?;

        Ok(Self {
            ssrc,
            report_blocks,
        })
    }
}

fn create_report_block(stats: &RtcpStats, remote_ssrc: u32) -> ReportBlock {
    ReportBlock {
        ssrc: remote_ssrc,
        fraction_lost: stats.calculate_fraction_lost(),
        cumulative_packets_lost: stats.packets_lost as i32,
        extended_highest_seq: (stats.seq_cycles << 16) | (stats.highest_seq_received as u32),
        jitter: stats.jitter as u32,
        last_sr: stats.last_sr_timestamp,
        delay_since_last_sr: calculate_delay(stats.last_sr_received_at),
    }
}

fn calculate_delay(last_sr: Option<SystemTime>) -> u32 {
    if let Some(last_sr_time) = last_sr {
        let delay = SystemTime::now()
            .duration_since(last_sr_time)
            .unwrap_or_default();
        (delay.as_secs_f64() * 65536.0) as u32
    } else {
        0
    }
}

fn parse_report_blocks(data: &[u8], rc: u8) -> Result<Vec<ReportBlock>, String> {
    let mut report_blocks = Vec::new();
    let mut offset = 8;

    for _ in 0..rc {
        if offset + 24 > data.len() {
            break;
        }
        if let Ok(block) = ReportBlock::from_bytes(&data[offset..offset + 24]) {
            report_blocks.push(block);
        }
        offset += 24;
    }

    Ok(report_blocks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receiver_report_serialization() {
        let mut stats = RtcpStats::new(12345);
        stats.packets_received = 95;
        stats.packets_lost = 5;
        stats.highest_seq_received = 100;

        let rr = ReceiverReport::new(&stats, 54321);
        let bytes = rr.to_bytes();

        assert_eq!(bytes[1], RtcpPacketType::RR as u8);
        assert!(bytes.len() >= 8);

        let parsed = ReceiverReport::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.ssrc, stats.ssrc);
    }
}
