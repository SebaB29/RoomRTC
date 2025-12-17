use std::path::PathBuf;

/// Commands initiated by the UI (View -> Controller)
/// These are "requests" to perform actions.
#[derive(Debug, Clone)]
pub enum UiCommand {
    // --- Authentication ---
    LoginWithPassword {
        username: String,
        password: String,
    },
    Register {
        username: String,
        password: String,
    },
    Logout,

    // --- Lobby ---
    CallUser(String), // to_user_id
    AcceptCall {
        call_id: String,
        caller_id: String,
        caller_name: String,
    },
    DeclineCall(String), // call_id

    // --- Room ---
    ToggleCamera,
    ToggleMute,
    UpdateCameraSettings(i32, f64), // device_id, fps
    ExitRoom,

    // --- File Transfer ---
    /// Open file picker dialog to send a file
    SendFile,
    /// File was selected from picker
    SendFileSelected(PathBuf),
    /// Accept an incoming file transfer
    AcceptFileTransfer {
        transfer_id: u64,
        save_path: PathBuf,
    },
    /// Reject an incoming file transfer  
    RejectFileTransfer {
        transfer_id: u64,
    },
    CancelFileTransfer {
        transfer_id: u64,
    },
}
