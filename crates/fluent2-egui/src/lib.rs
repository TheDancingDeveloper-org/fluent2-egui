//! Fluent 2 themes, controls and Office-style application chrome for egui.
//!
//! Not affiliated with or endorsed by Microsoft. See `docs/ARCHITECTURE.md` for the API
//! conventions every component follows.
//!
//! Status: scaffold plus the Fluent type ramp (`foundation::text`).

pub use fluent2_tokens as tokens;

pub mod foundation;
pub mod theme;

/// Install Fluent 2 text styles (and the OS UI font when one can be read).
///
/// The theme-module item extends this to Visuals and `Theme::get`.
pub fn install(ctx: &egui::Context) {
    theme::install(ctx);
}

pub use foundation::text::{TypeRamp, font_id, install_system_fonts, install_text_styles, rich, style};
