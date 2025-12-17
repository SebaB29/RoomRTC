//! Video-specific constants
//!
//! Constants for H.264 NAL parsing, YUV processing, and video codec parameters.

/// H.264 NAL unit start codes
pub mod h264 {
    /// 4-byte NAL start code (0x00 0x00 0x00 0x01)
    pub const NAL_START_CODE_4: [u8; 4] = [0x00, 0x00, 0x00, 0x01];
    /// 3-byte NAL start code (0x00 0x00 0x01)
    pub const NAL_START_CODE_3: [u8; 3] = [0x00, 0x00, 0x01];
    /// NAL unit type mask (lower 5 bits)
    pub const NAL_TYPE_MASK: u8 = 0x1F;
    /// NAL unit type: Non-IDR coded slice
    pub const NAL_TYPE_NON_IDR: u8 = 1;
    /// NAL unit type: IDR coded slice (keyframe)
    pub const NAL_TYPE_IDR: u8 = 5;
    /// NAL unit type: SPS (Sequence Parameter Set)
    pub const NAL_TYPE_SPS: u8 = 7;
    /// NAL unit type: PPS (Picture Parameter Set)
    pub const NAL_TYPE_PPS: u8 = 8;
    /// Length of 4-byte start code
    pub const START_CODE_4_LEN: usize = 4;
    /// Length of 3-byte start code
    pub const START_CODE_3_LEN: usize = 3;
}

/// YUV plane calculation constants
pub mod yuv {
    /// YUV420P U and V plane size divisor (Y_size / 4)
    pub const UV_PLANE_DIVISOR: usize = 4;
    /// YUV420P total plane count (Y, U, V)
    pub const PLANE_COUNT: usize = 3;
    /// Y plane index in FFmpeg frame
    pub const Y_PLANE_INDEX: usize = 0;
    /// U plane index in FFmpeg frame
    pub const U_PLANE_INDEX: usize = 1;
    /// V plane index in FFmpeg frame
    pub const V_PLANE_INDEX: usize = 2;
}
