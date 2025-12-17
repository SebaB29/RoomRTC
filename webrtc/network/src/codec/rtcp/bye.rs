//! RTCP BYE packet implementation

use super::RtcpPacketType;

/// RTCP BYE packet
#[derive(Debug, Clone)]
pub struct ByePacket {
    /// SSRC(s) leaving
    pub ssrcs: Vec<u32>,
    /// Optional reason for leaving
    pub reason: Option<String>,
}

impl ByePacket {
    /// Create a new BYE packet
    pub fn new(ssrc: u32, reason: Option<String>) -> Self {
        Self {
            ssrcs: vec![ssrc],
            reason,
        }
    }

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let reason_len = self.reason.as_ref().map(|r| r.len()).unwrap_or(0);
        let mut bytes = Vec::with_capacity(4 + self.ssrcs.len() * 4 + 1 + reason_len);

        write_header(&mut bytes, self.ssrcs.len(), reason_len);
        write_ssrcs(&mut bytes, &self.ssrcs);
        write_reason(&mut bytes, &self.reason);

        bytes
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        if data.len() < 4 {
            return Err("BYE packet too short".to_string());
        }

        let sc = data[0] & 0x1F;
        let (ssrcs, offset) = parse_ssrcs(data, sc)?;
        let reason = parse_reason(data, offset);

        Ok(Self { ssrcs, reason })
    }
}

fn write_header(bytes: &mut Vec<u8>, ssrc_count: usize, reason_len: usize) {
    let version = 2u8;
    let padding = 0u8;
    let sc = ssrc_count as u8;
    bytes.push((version << 6) | (padding << 5) | sc);
    bytes.push(RtcpPacketType::BYE as u8);

    let words = ssrc_count
        + if reason_len > 0 {
            (1 + reason_len).div_ceil(4)
        } else {
            0
        };
    let length = words as u16;
    bytes.extend_from_slice(&length.to_be_bytes());
}

fn write_ssrcs(bytes: &mut Vec<u8>, ssrcs: &[u32]) {
    for ssrc in ssrcs {
        bytes.extend_from_slice(&ssrc.to_be_bytes());
    }
}

fn write_reason(bytes: &mut Vec<u8>, reason: &Option<String>) {
    if let Some(reason_text) = reason {
        bytes.push(reason_text.len() as u8);
        bytes.extend_from_slice(reason_text.as_bytes());

        while !bytes.len().is_multiple_of(4) {
            bytes.push(0);
        }
    }
}

fn parse_ssrcs(data: &[u8], sc: u8) -> Result<(Vec<u32>, usize), String> {
    let mut ssrcs = Vec::new();
    let mut offset = 4;

    for _ in 0..sc {
        if offset + 4 > data.len() {
            return Err("BYE packet truncated".to_string());
        }
        let ssrc = crate::codec::rtp::parse_u32_be(data, offset);
        ssrcs.push(ssrc);
        offset += 4;
    }

    Ok((ssrcs, offset))
}

fn parse_reason(data: &[u8], offset: usize) -> Option<String> {
    if offset >= data.len() {
        return None;
    }

    let len = data[offset] as usize;
    let text_start = offset + 1;

    if text_start + len <= data.len() {
        Some(String::from_utf8_lossy(&data[text_start..text_start + len]).to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bye_packet() {
        let bye = ByePacket::new(12345, Some("Leaving call".to_string()));
        let bytes = bye.to_bytes();

        assert_eq!(bytes[1], RtcpPacketType::BYE as u8);

        let parsed = ByePacket::from_bytes(&bytes).unwrap();
        assert_eq!(parsed.ssrcs[0], 12345);
        assert_eq!(parsed.reason, Some("Leaving call".to_string()));
    }
}
