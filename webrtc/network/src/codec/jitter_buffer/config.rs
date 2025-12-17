//! Jitter buffer configuration

/// Configuration for timestamp-based jitter buffer
#[derive(Debug, Clone)]
pub struct JitterBufferConfig {
    pub clock_rate: u32,
    pub min_delay_frames: u32,
    pub max_delay_frames: u32,
    /// Target jitter (milliseconds) - buffer grows to accommodate this
    pub target_jitter_ms: f64,
    /// Maximum buffer size (packets)
    pub max_capacity: usize,
    /// Adaptive delay adjustment speed (0.0 - 1.0)
    pub adaptation_speed: f64,
    /// Ultra-low latency mode: release packets immediately if in order (for local/LAN)
    pub ultra_low_latency: bool,
}

impl Default for JitterBufferConfig {
    fn default() -> Self {
        Self {
            clock_rate: 90000,        // Standard video clock rate
            min_delay_frames: 3,      // Minimum 3 frame times
            max_delay_frames: 10,     // Maximum 10 frame times
            target_jitter_ms: 30.0,   // Target 30ms jitter accommodation
            max_capacity: 200,        // Max 200 packets in buffer
            adaptation_speed: 0.1,    // Slow adaptation (10% per adjustment)
            ultra_low_latency: false, // Default: normal jitter buffering
        }
    }
}
