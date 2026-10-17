//! Office-style application chrome. Each part is one module (see `docs/specs/office-chrome.md`).

pub mod title_bar;
pub mod window_edges;

pub use title_bar::{TitleBar, TitleBarResponse, TitleBarSlots, WindowAction, window_command};
