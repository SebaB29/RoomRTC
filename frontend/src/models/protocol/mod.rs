//! Protocol DTOs
//!
//! Data Transfer Objects for client-server communication.

mod server_message;
mod user_info;

pub use server_message::ServerMessage;
pub use user_info::UserInfo;
