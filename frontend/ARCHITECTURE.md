# Frontend Architecture

## Overview

This document describes the architectural patterns and design principles used in the Roome frontend application.

For development guides and how-to instructions, see [DEVELOPMENT.md](DEVELOPMENT.md).

## MVC Pattern

This project uses an **MVC architecture adapted for egui** with clear separation of responsibilities:

```
┌─────────────────────────────────────────────────────┐
│                   USER INTERFACE                    │
│                  (egui rendering)                   │
└──────────────────────┬──────────────────────────────┘
                       │
                       ▼
            ┌──────────────────┐
            │    UiCommand     │ ◄─── View emits commands
            └─────────┬────────┘
                      │
                      ▼
            ┌──────────────────┐
            │   CONTROLLER     │
            │ (App - handlers) │
            │                  │
            │  • Process UI    │
            │  • Update state  │
            │  • Send commands │
            └────┬────────┬────┘
                 │        │
       ┌─────────┘        └───────────┐
       ▼                              ▼
┌──────────────┐           ┌──────────────────┐
│    MODEL     │           │  LOGIC THREAD    │
│ (App state)  │           │  (background)    │
│              │           │                  │
│ • rooms      │           │  • WebRTC        │
│ • user       │           │  • Camera        │
│ • setup      │           │  • Frames        │
└──────────────┘           └─────────┬────────┘
       ▲                             │
       │                             ▼
       │                  ┌──────────────────┐
       │                  │   LogicEvent     │
       └──────────────────┤   (responses)    │
                          └──────────────────┘
```

## Directory Structure

```
frontend/src/
├── main.rs                    # Entry point
├── app/                       # CONTROLLER + MODEL
│   ├── mod.rs                 # Module exports
│   ├── state.rs               # Application state
│   ├── ui_handler.rs          # UI command processor
│   ├── logic_handler.rs       # Logic event handler
│   ├── server_handler.rs      # Server message handler
│   └── handlers/              # Domain-specific handlers
│       ├── auth_handlers.rs   # Authentication logic
│       ├── call_handlers.rs   # Call lifecycle
│       ├── camera_handlers.rs # Camera control
│       ├── lobby_handlers.rs  # Lobby operations
│       ├── room_handlers.rs   # Room management
│       └── signaling_handlers.rs # WebRTC signaling
├── models/                    # Data structures
│   └── room.rs                # Room, Participant
├── pages/                     # VIEW: Main pages
│   ├── login.rs               # Login/Register screen
│   ├── lobby.rs               # User list and call initiation
│   └── call.rs                # Active video call
├── components/                # Reusable UI components
│   ├── button.rs
│   ├── card.rs
│   ├── dialog.rs
│   ├── user_card.rs
│   └── ...
├── events/                    # Event system
│   ├── ui_command.rs          # VIEW → CONTROLLER
│   ├── logic_command.rs       # CONTROLLER → LOGIC THREAD
│   ├── logic_event.rs         # LOGIC THREAD → CONTROLLER
│   └── server_event.rs        # SERVER → CONTROLLER
├── logic/                     # Background logic thread
│   └── mod.rs                 # WebRTC and media handling
├── infrastructure/            # External integrations
│   └── server_connection.rs   # TCP connection to backend
├── context/                   # Shared context
│   └── user_context.rs        # User session state
└── config/                    # Configuration
    └── app_config.rs          # Application settings
```

## Data Flow

### 1. View → Controller (UiCommand)

```rust
// In a page/component (VIEW)
if Button::new("Start Call").show(ui).clicked() {
    return Some(UiCommand::InitiateCall { user_id: target_id });
}

// Controller processes it
impl App {
    fn handle_ui_command(&mut self, command: UiCommand) {
        match command {
            UiCommand::InitiateCall { user_id } => {
                self.handle_initiate_call(user_id);
            }
        }
    }
}
```

### 2. Controller → Logic Thread (LogicCommand)

```rust
// Send heavy work to background
self.logic_cmd_tx.send(LogicCommand::GenerateOffer {
    local_port,
    remote_port,
}).unwrap();
```

### 3. Logic Thread → Controller (LogicEvent)

```rust
// Thread responds with results
evt_tx.send(LogicEvent::OfferGenerated(sdp, conn)).unwrap();

// Controller updates model
impl App {
    fn handle_logic_event(&mut self, event: LogicEvent) {
        match event {
            LogicEvent::OfferGenerated(sdp, conn) => {
                self.room_setup.local_sdp_offer = Some(sdp);
            }
        }
    }
}
```

## Application State

```rust
pub struct App {
    // Navigation
    user_context: UserContext,
    current_page: Page,

    // Data
    rooms: HashMap<String, RoomData>,
    room_states: HashMap<String, RoomState>,

    // WebRTC
    room_setup: RoomSetup,
    webrtc_connection: Option<WebRtcConnection>,

    // Communication channels
    logic_cmd_tx: Sender<LogicCommand>,
    logic_evt_rx: Receiver<LogicEvent>,
}
```

## Logic Thread Pattern

The logic thread handles heavy operations asynchronously:

- **Non-blocking UI**: Long operations run in background
- **Thread-safe**: Uses `Arc<Mutex<>>` for shared WebRTC connection
- **Event-driven**: Communicates results via `LogicEvent`

### Example Flow:

```rust
// 1. User clicks "Generate Offer"
UiCommand::GenerateOffer

// 2. Controller sends to logic thread
self.logic_cmd_tx.send(LogicCommand::GenerateOffer {
    local_port: 5004,
    remote_port: 5004,
}).unwrap();

// 3. Thread does heavy work
let mut webrtc = WebRtcConnection::new(config, logger)?;
let offer = webrtc.create_offer()?;

// 4. Thread sends result back
evt_tx.send(LogicEvent::OfferGenerated(offer, webrtc)).unwrap();

// 5. Controller updates model
self.room_setup.local_sdp_offer = Some(sdp);

// 6. View re-renders automatically
```

## Key Principles

### 1. Separation of Concerns
- **View**: Renders and emits commands (no state mutation)
- **Controller**: Processes commands (no rendering)
- **Model**: Holds data (no logic)
- **Logic Thread**: Heavy work (no direct model access)

### 2. Unidirectional Flow
```
User → View → UiCommand → Controller → Model
                ↓
           LogicCommand → Thread → LogicEvent
```

### 3. Immutable Views
- Views receive `&data` references
- Only controller mutates state
- Pure functions returning commands

### 4. Event-Driven
- Explicit events for all actions
- Easy debugging and testing

## Resources

- [egui documentation](https://docs.rs/egui/)
- [WebRTC specification](https://webrtc.org/)
- [Rust async book](https://rust-lang.github.io/async-book/)
