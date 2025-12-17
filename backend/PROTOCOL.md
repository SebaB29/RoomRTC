# RoomRTC Protocol Specification v1.0

## Overview
This document defines the persistent TCP-based signaling protocol for RoomRTC. All messages are exchanged over a persistent TCP connection with optional TLS encryption using PKCS#12 certificates.

## Architecture

### Protocol Location
The protocol implementation is located in `backend/src/tcp/protocol.rs` with the following structure:

```
tcp/
├── protocol.rs        # Binary protocol implementation
├── messages/          # Message type definitions
├── server.rs          # TCP server with optional TLS
├── client_handler.rs  # Per-client connection handler
├── stream_type.rs     # Plain/TLS stream abstraction
└── tls/               # TLS module (PKCS#12 support)
```

### Key Features
- **Binary Protocol**: Efficient message format with 4-byte length + 1-byte type + JSON payload
- **Polling-based Reads**: Compatible with tunnel/proxy deployments (Tailscale Funnel)
- **TLS Support**: Native TLS using PKCS#12 certificates via `native-tls` crate
- **Clean Architecture**: Separated into domain, application, infrastructure, and TCP layers

## Connection Flow
1. Client establishes TCP connection to server
2. Client sends LOGIN_REQUEST
3. Server responds with LOGIN_RESPONSE
4. Connection remains open for bidirectional messaging
5. Server pushes USER_STATE_UPDATE messages when any user's state changes
6. Client sends CALL_REQUEST to initiate a call
7. Server forwards to target user, who responds with CALL_RESPONSE
8. Upon acceptance, both users exchange SDP_OFFER/SDP_ANSWER
9. ICE candidates exchanged via ICE_CANDIDATE messages
10. Either user can send HANGUP to end the call

## Message Format

All messages follow this binary format:

```
[4 bytes: message length (u32 big-endian)]
[1 byte: message type (u8)]
[N bytes: JSON payload (UTF-8)]
```

### Example Binary Layout
```
00 00 00 2A    # Length = 42 bytes
01             # Type = LOGIN_REQUEST
{ ... JSON ... }
```

## Message Types

| Type ID | Name | Direction | Description |
|---------|------|-----------|-------------|
| 0x01 | LOGIN_REQUEST | Client→Server | User authentication |
| 0x02 | LOGIN_RESPONSE | Server→Client | Login success/failure |
| 0x03 | REGISTER_REQUEST | Client→Server | New user registration |
| 0x04 | REGISTER_RESPONSE | Server→Client | Registration result |
| 0x05 | USER_LIST_REQUEST | Client→Server | Request all users |
| 0x06 | USER_LIST_RESPONSE | Server→Client | List of users with states |
| 0x07 | USER_STATE_UPDATE | Server→Client | Push notification of state change |
| 0x08 | CALL_REQUEST | Client→Server | Initiate call to user |
| 0x09 | CALL_NOTIFICATION | Server→Client | Notify user of incoming call |
| 0x0A | CALL_RESPONSE | Client→Server | Accept or decline call |
| 0x0B | CALL_ACCEPTED | Server→Client | Notify caller of acceptance |
| 0x0C | CALL_DECLINED | Server→Client | Notify caller of decline |
| 0x0D | SDP_OFFER | Client→Server→Client | WebRTC offer |
| 0x0E | SDP_ANSWER | Client→Server→Client | WebRTC answer |
| 0x0F | ICE_CANDIDATE | Client→Server→Client | ICE candidate |
| 0x10 | HANGUP | Client→Server→Client | End call |
| 0x11 | HEARTBEAT | Client⇄Server | Keep-alive ping/pong |
| 0x12 | ERROR | Server→Client | Error notification |
| 0x13 | LOGOUT_REQUEST | Client→Server | User logout |
| 0x14 | LOGOUT_RESPONSE | Server→Client | Logout confirmation |

## User States

```rust
/// User state enum with Display implementation
pub enum UserState {
    Disconnected,  // Registered but not connected
    Available,     // Connected and available for calls
    Busy,          // Currently in an active call
}
```

**Display format**:
- `Disconnected` → "Disconnected"
- `Available` → "Available"
- `Busy` → "Busy"

### State Transitions
```
Registration
     ↓
Disconnected
     ↓ (login)
Available ← → Busy (call accepted/hangup)
     ↓ (logout/disconnect)
Disconnected
```

**Automatic transitions**:
- Connection lost → Disconnected (broadcast to all clients)
- User in call disconnects → Peer notified with HANGUP, both return to Available
- Logout → Disconnected (explicit user action)
- State changes broadcast to all connected clients via USER_STATE_UPDATE

