//! Fluent surfaces. One file per surface; see `docs/ARCHITECTURE.md`.

mod dialog;
mod menu;

pub use dialog::{CloseReason, Dialog, DialogActions, DialogKind, DialogResult};
pub use menu::{MenuItem, context_menu, divider, group_header, menu_frame, submenu};
