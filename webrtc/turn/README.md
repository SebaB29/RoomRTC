# TURN - Traversal Using Relays around NAT

Implementation of TURN (Traversal Using Relays around NAT) according to [RFC 5766](https://datatracker.ietf.org/doc/html/rfc5766).

## Overview

This module provides TURN functionality for WebRTC peer-to-peer connections. TURN acts as a relay server when direct P2P connections fail due to restrictive NAT or firewall configurations. It extends STUN with relay capabilities to ensure connectivity in challenging network scenarios.

**TURN is used as a last resort when:**
- Both peers are behind symmetric NAT
- Firewall blocks direct P2P connections
- STUN alone cannot establish connectivity

## Features

- ✅ RFC 5766 compliant TURN message encoding/decoding
- ✅ TURN client for allocating relay addresses
- ✅ Allocate/Refresh relay allocations
- ✅ CreatePermission for peer authorization
- ✅ Send indications for relayed data
- ✅ ChannelBind for optimized data transfer
- ✅ XOR address encoding/decoding (RFC 5389)
- ✅ Username authentication support
- ✅ Type-safe error handling with detailed variants

## Quick Start

### Allocate Relay Address

```rust
use turn::TurnClient;

// Connect to TURN server
let turn_server = "turn.example.com:3478".parse().unwrap();
let mut client = TurnClient::new(turn_server, "username".to_string())?;

// Allocate relay address
let relay_addr = client.allocate()?;
println!("Relay address: {}", relay_addr);
// Output: Relay address: 198.51.100.1:54321
```

### Create Permission and Send Data

```rust
use turn::TurnClient;

let mut client = TurnClient::new(
    "turn.example.com:3478".parse().unwrap(),
    "user123".to_string(),
)?;

// Allocate relay
let relay_addr = client.allocate()?;

// Grant permission for peer to send data
let peer_addr = "203.0.113.1:8000".parse().unwrap();
client.create_permission(peer_addr)?;

// Send data through relay
client.send(b"Hello, peer!", peer_addr)?;

println!("Data sent successfully via relay address: {}", relay_addr);
```

### Refresh Allocation

```rust
use turn::TurnClient;

let mut client = TurnClient::new(
    "turn.example.com:3478".parse().unwrap(),
    "username".to_string(),
)?;

// Allocate relay
client.allocate()?;

// Check if refresh is needed (within 60 seconds of expiry)
if client.needs_refresh() {
    client.refresh(600)?; // Refresh for 600 seconds (10 minutes)
    println!("Allocation refreshed");
}
```

### Channel Binding (Optimized)

```rust
use turn::TurnClient;

let mut client = TurnClient::new(
    "turn.example.com:3478".parse().unwrap(),
    "username".to_string(),
)?;

client.allocate()?;

let peer_addr = "203.0.113.1:8000".parse().unwrap();
client.create_permission(peer_addr)?;

// Bind channel for optimized transmission
// Channel numbers must be in range 0x4000-0x7FFF
let channel_number = 0x4000;
client.channel_bind(peer_addr, channel_number)?;

println!("Channel {} bound to {}", channel_number, peer_addr);
// Note: Actual channel data transmission requires additional implementation
```

## TURN Protocol Flow

```
Client                    TURN Server                 Peer
  |                            |                        |
  |--Allocate Request--------->|                        |
  |<--Allocate Response--------|                        |
  |   (Relayed Address)        |                        |
  |                            |                        |
  |--CreatePermission--------->|                        |
  |<--CreatePermission OK------|                        |
  |                            |                        |
  |--Send Indication---------->|                        |
  |   (data for peer)          |----Data Indication---->|
  |                            |                        |
  |                            |<---Data from peer------|
  |<--Data Indication----------|                        |
  |                            |                        |
  |--Refresh Request---------->|                        |
  |<--Refresh Response---------|                        |
```

## Integration with ICE

```rust
use turn::TurnClient;
use ice::{IceAgent, CandidateBuilder, CandidateType};

// Allocate relay address via TURN
let mut turn_client = TurnClient::new(
    "turn.example.com:3478".parse().unwrap(),
    "username".to_string(),
)?;
let relay_addr = turn_client.allocate()?;

// Create relay candidate
let relay = CandidateBuilder::new()
    .foundation("relay1".to_string())
    .component_id(1)
    .transport("UDP".to_string())
    .address(relay_addr.ip())
    .port(relay_addr.port())
    .candidate_type(CandidateType::Relay)
    .build()?;

// Add to ICE agent
let mut agent = IceAgent::new();
agent.add_local_candidate(relay)?;
```

## TURN Message Format

TURN extends STUN messages with additional attributes:

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|0 0|   TURN Message Type      |         Message Length        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         Magic Cookie                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                     Transaction ID (96 bits)                  |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                        TURN Attributes                        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

## TURN Attributes

| Attribute              | Code   | Description                          |
|------------------------|--------|--------------------------------------|
| CHANNEL-NUMBER         | 0x000C | Channel number for binding           |
| LIFETIME               | 0x000D | Allocation lifetime in seconds       |
| XOR-PEER-ADDRESS       | 0x0012 | Peer's transport address (XOR-ed)    |
| DATA                   | 0x0013 | Application data being relayed       |
| XOR-RELAYED-ADDRESS    | 0x0016 | Allocated relay address (XOR-ed)     |
| REQUESTED-TRANSPORT    | 0x0019 | Transport protocol (UDP=17, TCP=6)   |
| DONT-FRAGMENT          | 0x001A | Request DF flag on IP packets        |
| RESERVATION-TOKEN      | 0x0022 | Token for allocation reservation     |

## Message Types

### Requests
- **Allocate**: Request relay address allocation
- **Refresh**: Extend allocation lifetime
- **CreatePermission**: Authorize peer to send data
- **ChannelBind**: Optimize data transfer with channel

### Indications
- **Send**: Send data to peer through relay
- **Data**: Receive data from peer through relay

## Security

TURN supports authentication mechanisms:

```rust
use turn::TurnClient;

// Client uses username for STUN authentication
let client = TurnClient::new(
    "turn.example.com:3478".parse().unwrap(),
    "username".to_string(),
)?;

// Username is included in all requests
// Full MESSAGE-INTEGRITY (HMAC-SHA1) support is planned for future releases
```

**Note**: Current implementation includes username-based authentication. Full HMAC-SHA1 
MESSAGE-INTEGRITY support is planned for production use.

## Error Handling

The TURN client provides detailed error types for different failure scenarios:

```rust
use turn::{TurnClient, TurnError};

let mut client = TurnClient::new(
    "turn.example.com:3478".parse().unwrap(),
    "username".to_string(),
)?;

match client.allocate() {
    Ok(relay_addr) => println!("Success: {}", relay_addr),
    Err(TurnError::AllocationFailed(msg)) => println!("Allocation failed: {}", msg),
    Err(TurnError::AllocationQuotaReached) => println!("Server quota exceeded"),
    Err(TurnError::InsufficientCapacity) => println!("Server capacity reached"),
    Err(TurnError::Timeout) => println!("Server not responding"),
    Err(TurnError::InvalidResponse) => println!("Invalid response from server"),
    Err(e) => println!("Error: {:?}", e),
}
```

### Available Error Types

- `AllocationFailed` - Server rejected allocation
- `PermissionFailed` - Permission creation failed
- `ChannelBindFailed` - Channel binding failed
- `RefreshFailed` - Refresh request failed
- `NoAllocation` - Operation requires active allocation
- `AllocationQuotaReached` - Server quota exceeded
- `InsufficientCapacity` - Server capacity reached
- `Timeout` - Network timeout
- `InvalidResponse` - Malformed server response
- `AttributeError` - Attribute parsing error
- `Io` - Network I/O error

## References

- [RFC 5766 - TURN Protocol](https://datatracker.ietf.org/doc/html/rfc5766)
- [RFC 5389 - STUN Protocol](https://datatracker.ietf.org/doc/html/rfc5389)
- [RFC 5245 - ICE](https://datatracker.ietf.org/doc/html/rfc5245)
- [RFC 8445 - ICE (revised)](https://datatracker.ietf.org/doc/html/rfc8445)
