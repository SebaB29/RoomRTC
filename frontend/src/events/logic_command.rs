use crate::models::Participant;
use std::path::PathBuf;

/// Logic commands sent from UI thread to Logic thread
/// WebRTC connections are managed internally by the Logic thread
#[derive(Debug)]
pub enum LogicCommand {
    /// Generate a WebRTC offer (creates new connection in logic thread)
    GenerateOffer,

    /// Generate a WebRTC answer from remote offer (creates new connection in logic thread)
    GenerateAnswer {
        offer_sdp: String,
    },

    /// Process remote answer to complete connection setup
    ProcessAnswer {
        answer_sdp: String,
    },

    /// Add remote ICE candidate to WebRTC connection
    AddIceCandidate {
        candidate: String,
        sdp_mid: String,
        sdp_mline_index: u16,
    },

    // --- Room ---
    /// Start WebRTC connection with media threads (uses connection from logic thread)
    StartConnection {
        participant: Participant,
    },
    StartCamera {
        device_id: i32,
        fps: f64,
    },
    StopCamera,
    StartAudio {
        sample_rate: u32,
        channels: u32,
    },
    ToggleMute,
    ClearVideoBuffers,
    SendDisconnect {
        is_owner: bool,
    },
    StopConnection,

    // --- File Transfer ---
    /// Send a file to the remote peer
    SendFile {
        path: PathBuf,
    },
    /// Accept an incoming file transfer
    AcceptFileTransfer {
        transfer_id: u64,
        save_path: PathBuf,
    },
    /// Reject an incoming file transfer
    RejectFileTransfer {
        transfer_id: u64,
        reason: String,
    },
    /// Cancel an ongoing file transfer
    CancelFileTransfer {
        transfer_id: u64,
    },
}
