//! File transfer message protocol

use std::io;

/// File transfer message types
#[derive(Debug, Clone)]
pub enum FileTransferMessage {
    /// Offer to send a file
    Offer {
        id: u64,
        filename: String,
        size: u64,
        mime_type: String,
    },
    /// Accept file transfer
    Accept { id: u64 },
    /// Reject file transfer
    Reject { id: u64, reason: String },
    /// File data chunk
    Data { id: u64, offset: u64, data: Vec<u8> },
    /// Transfer completed
    Complete { id: u64, checksum: u64 },
    /// Cancel ongoing transfer
    Cancel { id: u64, reason: String },
}

impl FileTransferMessage {
    const TYPE_OFFER: u8 = 0x01;
    const TYPE_ACCEPT: u8 = 0x02;
    const TYPE_REJECT: u8 = 0x03;
    const TYPE_DATA: u8 = 0x04;
    const TYPE_COMPLETE: u8 = 0x05;
    const TYPE_CANCEL: u8 = 0x06;

    /// Serialize to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Self::Offer {
                id,
                filename,
                size,
                mime_type,
            } => Self::serialize_offer(*id, filename, *size, mime_type),
            Self::Accept { id } => Self::serialize_accept(*id),
            Self::Reject { id, reason } => Self::serialize_reject(*id, reason),
            Self::Data { id, offset, data } => Self::serialize_data(*id, *offset, data),
            Self::Complete { id, checksum } => Self::serialize_complete(*id, *checksum),
            Self::Cancel { id, reason } => Self::serialize_cancel(*id, reason),
        }
    }

    fn serialize_offer(id: u64, filename: &str, size: u64, mime_type: &str) -> Vec<u8> {
        let filename_bytes = filename.as_bytes();
        let mime_bytes = mime_type.as_bytes();
        let mut buf = Vec::with_capacity(21 + filename_bytes.len() + mime_bytes.len());
        buf.push(Self::TYPE_OFFER);
        buf.extend_from_slice(&id.to_be_bytes());
        buf.extend_from_slice(&size.to_be_bytes());
        buf.extend_from_slice(&(filename_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(filename_bytes);
        buf.extend_from_slice(&(mime_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(mime_bytes);
        buf
    }

    fn serialize_accept(id: u64) -> Vec<u8> {
        let mut buf = Vec::with_capacity(9);
        buf.push(Self::TYPE_ACCEPT);
        buf.extend_from_slice(&id.to_be_bytes());
        buf
    }

    fn serialize_reject(id: u64, reason: &str) -> Vec<u8> {
        let reason_bytes = reason.as_bytes();
        let mut buf = Vec::with_capacity(11 + reason_bytes.len());
        buf.push(Self::TYPE_REJECT);
        buf.extend_from_slice(&id.to_be_bytes());
        buf.extend_from_slice(&(reason_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(reason_bytes);
        buf
    }

    fn serialize_data(id: u64, offset: u64, data: &[u8]) -> Vec<u8> {
        let mut buf = Vec::with_capacity(17 + data.len());
        buf.push(Self::TYPE_DATA);
        buf.extend_from_slice(&id.to_be_bytes());
        buf.extend_from_slice(&offset.to_be_bytes());
        buf.extend_from_slice(data);
        buf
    }

    fn serialize_complete(id: u64, checksum: u64) -> Vec<u8> {
        let mut buf = Vec::with_capacity(17);
        buf.push(Self::TYPE_COMPLETE);
        buf.extend_from_slice(&id.to_be_bytes());
        buf.extend_from_slice(&checksum.to_be_bytes());
        buf
    }

    fn serialize_cancel(id: u64, reason: &str) -> Vec<u8> {
        let reason_bytes = reason.as_bytes();
        let mut buf = Vec::with_capacity(11 + reason_bytes.len());
        buf.push(Self::TYPE_CANCEL);
        buf.extend_from_slice(&id.to_be_bytes());
        buf.extend_from_slice(&(reason_bytes.len() as u16).to_be_bytes());
        buf.extend_from_slice(reason_bytes);
        buf
    }

    /// Parse from bytes
    pub fn from_bytes(data: &[u8]) -> io::Result<Self> {
        if data.is_empty() {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Empty message"));
        }
        match data[0] {
            Self::TYPE_OFFER => Self::parse_offer(data),
            Self::TYPE_ACCEPT => Self::parse_accept(data),
            Self::TYPE_REJECT => Self::parse_reject(data),
            Self::TYPE_DATA => Self::parse_data(data),
            Self::TYPE_COMPLETE => Self::parse_complete(data),
            Self::TYPE_CANCEL => Self::parse_cancel(data),
            _ => Err(io::Error::new(io::ErrorKind::InvalidData, "Unknown type")),
        }
    }

    fn parse_offer(data: &[u8]) -> io::Result<Self> {
        if data.len() < 21 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Offer too short",
            ));
        }
        let id = u64::from_be_bytes(data[1..9].try_into().unwrap());
        let size = u64::from_be_bytes(data[9..17].try_into().unwrap());
        let filename_len = u16::from_be_bytes(data[17..19].try_into().unwrap()) as usize;
        if data.len() < 21 + filename_len {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Truncated"));
        }
        let filename = String::from_utf8(data[19..19 + filename_len].to_vec())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "Invalid UTF-8"))?;
        let mime_offset = 19 + filename_len;
        let mime_len =
            u16::from_be_bytes(data[mime_offset..mime_offset + 2].try_into().unwrap()) as usize;
        let mime_type = String::from_utf8(
            data[mime_offset + 2..]
                .get(..mime_len)
                .unwrap_or_default()
                .to_vec(),
        )
        .unwrap_or_default();
        Ok(Self::Offer {
            id,
            filename,
            size,
            mime_type,
        })
    }

    fn parse_accept(data: &[u8]) -> io::Result<Self> {
        if data.len() < 9 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Too short"));
        }
        let id = u64::from_be_bytes(data[1..9].try_into().unwrap());
        Ok(Self::Accept { id })
    }

    fn parse_reject(data: &[u8]) -> io::Result<Self> {
        if data.len() < 11 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Too short"));
        }
        let id = u64::from_be_bytes(data[1..9].try_into().unwrap());
        let reason_len = u16::from_be_bytes(data[9..11].try_into().unwrap()) as usize;
        let reason = String::from_utf8(data.get(11..11 + reason_len).unwrap_or_default().to_vec())
            .unwrap_or_default();
        Ok(Self::Reject { id, reason })
    }

    fn parse_data(data: &[u8]) -> io::Result<Self> {
        if data.len() < 17 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Too short"));
        }
        let id = u64::from_be_bytes(data[1..9].try_into().unwrap());
        let offset = u64::from_be_bytes(data[9..17].try_into().unwrap());
        Ok(Self::Data {
            id,
            offset,
            data: data[17..].to_vec(),
        })
    }

    fn parse_complete(data: &[u8]) -> io::Result<Self> {
        if data.len() < 17 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Too short"));
        }
        let id = u64::from_be_bytes(data[1..9].try_into().unwrap());
        let checksum = u64::from_be_bytes(data[9..17].try_into().unwrap());
        Ok(Self::Complete { id, checksum })
    }

    fn parse_cancel(data: &[u8]) -> io::Result<Self> {
        if data.len() < 11 {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "Too short"));
        }
        let id = u64::from_be_bytes(data[1..9].try_into().unwrap());
        let reason_len = u16::from_be_bytes(data[9..11].try_into().unwrap()) as usize;
        let reason = String::from_utf8(data.get(11..11 + reason_len).unwrap_or_default().to_vec())
            .unwrap_or_default();
        Ok(Self::Cancel { id, reason })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_offer_roundtrip() {
        let msg = FileTransferMessage::Offer {
            id: 12345,
            filename: "test.txt".to_string(),
            size: 1024,
            mime_type: "text/plain".to_string(),
        };
        let bytes = msg.to_bytes();
        let parsed = FileTransferMessage::from_bytes(&bytes).unwrap();
        match parsed {
            FileTransferMessage::Offer {
                id, filename, size, ..
            } => {
                assert_eq!(id, 12345);
                assert_eq!(filename, "test.txt");
                assert_eq!(size, 1024);
            }
            _ => panic!("Wrong type"),
        }
    }
}
