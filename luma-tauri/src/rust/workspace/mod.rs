//! Workspace domain layer.
//!
//! `commands` are the IPC entry points, `layout` persists tool-window state,
//! `metadata` is the on-disk descriptor, `recent` is the recent-workspace list,
//! `store` tracks the loaded workspace per IDE window, and `window` owns IDE
//! window setup.

pub mod commands;
pub mod layout;

mod metadata;
mod recent;
mod store;
mod window;

pub use store::workspace_for_window;
pub use window::free_main_label;
