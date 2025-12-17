use crate::tcp::messages::{Message, MessageType};
use json_parser::parse_json;
use std::io::{self, Read, Write};

/// Protocol error types
#[derive(Debug)]
pub enum ProtocolError {
    Io(io::Error),
    InvalidMessageType(u8),
    JsonParse(String),
    MessageTooLarge(u32),
}

impl From<io::Error> for ProtocolError {
    fn from(err: io::Error) -> Self {
        ProtocolError::Io(err)
    }
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::Io(e) => write!(f, "IO error: {}", e),
            ProtocolError::InvalidMessageType(t) => write!(f, "Invalid message type: 0x{:02X}", t),
            ProtocolError::JsonParse(e) => write!(f, "JSON parse error: {}", e),
            ProtocolError::MessageTooLarge(size) => write!(f, "Message too large: {} bytes", size),
        }
    }
}

impl std::error::Error for ProtocolError {}

pub type Result<T> = std::result::Result<T, ProtocolError>;

const MAX_MESSAGE_SIZE: u32 = 1024 * 1024; // 1 MB
const RETRY_DELAY_MS: u64 = 10;

/// Read exact amount of bytes with retry logic for non-blocking streams
///
/// Handles partial reads and WouldBlock errors gracefully.
fn read_exact_with_retry<S: Read>(
    stream: &mut S,
    buf: &mut [u8],
    description: &str,
) -> io::Result<()> {
    use std::io::ErrorKind;
    use std::time::Duration;

    let mut total_read = 0;

    while total_read < buf.len() {
        match stream.read(&mut buf[total_read..]) {
            Ok(0) => {
                return Err(io::Error::other(format!(
                    "Connection closed while reading {}",
                    description
                )));
            }
            Ok(n) => {
                total_read += n;
            }
            Err(e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::Interrupted => {
                // If no data read yet, propagate WouldBlock to caller
                if total_read == 0 {
                    return Err(e);
                }
                // Partial read: brief sleep then retry for remaining bytes
                std::thread::sleep(Duration::from_millis(RETRY_DELAY_MS));
            }
            Err(e) => return Err(e),
        }
    }
    Ok(())
}

/// Read a message from any stream implementing Read + Write
///
/// Format: [4 bytes length][1 byte type][N bytes JSON]
///
/// # Notes
/// Uses polling-based reads for compatibility with tunnel/proxy deployments.
/// For direct connections, blocking reads would be more efficient.
pub fn read_message<S: Read + Write>(stream: &mut S) -> Result<Message> {
    // Read 4-byte length header
    let mut len_buf = [0u8; 4];
    read_exact_with_retry(stream, &mut len_buf, "length header")?;
    let len = u32::from_be_bytes(len_buf);

    if len > MAX_MESSAGE_SIZE {
        return Err(ProtocolError::MessageTooLarge(len));
    }

    // Read 1-byte message type
    let mut type_buf = [0u8; 1];
    read_exact_with_retry(stream, &mut type_buf, "message type")?;
    let msg_type =
        MessageType::from_u8(type_buf[0]).ok_or(ProtocolError::InvalidMessageType(type_buf[0]))?;

    // Read JSON payload (len includes type byte, so payload is len - 1)
    let payload_len = (len - 1) as usize;
    let mut payload = vec![0u8; payload_len];
    read_exact_with_retry(stream, &mut payload, "payload")?;

    parse_message(msg_type, &payload)
}

/// Parse JSON string into json_parser Value
fn parse_json_payload(payload: &[u8]) -> Result<json_parser::JsonValue> {
    let json_str = String::from_utf8(payload.to_vec())
        .map_err(|e| ProtocolError::JsonParse(format!("UTF-8 error: {}", e)))?;
    parse_json(&json_str).map_err(|e| ProtocolError::JsonParse(e.to_string()))
}

/// Parse JSON payload into Message enum based on message type
fn parse_message(msg_type: MessageType, payload: &[u8]) -> Result<Message> {
    use crate::tcp::messages::{
        CallRequest, CallResponseMsg, HangupMsg, HeartbeatMsg, IceCandidateMsg, LoginRequest,
        LogoutRequest, Message, RegisterRequest, SdpAnswerMsg, SdpOfferMsg,
    };

    let json = parse_json_payload(payload)?;

    // Helper to parse and wrap message variants
    let parse_and_wrap = |json: &json_parser::JsonValue| -> Result<Message> {
        match msg_type {
            MessageType::LoginRequest => LoginRequest::from_json(json)
                .map(Message::LoginRequest)
                .map_err(ProtocolError::JsonParse),
            MessageType::RegisterRequest => RegisterRequest::from_json(json)
                .map(Message::RegisterRequest)
                .map_err(ProtocolError::JsonParse),
            MessageType::LogoutRequest => {
                LogoutRequest::from_json(json).map_err(ProtocolError::JsonParse)?;
                Ok(Message::LogoutRequest(LogoutRequest))
            }
            MessageType::UserListRequest => Ok(Message::UserListRequest),
            MessageType::CallRequest => CallRequest::from_json(json)
                .map(Message::CallRequest)
                .map_err(ProtocolError::JsonParse),
            MessageType::CallResponse => CallResponseMsg::from_json(json)
                .map(Message::CallResponse)
                .map_err(ProtocolError::JsonParse),
            MessageType::SdpOffer => SdpOfferMsg::from_json(json)
                .map(Message::SdpOffer)
                .map_err(ProtocolError::JsonParse),
            MessageType::SdpAnswer => SdpAnswerMsg::from_json(json)
                .map(Message::SdpAnswer)
                .map_err(ProtocolError::JsonParse),
            MessageType::IceCandidate => IceCandidateMsg::from_json(json)
                .map(Message::IceCandidate)
                .map_err(ProtocolError::JsonParse),
            MessageType::Hangup => HangupMsg::from_json(json)
                .map(Message::Hangup)
                .map_err(ProtocolError::JsonParse),
            MessageType::Heartbeat => HeartbeatMsg::from_json(json)
                .map(Message::Heartbeat)
                .map_err(ProtocolError::JsonParse),

            // These are server→client messages, shouldn't be received by server
            MessageType::LoginResponse
            | MessageType::RegisterResponse
            | MessageType::LogoutResponse
            | MessageType::UserListResponse
            | MessageType::UserStateUpdate
            | MessageType::CallNotification
            | MessageType::CallAccepted
            | MessageType::CallDeclined
            | MessageType::Error => Err(ProtocolError::InvalidMessageType(msg_type as u8)),
        }
    };

    parse_and_wrap(&json)
}

/// Serialize message to JSON bytes
fn serialize_message(message: &Message) -> Vec<u8> {
    message.to_json().to_string().into_bytes()
}

/// Write a message to the TCP stream
///
/// Format: [4 bytes length][1 byte type][N bytes JSON]
pub fn write_message<S: Read + Write>(stream: &mut S, message: &Message) -> Result<()> {
    let msg_type = message.message_type();
    let payload = serialize_message(message);

    // Calculate total length (1 byte type + payload length)
    let total_len = 1 + payload.len() as u32;

    if total_len > MAX_MESSAGE_SIZE {
        return Err(ProtocolError::MessageTooLarge(total_len));
    }

    // Write length (4 bytes, big-endian)
    stream.write_all(&total_len.to_be_bytes())?;

    // Write type (1 byte)
    stream.write_all(&[msg_type as u8])?;

    // Write payload
    stream.write_all(&payload)?;

    // Flush to ensure message is sent
    stream.flush()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tcp::messages::{ErrorMsg, LoginResponse};
    use std::io::Cursor;

    #[test]
    fn test_write_and_read_login_response() {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let response = Message::LoginResponse(LoginResponse {
            success: true,
            user_id: Some("user123".to_string()),
            username: Some("alice".to_string()),
            error: None,
        });

        // Write message (server would send this)
        write_message(&mut cursor, &response).expect("Failed to write message");

        // Verify the bytes were written correctly
        assert!(buffer.len() > 5); // At least header + type + some payload

        // Verify message type byte
        assert_eq!(buffer[4], MessageType::LoginResponse as u8);
    }

    #[test]
    fn test_write_and_read_error_message() {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        let error = Message::Error(ErrorMsg {
            code: 404,
            message: "Not found".to_string(),
        });

        // Write message (server would send this)
        write_message(&mut cursor, &error).expect("Failed to write message");

        // Verify the bytes were written correctly
        assert!(buffer.len() > 5);

        // Verify message type byte
        assert_eq!(buffer[4], MessageType::Error as u8);
    }

    #[test]
    fn test_message_too_large() {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        // Create a message with huge payload (should exceed MAX_MESSAGE_SIZE)
        let large_string = "x".repeat(MAX_MESSAGE_SIZE as usize);
        let error = Message::Error(ErrorMsg {
            code: 0,
            message: large_string,
        });

        let result = write_message(&mut cursor, &error);
        assert!(matches!(result, Err(ProtocolError::MessageTooLarge(_))));
    }

    #[test]
    fn test_read_invalid_message_type() {
        let mut buffer = Vec::new();
        let mut cursor = Cursor::new(&mut buffer);

        // Write manually with invalid type
        let len: u32 = 10; // length
        cursor.write_all(&len.to_be_bytes()).unwrap();
        cursor.write_all(&[255u8]).unwrap(); // Invalid message type
        cursor.write_all(b"{\"test\":1}").unwrap();

        cursor.set_position(0);

        let result = read_message(&mut cursor);
        assert!(matches!(
            result,
            Err(ProtocolError::InvalidMessageType(255))
        ));
    }

    #[test]
    fn test_read_truncated_header() {
        let mut buffer = vec![0u8, 1u8]; // Only 2 bytes, need 4 for length
        let mut cursor = Cursor::new(&mut buffer);

        let result = read_message(&mut cursor);
        assert!(matches!(result, Err(ProtocolError::Io(_))));
    }

    #[test]
    fn test_parse_json_payload_invalid_utf8() {
        let invalid_utf8 = vec![0xFF, 0xFE, 0xFD];
        let result = parse_json_payload(&invalid_utf8);
        assert!(matches!(result, Err(ProtocolError::JsonParse(_))));
    }

    #[test]
    fn test_serialize_message() {
        let response = Message::LoginResponse(LoginResponse {
            success: false,
            user_id: None,
            username: None,
            error: Some("Invalid credentials".to_string()),
        });

        let json_bytes = serialize_message(&response);
        let json_str = String::from_utf8(json_bytes).unwrap();

        assert!(json_str.contains("success"));
        assert!(json_str.contains("false"));
        assert!(json_str.contains("Invalid credentials"));
    }
}
