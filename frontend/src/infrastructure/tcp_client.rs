use crate::models::protocol::{ServerMessage, UserInfo};
use logging::Logger;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::{Arc, Mutex};

/// TCP client for persistent connection to server
pub struct TcpClient {
    stream: Arc<Mutex<Option<native_tls::TlsStream<TcpStream>>>>,
    pending_messages: Arc<Mutex<Vec<ServerMessage>>>,
    logger: Logger,
}

impl TcpClient {
    /// Connect to the server with TLS
    pub fn connect(addr: &str, logger: &Logger) -> Result<Self, String> {
        logger.info(&format!("[TCP] Connecting to signaling server: {}", addr));

        let tls_stream = super::tls_client::connect_tls(addr)
            .map_err(|e| format!("Failed to connect with TLS to {}: {}", addr, e))?;

        logger.info("[TCP] TLS handshake successful");

        // Keep stream in blocking mode for reliable message reading

        Ok(TcpClient {
            stream: Arc::new(Mutex::new(Some(tls_stream))),
            pending_messages: Arc::new(Mutex::new(Vec::new())),
            logger: logger.clone(),
        })
    }

    pub fn login(&self, username: &str, password: &str) -> Result<(), String> {
        let message = format!(
            r#"{{"username":"{}","password_hash":"{}"}}"#,
            username,
            hash_password(password)
        );
        self.send_message(0x01, &message)
    }

    pub fn register(&self, username: &str, password: &str) -> Result<(), String> {
        let message = format!(
            r#"{{"username":"{}","password_hash":"{}"}}"#,
            username,
            hash_password(password)
        );
        self.send_message(0x03, &message)
    }

    /// Send logout request
    pub fn logout(&self) -> Result<(), String> {
        self.send_message(0x13, "{}")
    }

    pub fn request_user_list(&self) -> Result<(), String> {
        self.send_message(0x05, "{}")
    }

