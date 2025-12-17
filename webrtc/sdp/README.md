# SDP - Session Description Protocol

Implementation of Session Description Protocol (SDP) according to [RFC 4566](https://datatracker.ietf.org/doc/html/rfc4566).

## Overview

This module provides a complete and type-safe SDP implementation for WebRTC. SDP describes multimedia sessions including codecs, IP addresses, ports, and other parameters needed for peer-to-peer connections.

## Features

- ✅ Complete RFC 4566 parser and serializer
- ✅ Builder pattern for easy construction
- ✅ Automatic validation
- ✅ Type-safe error handling

## Quick Start

### Parse SDP

```rust
use sdp::{SessionDescription, SdpType};

let sdp_str = r#"v=0
o=alice 2890844526 2890844526 IN IP4 host.example.com
s=Video Call
t=0 0
m=audio 49170 RTP/AVP 0
a=rtpmap:0 PCMU/8000"#;

let sdp = SessionDescription::parse(SdpType::Offer, sdp_str)?;
println!("Session: {}", sdp.session_name);
```

### Create SDP with Builder

```rust
use sdp::{
    SessionDescription, SdpType, Origin, Timing, Connection,
    MediaDescription, Attribute
};

let offer = SessionDescription::builder(SdpType::Offer)
    .session_name("Video Conference")
    .origin(Origin {
        username: "user".to_string(),
        session_id: 12345678,
        session_version: 1,
        network_type: "IN".to_string(),
        address_type: "IP4".to_string(),
        unicast_address: "192.168.1.100".to_string(),
    })
    .timing(Timing {
        start_time: 0,
        stop_time: 0,
    })
    .connection(Connection {
        network_type: "IN".to_string(),
        address_type: "IP4".to_string(),
        address: "192.168.1.100".parse().unwrap(),
        ttl: None,
        num_addresses: None,
    })
    .add_attribute(Attribute {
        name: "tool".to_string(),
        value: Some("rusty-webrtc-0.1".to_string()),
    })
    .add_media(MediaDescription {
        media_type: "video".to_string(),
        port: 9,
        protocol: "UDP/TLS/RTP/SAVPF".to_string(),
        formats: vec!["96".to_string(), "97".to_string()],
        connection: None,
        attributes: vec![
            Attribute {
                name: "rtpmap".to_string(),
                value: Some("96 VP8/90000".to_string()),
            },
            Attribute {
                name: "rtpmap".to_string(),
                value: Some("97 H264/90000".to_string()),
            },
            Attribute {
                name: "sendrecv".to_string(),
                value: None,
            },
        ],
    })
    .build()?;

// Convert to string for signaling
let sdp_string = offer.to_string();
```

## SDP Format

```
v=0                                    ← Version
o=alice 2890844526 2890844526 IN IP4   ← Origin
s=Session Name                         ← Session name
t=0 0                                  ← Timing
c=IN IP4 192.0.2.1                     ← Connection
a=ice-ufrag:F7gI                       ← Session attributes
m=audio 54400 RTP/SAVPF 0 96           ← Media description
a=rtpmap:0 PCMU/8000                   ← Media attributes
```

## References

- [RFC 4566 - SDP Protocol](https://datatracker.ietf.org/doc/html/rfc4566)
- [WebRTC SDP Anatomy](https://webrtchacks.com/sdp-anatomy/)
