//! Application Module - MVU Controller
//!
//! This module implements the Controller layer of the MVU architecture.
//! It coordinates between the view layer (pages) and the background logic thread.
//!
//! # Structure
//!
//! - `state.rs`: Application state definition and MVU loop
//! - `ui_handler.rs`: Command dispatcher for UI actions
//! - `handlers/`: Domain-specific UI command handlers
//!   - `room_handlers.rs`: Room creation, joining, exit
//!   - `webrtc_handlers.rs`: SDP offer/answer, connection setup
//!   - `camera_handlers.rs`: Camera operations
//! - `logic_handler.rs`: Processes events from background thread
//!
//! # Communication Flow
//!
//! ```text
//! View (pages) --> UiCommand --> ui_handler --> handlers/* --> State mutation
//!                                                          \--> LogicCommand --> Logic thread
//!
//! Logic thread --> LogicEvent --> logic_handler --> State update (textures, connection)
//! ```

mod handlers;
mod logic_handler;
mod server_handler;
mod state;
mod ui_handler;

pub use state::App;
