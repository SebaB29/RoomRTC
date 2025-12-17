pub mod lobby;
pub mod login;
pub mod room;

pub use lobby::Lobby;
pub use login::Login;
pub use room::Room;

/// Page enum to represent different views
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Login,
    Lobby,
    Room,
}