    /// Send call request
    pub fn call_user(&self, to_user_id: &str) -> Result<(), String> {
        let message = format!(r#"{{"to_user_id":"{}"}}"#, to_user_id);
        self.send_message(0x08, &message)
    }

    /// Accept or decline a call
    pub fn respond_to_call(&self, call_id: &str, accept: bool) -> Result<(), String> {
        let message = format!(r#"{{"call_id":"{}","accepted":{}}}"#, call_id, accept);
        self.send_message(0x0A, &message)
    }

    pub fn send_sdp_offer(
        &self,
        call_id: &str,
        to_user_id: &str,
        from_user_id: &str,
        sdp: &str,
    ) -> Result<(), String> {
        self.logger.info(&format!(
            "[TCP] Sending SDP offer - call_id: {}, from: {}, to: {}, sdp_len: {} bytes",
            call_id,
            from_user_id,
            to_user_id,
            sdp.len()
        ));

        let sdp_escaped = sdp
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('"', "\\\"");
        let message = format!(
            r#"{{"call_id":"{}","from_user_id":"{}","to_user_id":"{}","sdp":"{}"}}"#,
            call_id, from_user_id, to_user_id, sdp_escaped
        );

        let result = self.send_message(0x0D, &message);

        match &result {
            Ok(_) => self.logger.info(&format!(
                "[TCP] SDP offer sent successfully - call_id: {}, msg_len: {} bytes",
                call_id,
                message.len()
            )),
            Err(e) => self.logger.error(&format!(
                "[TCP] Failed to send SDP offer for call '{}': {}",
                call_id, e
            )),
        }

        result
    }

    pub fn send_sdp_answer(
        &self,
        call_id: &str,
        to_user_id: &str,
        from_user_id: &str,
        sdp: &str,
    ) -> Result<(), String> {
        self.logger.info(&format!(
            "[TCP] Sending SDP answer - call_id: {}, from: {}, to: {}, sdp_len: {} bytes",
            call_id,
            from_user_id,
            to_user_id,
            sdp.len()
        ));

        let sdp_escaped = sdp
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('"', "\\\"");
        let message = format!(
            r#"{{"call_id":"{}","from_user_id":"{}","to_user_id":"{}","sdp":"{}"}}"#,
            call_id, from_user_id, to_user_id, sdp_escaped
        );

        let result = self.send_message(0x0E, &message);
        if let Err(ref e) = result {
            self.logger.error(&format!(
                "[TCP] Failed to send SDP answer for call '{}': {}",
                call_id, e
            ));
        }
        result
    }

    pub fn hangup(&self, call_id: &str) -> Result<(), String> {
        let message = format!(r#"{{"call_id":"{}"}}"#, call_id);
        self.send_message(0x10, &message)
    }

    /// Poll for incoming messages (non-blocking)
    /// Reads from the stream and returns any complete messages
    pub fn poll_messages(&self) -> Vec<ServerMessage> {
        let mut stream_lock = match self.stream.lock() {
            Ok(lock) => lock,
            Err(_) => return Vec::new(),
        };

        let stream = match stream_lock.as_mut() {
            Some(s) => s,
            None => return Vec::new(),
        };

        // Set stream to non-blocking temporarily to check if data is available
        if stream.get_ref().set_nonblocking(true).is_err() {
            return Vec::new();
        }

        // Try to read messages from the stream
        loop {
            // Peek to see if data is available
            let mut peek_buf = [0u8; 1];
            match stream.get_ref().peek(&mut peek_buf) {
                Ok(0) | Err(_) => {
                    // No data available or error, stop reading
                    break;
                }
                Ok(_) => {
                    // Data is available, switch back to blocking for reliable read
                    if stream.get_ref().set_nonblocking(false).is_err() {
                        break;
                    }
                }
            }

            // Read message length (4 bytes) - now in blocking mode
            let mut len_buf = [0u8; 4];
            if let Err(e) = stream.read_exact(&mut len_buf) {
                self.logger
                    .error(&format!("[TCP] Failed to read message length: {}", e));
                break;
            }

            let total_len = u32::from_be_bytes(len_buf) as usize;
            if total_len == 0 {
                self.logger
                    .warn("[TCP] Received zero-length message, closing connection");
                break;
            }

            // Read message type (1 byte)
            let mut type_buf = [0u8; 1];
            if let Err(e) = stream.read_exact(&mut type_buf) {
                self.logger
                    .error(&format!("[TCP] Failed to read message type: {}", e));
                break;
            }
            let msg_type = type_buf[0];

            let payload_len = total_len.saturating_sub(1);
            let mut payload = vec![0u8; payload_len];
            if let Err(e) = stream.read_exact(&mut payload) {
                self.logger
                    .error(&format!("[TCP] Failed to read message payload: {}", e));
                break;
            }

            // Parse JSON
            if let Ok(json) = String::from_utf8(payload)
                && let Some(server_msg) = parse_server_message(msg_type, &json)
                && let Ok(mut pending) = self.pending_messages.lock()
            {
                pending.push(server_msg);
            }

            // Set back to non-blocking to check for more messages
            if stream.get_ref().set_nonblocking(true).is_err() {
                break;
            }
        }

        // Ensure stream is left in blocking mode for writes
        let _ = stream.get_ref().set_nonblocking(false);

        let mut pending = match self.pending_messages.lock() {
            Ok(lock) => lock,
            Err(_) => return Vec::new(),
        };

        std::mem::take(&mut *pending)
    }

    /// Send a message with the protocol format: [length][type][payload]
    fn send_message(&self, msg_type: u8, payload: &str) -> Result<(), String> {
        let payload_bytes = payload.as_bytes();
        let total_len = 1 + payload_bytes.len() as u32;

        let mut stream = self.stream.lock().map_err(|_| "Failed to lock stream")?;

        let stream = stream.as_mut().ok_or("Not connected")?;

        // Create an unique buffer to avoid TLS fragmentation
        let mut buffer = Vec::with_capacity(4 + 1 + payload_bytes.len());

        // 1. Write length (4 bytes, big-endian)
        buffer.extend_from_slice(&total_len.to_be_bytes());

        // 2. Write type (1 byte)
        buffer.push(msg_type);

        // 3. Write payload
        buffer.extend_from_slice(payload_bytes);

        // Flush all
        stream
            .write_all(&buffer)
            .map_err(|e| format!("Failed to write message: {}", e))?;

        stream
            .flush()
            .map_err(|e| format!("Failed to flush: {}", e))?;

        Ok(())
    }

    /// Disconnect from server
    pub fn disconnect(&mut self) {
        if let Ok(mut stream) = self.stream.lock() {
            *stream = None;
        }
    }
}

impl Drop for TcpClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Parse server message from JSON
fn parse_server_message(msg_type: u8, json: &str) -> Option<ServerMessage> {
    match msg_type {
        0x02 => parse_login_response(json),
        0x04 => parse_register_response(json),
        0x14 => parse_logout_response(json),
        0x06 => parse_user_list_response(json),
        0x07 => parse_user_state_update(json),
        0x09 => parse_call_notification(json),
        0x0B => parse_call_accepted(json),
        0x0C => parse_call_declined(json),
        0x0D => parse_sdp_offer(json),
        0x0E => parse_sdp_answer(json),
        0x0F => parse_ice_candidate(json),
        0x10 => parse_hangup(json),
        0x12 => parse_error(json),
        _ => None,
    }
}

/// Simple JSON parsing helpers
fn parse_login_response(json: &str) -> Option<ServerMessage> {
    let success = json.contains("\"success\":true");
    let username = extract_string(json, "username");
    let user_id = extract_string(json, "user_id");
    let error = extract_string(json, "error");

    Some(ServerMessage::LoginResponse {
        success,
        username,
        user_id,
        error,
    })
}

fn parse_register_response(json: &str) -> Option<ServerMessage> {
    let success = json.contains("\"success\":true");
    let username = extract_string(json, "username");
    let user_id = extract_string(json, "user_id");
    let error = extract_string(json, "error");

    Some(ServerMessage::RegisterResponse {
        success,
        username,
        user_id,
        error,
    })
}

fn parse_logout_response(json: &str) -> Option<ServerMessage> {
    let success = json.contains("\"success\":true");
    let error = extract_string(json, "error");

    Some(ServerMessage::LogoutResponse { success, error })
}

fn parse_user_list_response(json: &str) -> Option<ServerMessage> {
    let mut users = Vec::new();

    // Find "users" array
    if let Some(start) = json.find("\"users\":[") {
        let rest = &json[start + 9..];
        if let Some(end) = rest.find(']') {
            let array_content = &rest[..end];

            // Split by objects
            for obj in array_content.split("},") {
                let obj = obj.trim().trim_start_matches('{').trim_end_matches('}');
                if !obj.is_empty()
                    && let (Some(user_id), Some(username), Some(state)) = (
                        extract_string(obj, "user_id"),
                        extract_string(obj, "username"),
                        extract_string(obj, "state"),
                    )
                {
                    users.push(UserInfo {
                        user_id,
                        username,
                        state,
                    });
                }
            }
        }
    }

    Some(ServerMessage::UserListResponse { users })
}

fn parse_user_state_update(json: &str) -> Option<ServerMessage> {
    Some(ServerMessage::UserStateUpdate {
        user_id: extract_string(json, "user_id")?,
        username: extract_string(json, "username")?,
        state: extract_string(json, "state")?,
    })
}

fn parse_call_notification(json: &str) -> Option<ServerMessage> {
    Some(ServerMessage::CallNotification {
        call_id: extract_string(json, "call_id")?,
        from_user_id: extract_string(json, "from_user_id")?,
        from_username: extract_string(json, "from_username")?,
    })
}

fn parse_call_accepted(json: &str) -> Option<ServerMessage> {
    Some(ServerMessage::CallAccepted {
        call_id: extract_string(json, "call_id")?,
        peer_user_id: extract_string(json, "peer_user_id")?,
        peer_username: extract_string(json, "peer_username")?,
    })
}

fn parse_call_declined(json: &str) -> Option<ServerMessage> {
    Some(ServerMessage::CallDeclined {
        peer_username: extract_string(json, "peer_username")?,
    })
}

fn parse_sdp_offer(json: &str) -> Option<ServerMessage> {
    Some(ServerMessage::SdpOffer {
        call_id: extract_string(json, "call_id")?,
        from_user_id: extract_string(json, "from_user_id")?,
        sdp: extract_string(json, "sdp")?
            .replace("\\n", "\n")
            .replace("\\r", "\r"),
    })
}

fn parse_sdp_answer(json: &str) -> Option<ServerMessage> {
    Some(ServerMessage::SdpAnswer {
        call_id: extract_string(json, "call_id")?,
        from_user_id: extract_string(json, "from_user_id")?,
        sdp: extract_string(json, "sdp")?
            .replace("\\n", "\n")
            .replace("\\r", "\r"),
    })
}

fn parse_ice_candidate(json: &str) -> Option<ServerMessage> {
    let sdp_mline_index = extract_number(json, "sdp_mline_index").unwrap_or(0);

    Some(ServerMessage::IceCandidate {
        candidate: extract_string(json, "candidate")?,
        sdp_mid: extract_string(json, "sdp_mid")?,
        sdp_mline_index,
    })
}

fn parse_hangup(json: &str) -> Option<ServerMessage> {
    Some(ServerMessage::Hangup {
        call_id: extract_string(json, "call_id")?,
    })
}

fn parse_error(json: &str) -> Option<ServerMessage> {
    let message = extract_string(json, "message").unwrap_or_else(|| "Unknown error".to_string());

    Some(ServerMessage::Error { message })
}

/// Extract string value from JSON
fn extract_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":\"", key);
    if let Some(start) = json.find(&pattern) {
        let value_start = start + pattern.len();
        let rest = &json[value_start..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    None
}

/// Extract number value from JSON
fn extract_number(json: &str, key: &str) -> Option<u32> {
    let pattern = format!("\"{}\":", key);
    if let Some(start) = json.find(&pattern) {
        let value_start = start + pattern.len();
        let rest = &json[value_start..].trim();

        // Find the end of the number (comma, brace, or bracket)
        let mut end = 0;
        for (i, ch) in rest.chars().enumerate() {
            if ch == ',' || ch == '}' || ch == ']' {
                end = i;
                break;
            }
        }

        if end > 0
            && let Ok(num) = rest[..end].trim().parse()
        {
            return Some(num);
        }
    }
    None
}

/// Simple password hashing
/// SHA-256 would be better
fn hash_password(password: &str) -> String {
    format!(
        "{:x}",
        password
            .bytes()
            .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64))
    )
}
