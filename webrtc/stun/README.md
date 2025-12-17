# STUN - Session Traversal Utilities for NAT

Implementation of STUN (Session Traversal Utilities for NAT) according to [RFC 5389](https://datatracker.ietf.org/doc/html/rfc5389).

## Overview

This module provides STUN functionality for WebRTC peer-to-peer connections. STUN helps peers behind NAT discover their public addresses (reflexive addresses) needed to establish direct connections.

**This module provides only a STUN client to work with public STUN servers (like Google's).**

## Features

- ✅ RFC 5389 compliant STUN message encoding/decoding
- ✅ STUN client for discovering reflexive addresses
- ✅ Support for MAPPED-ADDRESS and XOR-MAPPED-ADDRESS
- ✅ IPv4 and IPv6 support
- ✅ Builder pattern for message construction
- ✅ Type-safe error handling
- ✅ Works with public STUN servers (Google, etc.)

## Quick Start

### Simple Usage with Helper Function

The easiest way to discover your reflexive address:

```rust
use stun::StunClient;

let servers = &[
    "stun.l.google.com:19302",
    "stun1.l.google.com:19302",
];

let bind_addr = "0.0.0.0:0".parse().unwrap();
let reflexive_addr = StunClient::discover_reflexive_from_servers(bind_addr, servers)?;

println!("Public address: {}", reflexive_addr);
// Output: Public address: 203.0.113.5:54321
```

### Advanced Usage with Direct Control

For more control over DNS resolution and error handling:

```rust
use stun::StunClient;

// Use Google's public STUN server
let bind_addr = "0.0.0.0:0".parse().unwrap();
let server_addr = "stun.l.google.com:19302"
    .to_socket_addrs()?
    .next()
    .ok_or_else(|| std::io::Error::other("DNS failed"))?;

let client = StunClient::new(bind_addr, server_addr)?;
let reflexive_addr = client.get_reflexive_address()?;

println!("Public address: {}", reflexive_addr);
```

### Build STUN Messages

```rust
use stun::{MessageBuilder, MessageType};

let message = MessageBuilder::new(MessageType::Request)
    .random_transaction_id()
    .build()?;

// Send message over UDP socket
let bytes = message.encode();
```

## Integration with ICE

```rust
use stun::StunClient;
use ice::{IceAgent, CandidateBuilder, CandidateType};

// Discover reflexive address via STUN
let client = StunClient::new(
    "0.0.0.0:0".parse().unwrap(),
    "stun.example.com:3478".parse().unwrap()
)?;
let reflexive_addr = client.get_reflexive_address()?;

// Create server reflexive candidate
let srflx = CandidateBuilder::new()
    .foundation("srflx1".to_string())
    .component_id(1)
    .transport("UDP".to_string())
    .address(reflexive_addr.ip())
    .port(reflexive_addr.port())
    .candidate_type(CandidateType::Srflx)
    .build()?;

// Add to ICE agent
let mut agent = IceAgent::new();
agent.add_local_candidate(srflx)?;
```

## STUN Message Format

```
 0                   1                   2                   3
 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1 2 3 4 5 6 7 8 9 0 1
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|0 0|     STUN Message Type     |         Message Length        |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                         Magic Cookie                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                                                               |
|                     Transaction ID (96 bits)                  |
|                                                               |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
|                           Attributes                          |
+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+-+
```

## References

- [RFC 5389 - STUN Protocol](https://datatracker.ietf.org/doc/html/rfc5389)
- [RFC 5245 - ICE](https://datatracker.ietf.org/doc/html/rfc5245)
- [RFC 8445 - ICE (revised)](https://datatracker.ietf.org/doc/html/rfc8445)
