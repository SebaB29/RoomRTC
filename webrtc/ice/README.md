# ICE - Interactive Connectivity Establishment

Implementation of ICE (Interactive Connectivity Establishment) according to [RFC 5245](https://datatracker.ietf.org/doc/html/rfc5245).

## Overview

This module provides ICE functionality for WebRTC peer-to-peer connections. ICE handles NAT traversal by gathering, exchanging, and testing network candidates to find the best path for media between peers.

## Features

- ✅ ICE candidate gathering and management
- ✅ Support for multiple candidate types (Host, Srflx, Relay, Prflx)
- ✅ Candidate pair formation and prioritization
- ✅ SDP integration for candidate exchange
- ✅ Builder pattern for easy candidate creation
- ✅ Type-safe error handling
- ✅ Extensible for future STUN/TURN support

## Quick Start

### Create ICE Agent

```rust
use ice::IceAgent;

// Create agent with auto-generated credentials
let mut agent = IceAgent::new();
println!("ufrag: {}", agent.ufrag);
println!("pwd: {}", agent.pwd);

// Or with specific credentials
let agent = IceAgent::with_credentials(
    "a1b2c3d4".to_string(),
    "x9y8z7w6v5u4t3s2r1q0".to_string()
);
```

### Gather Local Candidates

```rust
use ice::{IceAgent, CandidateBuilder, CandidateType};

let mut agent = IceAgent::new();

// Gather host candidates
agent.gather_host_candidates(5000)?;

// Create custom candidate with builder
let candidate = CandidateBuilder::new()
    .foundation("1".to_string())
    .component_id(1)
    .transport("UDP".to_string())
    .address("192.168.1.100".parse().unwrap())
    .port(9000)
    .candidate_type(CandidateType::Host)
    .build()?;

agent.add_local_candidate(candidate)?;
```

### Exchange Candidates via SDP

```rust
use ice::IceAgent;

let mut agent = IceAgent::new();
agent.gather_host_candidates(5000)?;

// Export local candidates to SDP format
let sdp_lines = agent.get_local_candidates_sdp();
for line in &sdp_lines {
    println!("{}", line);
}
// Output example:
// a=candidate:1 1 UDP 2130706431 127.0.0.1 5000 typ host

// Parse and add remote candidates from peer
let remote_attrs = vec![
    "candidate:1 1 UDP 2130706431 192.168.1.50 8000 typ host".to_string(),
];
agent.add_remote_candidates_from_sdp(&remote_attrs)?;
```

## Candidate Types
`
- **Host**: Local network interface address
- **Srflx**: Server Reflexive (from STUN server) - *Prepared for future*
- **Relay**: Relayed through TURN server - *Prepared for future*
- **Prflx**: Peer Reflexive (discovered during checks) - *Prepared for future*

## Candidate Pair Priority

Candidate pairs are sorted by priority according to RFC 5245:

```
pair priority = 2^32 × MIN(G,D) + 2 × MAX(G,D) + (G>D ? 1 : 0)
```

Where G is the controlling agent's candidate priority and D is the controlled agent's candidate priority.

## ICE Candidate Format

```
a=candidate:<foundation> <component-id> <transport> <priority> <address> <port> typ <type> [raddr <rel-addr>] [rport <rel-port>]
```

Example:
```
a=candidate:1 1 UDP 2130706431 192.168.1.100 5000 typ host
```

## References

- [RFC 5245 - ICE Protocol](https://datatracker.ietf.org/doc/html/rfc5245)
- [RFC 8445 - ICE for SDP Offer/Answer](https://datatracker.ietf.org/doc/html/rfc8445)
