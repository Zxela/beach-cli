//! UI rendering module for Vancouver Beach CLI
//!
//! This module contains all the rendering logic for the terminal user interface,
//! using the ratatui library for TUI components.

pub mod beach_detail;
pub mod beach_list;
pub mod help_overlay;
pub mod plan_trip;

pub use beach_detail::render as render_beach_detail;
pub use beach_list::render_beach_list;
pub use help_overlay::render as render_help_overlay;
pub use plan_trip::render as render_plan_trip;