## Message Definitions

### 0x01 - LOGIN_REQUEST
Client authenticates with username and password hash.

**Client → Server**
```json
{
  "username": "alice",
  "password_hash": "hashed_password"
}
```

### 0x02 - LOGIN_RESPONSE
Server responds with success and user ID, or failure reason.

**Server → Client (Success)**
```json
{
  "success": true,
  "user_id": "abc123",
  "username": "alice"
}
```

**Server → Client (Failure)**
```json
{
  "success": false,
  "error": "Invalid credentials"
}
```

### 0x03 - REGISTER_REQUEST
Client creates a new account.

**Client → Server**
```json
{
  "username": "bob",
  "password_hash": "hashed_password"
}
```

### 0x04 - REGISTER_RESPONSE
Server confirms registration or reports conflict.

**Server → Client**
```json
{
  "success": true,
  "user_id": "def456"
}
```

### 0x05 - USER_LIST_REQUEST
Client requests the current list of all registered users.

**Client → Server**
```json
{}
```

### 0x06 - USER_LIST_RESPONSE
Server provides list of users with their current states.

**Server → Client**
```json
{
  "users": [
    {
      "user_id": "abc123",
      "username": "alice",
      "state": "Available"
    },
    {
      "user_id": "def456",
      "username": "bob",
      "state": "Busy"
    },
    {
      "user_id": "ghi789",
      "username": "charlie",
      "state": "Disconnected"
    }
  ]
}
```

### 0x07 - USER_STATE_UPDATE
Server pushes state changes to all connected clients in real-time.

**Server → Client**
```json
{
  "user_id": "abc123",
  "username": "alice",
  "state": "Busy"
}
```

### 0x08 - CALL_REQUEST
Client initiates a call to another user.

**Client → Server**
```json
{
  "to_user_id": "def456"
}
```

### 0x09 - CALL_NOTIFICATION
Server notifies the target user of an incoming call.

**Server → Client**
```json
{
  "call_id": "call_xyz789",
  "from_user_id": "abc123",
  "from_username": "alice"
}
```

### 0x0A - CALL_RESPONSE
Client accepts or declines an incoming call.

**Client → Server**
```json
{
  "call_id": "call_xyz789",
  "accepted": true
}
```

### 0x0B - CALL_ACCEPTED
Server notifies the caller that their call was accepted.

**Server → Client**
```json
{
  "call_id": "call_xyz789",
  "peer_user_id": "def456",
  "peer_username": "bob"
}
```

### 0x0C - CALL_DECLINED
Server notifies the caller that their call was declined.

**Server → Client**
```json
{
  "call_id": "call_xyz789",
  "peer_user_id": "def456",
  "peer_username": "bob"
}
```

### 0x0D - SDP_OFFER
Client sends WebRTC SDP offer, server forwards to peer.

**Client → Server → Client**
```json
{
  "call_id": "call_xyz789",
  "from_user_id": "abc123",
  "to_user_id": "def456",
  "sdp": "v=0\r\no=- 123456 2 IN IP4 0.0.0.0\r\n..."
}
```

### 0x0E - SDP_ANSWER
Client sends WebRTC SDP answer, server forwards to peer.

**Client → Server → Client**
```json
{
  "call_id": "call_xyz789",
  "from_user_id": "def456",
  "to_user_id": "abc123",
  "sdp": "v=0\r\no=- 789012 2 IN IP4 0.0.0.0\r\n..."
}
```

### 0x0F - ICE_CANDIDATE
Client sends ICE candidate, server forwards to peer.

**Client → Server → Client**
```json
{
  "call_id": "call_xyz789",
  "from_user_id": "abc123",
  "to_user_id": "def456",
  "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54321 typ host",
  "sdp_mid": "0",
  "sdp_mline_index": 0
}
```

### 0x10 - HANGUP
Client ends an active call.

**Client → Server → Client**
```json
{
  "call_id": "call_xyz789"
}
```

### 0x11 - HEARTBEAT
Keep-alive message to detect dead connections.

**Client ⇄ Server**
```json
{
  "timestamp": 1234567890
}
```

Server expects HEARTBEAT every 30 seconds. If no message received for 60 seconds, connection is terminated.

### 0x12 - ERROR
Server reports an error to client.

**Server → Client**
```json
{
  "code": 400,
  "message": "User not available for calls"
}
```

### 0x13 - LOGOUT_REQUEST
Client initiates logout and disconnection.

**Client → Server**
```json
{}
```

**Note**: Empty JSON object. The server identifies the user from the authenticated session.

### 0x14 - LOGOUT_RESPONSE
Server confirms successful logout.

