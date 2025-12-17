//! P2P session configuration
//!
//! This module defines the configuration structure for P2P sessions,
//! including video resolution, codec settings, and network parameters.

/// Configuration for P2P session
#[derive(Debug, Clone)]
pub(crate) struct P2PConfig {
    /// Frame width
    frame_width: u32,
    /// Frame height
    frame_height: u32,
    /// Frames per second
    fps: f64,
    /// Codec bitrate
    codec_bitrate: u32,
    /// Local port
    local_port: u16,
    /// Remote port
    remote_port: u16,
}

impl P2PConfig {
    // Getters
    pub(crate) fn frame_width(&self) -> u32 {
        self.frame_width
    }

    pub(crate) fn frame_height(&self) -> u32 {
        self.frame_height
    }

    pub(crate) fn fps(&self) -> f64 {
        self.fps
    }

    pub(crate) fn codec_bitrate(&self) -> u32 {
        self.codec_bitrate
    }

    pub(crate) fn local_port(&self) -> u16 {
        self.local_port
    }

    pub(crate) fn remote_port(&self) -> u16 {
        self.remote_port
    }

    /// Creates a builder for P2PConfig
    pub(crate) fn builder() -> P2PConfigBuilder {
        P2PConfigBuilder::default()
    }

    /// Calculates the optimal bitrate based on resolution and FPS.
    ///
    /// Uses a quality factor to determine appropriate bitrate:
    /// - 640x480 (VGA): ~1-2 Mbps
    /// - 1280x720 (HD): ~2.5-4 Mbps
    pub(crate) fn calculate_optimal_bitrate(&self) -> u32 {
        let pixels = self.frame_width * self.frame_height;
        let fps_factor = self.fps / 30.0;

        let base_bitrate = (pixels as f64 * 0.15 * fps_factor) as u32;

        base_bitrate.clamp(1_000_000, 10_000_000)
    }

    /// Creates a config with automatic bitrate calculation.
    pub(crate) fn with_auto_bitrate(mut self) -> Self {
        self.codec_bitrate = self.calculate_optimal_bitrate();
        self
    }

    /// Sets the resolution (width and height).
    pub(crate) fn with_resolution(mut self, width: u32, height: u32) -> Self {
        self.frame_width = width;
        self.frame_height = height;
        self
    }

    /// Sets the frames per second.
    pub(crate) fn with_fps(mut self, fps: f64) -> Self {
        self.fps = fps;
        self
    }

    /// Sets the codec bitrate.
    pub(crate) fn with_bitrate(mut self, bitrate: u32) -> Self {
        self.codec_bitrate = bitrate;
        self
    }

    /// Sets the remote port.
    pub(crate) fn with_remote_port(mut self, port: u16) -> Self {
        self.remote_port = port;
        self
    }
}

impl Default for P2PConfig {
    fn default() -> Self {
        Self {
            frame_width: 1280,
            frame_height: 720,
            fps: 30.0,
            codec_bitrate: 5000000, // 5 Mbps
            local_port: 5004,
            remote_port: 5004,
        }
    }
}

/// Builder for P2PConfig
#[derive(Debug, Clone, Default)]
pub(crate) struct P2PConfigBuilder {
    config: P2PConfig,
}

impl P2PConfigBuilder {
    pub(crate) fn local_port(mut self, port: u16) -> Self {
        self.config.local_port = port;
        self
    }

    pub(crate) fn remote_port(mut self, port: u16) -> Self {
        self.config.remote_port = port;
        self
    }

    pub(crate) fn build(self) -> P2PConfig {
        self.config
    }
}
