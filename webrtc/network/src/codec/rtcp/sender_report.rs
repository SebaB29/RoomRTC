//! RTCP Sender Report implementation

use super::RtcpPacketType;
use super::stats::RtcpStats;
use std::time::{SystemTime, UNIX_EPOCH};

/// Report block (used in SR and RR)
#[derive(Debug, Clone)]
pub struct ReportBlock {
    /// SSRC of source being reported on
    pub ssrc: u32,
    /// Fraction of packets lost (0-255, 255 = 100%)
    pub fraction_lost: u8,
    pub cumulative_packets_lost: i32,
    pub extended_highest_seq: u32,
    /// Interarrival jitter
    pub jitter: u32,
    /// Last SR timestamp from this source
    pub last_sr: u32,
    pub delay_since_last_sr: u32,
}

impl ReportBlock {
    pub(super) fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(24);

        bytes.extend_from_slice(&self.ssrc.to_be_bytes());
        bytes.push(self.fraction_lost);

        // Cumulative packets lost (24 bits)
        let lost_bytes = self.cumulative_packets_lost.to_be_bytes();
        bytes.extend_from_slice(&lost_bytes[1..4]);

        bytes.extend_from_slice(&self.extended_highest_seq.to_be_bytes());
        bytes.extend_from_slice(&self.jitter.to_be_bytes());
        bytes.extend_from_slice(&self.last_sr.to_be_bytes());
        bytes.extend_from_slice(&self.delay_since_last_sr.to_be_bytes());

        bytes
    }

    pub(super) fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 24 {
            return Err("Report block too short".to_string());
        }

        let ssrc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let fraction_lost = data[4];

        let lost_bytes = [0, data[5], data[6], data[7]];
        let cumulative_packets_lost = i32::from_be_bytes(lost_bytes);

        let extended_highest_seq = crate::codec::rtp::parse_u32_be(data, 8);
        let jitter = crate::codec::rtp::parse_u32_be(data, 12);
        let last_sr = crate::codec::rtp::parse_u32_be(data, 16);
        let delay_since_last_sr = crate::codec::rtp::parse_u32_be(data, 20);

        Ok(Self {
            ssrc,
            fraction_lost,
            cumulative_packets_lost,
            extended_highest_seq,
            jitter,
            last_sr,
            delay_since_last_sr,
        })
    }
}

/// RTCP Sender Report
#[derive(Debug, Clone)]
pub struct SenderReport {
    pub ssrc: u32,
    /// NTP timestamp (most significant 32 bits)
    pub ntp_timestamp_msw: u32,
    /// NTP timestamp (least significant 32 bits)
    pub ntp_timestamp_lsw: u32,
    /// RTP timestamp corresponding to NTP timestamp
    pub rtp_timestamp: u32,
    pub sender_packet_count: u32,
    pub sender_byte_count: u32,
    /// Optional receiver report blocks
    pub report_blocks: Vec<ReportBlock>,
}

impl SenderReport {
    /// Create a new Sender Report
    pub fn new(stats: &RtcpStats) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX_EPOCH");

        let ntp_secs = now.as_secs() + 2_208_988_800;

        Self {
            ssrc: stats.ssrc,
            ntp_timestamp_msw: (ntp_secs >> 32) as u32,
            ntp_timestamp_lsw: (ntp_secs & 0xFFFFFFFF) as u32,
            rtp_timestamp: stats.last_rtp_timestamp,
            sender_packet_count: stats.packets_sent,
            sender_byte_count: stats.bytes_sent as u32,
            report_blocks: Vec::new(),
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(28 + self.report_blocks.len() * 24);

        super::write_rtcp_header(&mut bytes, RtcpPacketType::SR, self.report_blocks.len());
        bytes.extend_from_slice(&self.ssrc.to_be_bytes());

        write_sender_info(&mut bytes, self);

        for block in &self.report_blocks {
            bytes.extend_from_slice(&block.to_bytes());
        }

        bytes
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 28 {
            return Err("SR packet too short".to_string());
        }

        let rc = data[0] & 0x1F;
        let ssrc = crate::codec::rtp::parse_u32_be(data, 4);
        let ntp_msw = crate::codec::rtp::parse_u32_be(data, 8);
        let ntp_lsw = crate::codec::rtp::parse_u32_be(data, 12);
        let rtp_timestamp = crate::codec::rtp::parse_u32_be(data, 16);
        let packet_count = crate::codec::rtp::parse_u32_be(data, 20);
        let byte_count = crate::codec::rtp::parse_u32_be(data, 24);

        let report_blocks = parse_report_blocks(data, rc, 28)?;

        Ok(Self {
            ssrc,
            ntp_timestamp_msw: ntp_msw,
            ntp_timestamp_lsw: ntp_lsw,
            rtp_timestamp,
            sender_packet_count: packet_count,
            sender_byte_count: byte_count,
            report_blocks,
        })
    }
}

fn write_sender_info(bytes: &mut Vec<u8>, sr: &SenderReport) {
    bytes.extend_from_slice(&sr.ntp_timestamp_msw.to_be_bytes());
    bytes.extend_from_slice(&sr.ntp_timestamp_lsw.to_be_bytes());
    bytes.extend_from_slice(&sr.rtp_timestamp.to_be_bytes());
    bytes.extend_from_slice(&sr.sender_packet_count.to_be_bytes());
    bytes.extend_from_slice(&sr.sender_byte_count.to_be_bytes());
}

fn parse_report_blocks(
    data: &[u8],
    rc: u8,
    start_offset: usize,
) -> Result<Vec<ReportBlock>, String> {
    let mut report_blocks = Vec::new();
    let mut offset = start_offset;

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
    fn test_sender_report_serialization() {
        let mut stats = RtcpStats::new(12345);
        stats.packets_sent = 100;
        stats.bytes_sent = 50000;
        stats.last_rtp_timestamp = 160000;

        let sr = SenderReport::new(&stats);
        let bytes = sr.to_bytes();

        assert_eq!(bytes[1], RtcpPacketType::SR as u8);
        assert!(bytes.len() >= 28);

        let parsed = SenderReport::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.ssrc, stats.ssrc);
        assert_eq!(parsed.sender_packet_count, stats.packets_sent);
    }
}