**Server → Client**
```json
{
  "success": true,
  "error": null
}
```

**On Error**:
```json
{
  "success": false,
  "error": "Error message"
}
```

## Error Codes

| Code | Meaning |
|------|---------|
| 400 | Bad Request - Invalid message format |
| 401 | Unauthorized - Login required |
| 404 | Not Found - User does not exist |
| 409 | Conflict - Username already taken or user already in call |
| 500 | Internal Server Error |

## Security

### Transport Encryption
**TLS 1.2+** with PKCS#12 certificates. The server uses `native-tls` crate for platform-native TLS implementation.

**Certificate Format**: PKCS#12 (.p12 or .pfx) containing both certificate and private key.

**Generate PKCS#12**:
```bash
# From PEM files
openssl pkcs12 -export -out server.p12 \
  -inkey server.key \
  -in server.crt \
  -password pass:your_password
```

**Configuration**:
```json
{
  "server": {
    "enable_tls": true,
    "pkcs12_path": "/path/to/server.p12",
    "pkcs12_password": "your_password"
  }
}
```

**Without TLS**: Only use with TLS-terminating proxy (e.g., Tailscale Funnel, nginx).

### Authentication
Passwords are hashed using bcrypt before storage. The user authentication flow:
1. Client sends username + password_hash
2. Server verifies against bcrypt hash stored in `users.txt`
3. Server creates session and sends success response
4. Session persists for duration of TCP connection

**Note**: The `password_hash` field in LOGIN_REQUEST and REGISTER_REQUEST should contain a pre-hashed password from the client. The server applies an additional bcrypt hash before storage/verification for defense in depth.

### Session Management
- Each connection is identified by a unique session ID
- Only one connection per user is allowed
- New login terminates previous session

## Example Flow: Alice calls Bob

```
1. Alice connects, sends LOGIN_REQUEST
   Server responds with LOGIN_RESPONSE (success)
   
2. Alice requests user list: USER_LIST_REQUEST
   Server responds: USER_LIST_RESPONSE (Bob is "Available")
   
3. Alice initiates call: CALL_REQUEST { to_user_id: "bob_id" }
   Server forwards: CALL_NOTIFICATION to Bob
   Server broadcasts: USER_STATE_UPDATE (Alice → "Busy")
   
4. Bob accepts: CALL_RESPONSE { call_id: "...", accepted: true }
   Server notifies Alice: CALL_ACCEPTED
   Server broadcasts: USER_STATE_UPDATE (Bob → "Busy")
   
5. Alice sends SDP_OFFER to Bob (via server)
6. Bob sends SDP_ANSWER to Alice (via server)
7. Both exchange ICE_CANDIDATE messages

8. (Call in progress - media flows via P2P RTP/SRTP)

9. Alice hangs up: HANGUP { call_id: "..." }
   Server forwards HANGUP to Bob
   Server broadcasts: USER_STATE_UPDATE (Alice → "Available")
   Server broadcasts: USER_STATE_UPDATE (Bob → "Available")
```

## Implementation Notes

### Protocol Module Structure

The protocol is organized following clean architecture principles:

**`tcp/protocol.rs`**:
- `read_exact_with_retry()`: Handles non-blocking reads with retry logic
- `read_message()`: Reads length + type + payload from stream
- `parse_json_payload()`: UTF-8 validation and JSON parsing
- `parse_message()`: Type-based message deserialization
- `serialize_message()`: Message to JSON bytes
- `write_message()`: Writes formatted message to stream

**Constants**:
```rust
const MAX_MESSAGE_SIZE: u32 = 1024 * 1024; // 1 MB
const RETRY_DELAY_MS: u64 = 10;            // Polling interval
```

### Threading Model
- Main thread: Accepts new TCP connections
- Per-client thread: Handles each connection independently
- Shared state: `Arc<Mutex<Storage>>` for thread-safe access
- Background cleanup: Removes disconnected users

### Error Handling

**`ProtocolError` enum**:
```rust
pub enum ProtocolError {
    Io(io::Error),              // Network/IO errors
    InvalidMessageType(u8),     // Unknown message type
    JsonParse(String),          // JSON parsing errors
    MessageTooLarge(u32),       // Exceeds MAX_MESSAGE_SIZE
}
```

All errors implement `std::error::Error` and `Display` for proper error propagation.

### Broadcasting
When a user state changes, server must send USER_STATE_UPDATE to ALL connected clients except the originating user.

### Cleanup
On disconnect or timeout:
1. Update user state to "Disconnected"
2. If user was in call, send HANGUP to peer
3. Broadcast USER_STATE_UPDATE
4. Remove from connected clients map
