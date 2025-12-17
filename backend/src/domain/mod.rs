//! Domain layer - Pure business logic models

/// Type alias for User identifiers
pub type UserId = String;

mod call;
mod call_state;
mod user;
mod user_state;

pub use call::Call;
pub use call_state::CallState;
pub use user::User;
pub use user_state::UserState;
