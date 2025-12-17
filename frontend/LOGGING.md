# Logging Prefix Reference

Internal team reference for structured logging prefixes used across the frontend application.

## Log Prefixes by Module

| Prefix | Module | File Location |
|--------|--------|---------------|
| `[APP]` | Application lifecycle | `src/app/state.rs` |
| `[AUTH]` | Authentication & session | `src/app/handlers/auth_handlers.rs` |
| `[LOBBY]` | Lobby operations | `src/app/handlers/lobby_handlers.rs` |
| `[CALL]` | Call lifecycle | `src/app/handlers/call_handlers.rs`, `src/app/server_handler.rs` |
| `[SIGNALING]` | WebRTC signaling (SDP/ICE) | `src/app/handlers/signaling_handlers.rs`, `src/app/server_handler.rs` |
| `[WEBRTC]` | WebRTC peer connections | `src/app/logic_handler.rs` |
| `[ROOM]` | Room management | `src/app/handlers/room_handlers.rs`, `src/app/logic_handler.rs` |
| `[CAMERA]` | Camera control | `src/app/handlers/camera_handlers.rs` |
| `[VIDEO]` | Video streaming state | `src/app/logic_handler.rs` |
| `[USER_STATE]` | User state updates | `src/app/server_handler.rs` |
| `[SERVER_MSG]` | Server messages | `src/app/server_handler.rs` |
| `[SERVER_ERROR]` | Server errors | `src/app/server_handler.rs` |
| `[UI]` | UI commands | `src/app/ui_handler.rs` |

## Log Levels

- **debug**: UI commands, internal state (verbose)
- **info**: Normal operations, successful flows
- **warn**: Anomalous but non-critical situations
- **error**: Operation failures, network errors
