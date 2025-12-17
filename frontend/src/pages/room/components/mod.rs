//! Room UI Components
//!
//! This module contains reusable UI components for the Room page.
//! Each component is in its own file for better organization.

mod controls;
mod header;
mod sidebar;
mod video_grid;
mod video_placeholder;

pub use controls::render_controls;
pub use header::render_header;
pub use sidebar::{SIDEBAR_CONSTANT, render_settings_sidebar};
pub use video_grid::render_video_grid;
