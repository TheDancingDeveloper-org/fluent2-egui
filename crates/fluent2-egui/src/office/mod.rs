//! Office-style application chrome. Each part is one module (see `docs/specs/office-chrome.md`).

use std::hash::{DefaultHasher, Hash, Hasher};

pub mod backstage;
pub mod command_bar;
pub mod keytips;
pub mod qat;
pub mod status_bar;
pub mod title_bar;
pub mod window_edges;

pub mod ribbon;
pub use title_bar::{TitleBar, TitleBarResponse, TitleBarSlots, WindowAction, window_command};

/// An egui id from any hashable caller value. Only `Hash` is required, so the public
/// constructors take `impl Hash` as the spec says.
fn chrome_id(value: impl Hash) -> egui::Id {
    let mut hasher = DefaultHasher::new();
    value.hash(&mut hasher);
    egui::Id::new(hasher.finish())
}
