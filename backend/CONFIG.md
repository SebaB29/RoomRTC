# RoomRTC Server Configuration

## Configuration File

The server uses a JSON file for configuration: `server_config.json`

## Complete Structure

```json
{
  "server": {
    "bind_address": "0.0.0.0",
    "port": 8080,
    "max_connections": 100,
    "enable_tls": false,
    "pkcs12_path": null,
    "pkcs12_password": ""
  },
  "logging": {
    "log_file_path": "roomrtc-server.log",
    "log_level": "info",
    "enable_console": true,
    "enable_file": true
  },
  "webrtc": {
    "stun_servers": [
      "stun.l.google.com:19302",
      "stun1.l.google.com:19302"
    ],
    "turn_servers": [],
    "rtp_port_range_start": 5000,
    "rtp_port_range_end": 6000,
    "ice_gathering_timeout_ms": 5000,
    "connection_timeout_ms": 10000
  },
  "security": {
    "session_timeout_secs": 3600,
    "max_sessions_per_user": 5,
    "enable_rate_limiting": true,
    "max_requests_per_minute": 60
  },
  "persistence": {
    "enable_persistence": true,
    "data_directory": "./data",
    "auto_save_interval_secs": 300
  }
}
```

## Field Descriptions

### Server Configuration (`server`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `bind_address` | String | `"127.0.0.1"` | IP address where server listens. Use `0.0.0.0` for all interfaces, `127.0.0.1` for local only |
| `port` | Number | `8080` | TCP port for signaling server |
| `max_connections` | Number | `100` | Maximum concurrent connections |
| `enable_tls` | Boolean | `false` | Enable TLS for secure TCP connections |
| `pkcs12_path` | String | `null` | Path to PKCS#12 file (.pfx/.p12) containing certificate and private key |
| `pkcs12_password` | String | `""` | Password for PKCS#12 file (can be empty) |

### Logging Configuration (`logging`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `log_file_path` | String | `"roomrtc-server.log"` | Log file path |
| `log_level` | String | `"info"` | Log level: `debug`, `info`, `warn`, `error` |
| `enable_console` | Boolean | `true` | Show logs in console (stdout) |
| `enable_file` | Boolean | `true` | Save logs to file |

### WebRTC Configuration (`webrtc`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `stun_servers` | Array[String] | Google STUN | List of STUN servers for NAT traversal |
| `turn_servers` | Array[Object] | `[]` | List of TURN servers (see structure below) |
| `rtp_port_range_start` | Number | `5000` | Start port of RTP range for media |
| `rtp_port_range_end` | Number | `6000` | End port of RTP range for media |
| `ice_gathering_timeout_ms` | Number | `5000` | Timeout in ms for ICE candidate gathering |
| `connection_timeout_ms` | Number | `10000` | Timeout in ms to establish WebRTC connection |

#### TURN Server Configuration

```json
{
  "url": "turn:turnserver.example.com:3478",
  "username": "user",
  "credential": "password"
}
```

| Field | Type | Description |
|-------|------|-------------|
| `url` | String | TURN server URL (format: `turn:host:port`) |
| `username` | String | Username for TURN authentication |
| `credential` | String | Password for TURN authentication |

### Security Configuration (`security`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `session_timeout_secs` | Number | `3600` | Session lifetime in seconds (1 hour) |
| `max_sessions_per_user` | Number | `5` | Maximum concurrent sessions per user |
| `enable_rate_limiting` | Boolean | `true` | Enable request rate limiting |
| `max_requests_per_minute` | Number | `60` | Maximum requests allowed per minute per IP |

### Persistence Configuration (`persistence`)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable_persistence` | Boolean | `true` | Enable persistent data storage |
| `data_directory` | String | `"./data"` | Directory for persistent data |
| `auto_save_interval_secs` | Number | `300` | Auto-save interval in seconds (5 minutes) |

## Default Values

All fields are **optional**. If a field is not present in the JSON, its default value will be used.

### Example: Minimal Configuration

```json
{
  "server": {
    "bind_address": "0.0.0.0",
    "port": 8080
  }
}
```

All other fields will use their default values.

## Loading Configuration

The server automatically searches for `server_config.json` in the current directory.

You can also specify a different file:

```bash
./roomrtc-server /path/to/custom_config.json
```

## Configuration Examples

### Production (with TLS)

```json
{
  "server": {
    "bind_address": "0.0.0.0",
    "port": 8443,
    "enable_tls": true,
    "pkcs12_path": "/etc/ssl/certs/server.p12",
    "pkcs12_password": "your_secure_password"
  },
  "logging": {
    "log_level": "warn"
  },
  "security": {
    "enable_rate_limiting": true,
    "max_requests_per_minute": 100
  }
}
```

### Local Development

```json
{
  "server": {
    "bind_address": "127.0.0.1",
    "port": 8080
  },
  "logging": {
    "log_level": "debug",
    "enable_console": true
  },
  "persistence": {
    "enable_persistence": false
  }
}
```

### Con TURN Server

```json
{
  "webrtc": {
    "stun_servers": [
      "stun.l.google.com:19302"
    ],
    "turn_servers": [
      {
        "url": "turn:turnserver.example.com:3478",
        "username": "myuser",
        "credential": "mypassword"
      }
    ]
  }
}
```

## Notes

- **Architecture**: TCP server with custom binary protocol (not HTTP)
- **TLS**: Uses PKCS#12 format that packages certificate and private key in a single file
- **PKCS#12 Generation**: 
  ```bash
  # From PEM certificate and private key
  openssl pkcs12 -export -out server.p12 -inkey server.key -in server.crt
  ```
- **STUN Servers**: Required for NAT traversal on most networks
- **TURN Servers**: Optional but recommended for networks with restrictive NAT
- **RTP Port Range**: Must have enough ports for expected number of concurrent connections
- **Persistence**: Saves users to plain text file `users.txt`
- **Rate Limiting**: Helps prevent abuse and DoS attacks
- **Without TLS**: Only use with TLS-terminating proxy (e.g., Tailscale Funnel)
