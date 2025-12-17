//! Common constants shared across media modules

/// Logging intervals for frame processing
pub mod logging {
    /// Log progress every N frames (camera capture)
    pub const CAMERA_LOG_INTERVAL: u64 = 1000;
    /// Log progress every N frames (audio capture)
    pub const AUDIO_LOG_INTERVAL: u64 = 1000;
    /// Log progress every N frames (encoder)
    pub const ENCODER_LOG_INTERVAL: u64 = 60;
    /// Log progress every N frames (decoder)
    pub const DECODER_LOG_INTERVAL: u64 = 60;
}
