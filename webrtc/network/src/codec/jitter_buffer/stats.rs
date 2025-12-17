//! Jitter buffer statistics

/// Statistics for jitter buffer
#[derive(Debug, Clone)]
pub struct JitterBufferStats {
    pub playout_delay_ms: f64,
    /// Estimated jitter (milliseconds)
    pub jitter_ms: f64,
    pub buffer_size: usize,
    pub packets_played: u64,
    /// Packets discarded (late arrivals)
    pub packets_late: u64,
    /// Packets discarded (duplicates)
    pub packets_duplicate: u64,
    /// Number of underruns (buffer empty when playout requested)
    pub underruns: u64,
}

impl Default for JitterBufferStats {
    fn default() -> Self {
        Self {
            playout_delay_ms: 0.0,
            jitter_ms: 0.0,
            buffer_size: 0,
            packets_played: 0,
            packets_late: 0,
            packets_duplicate: 0,
            underruns: 0,
        }
    }
}
