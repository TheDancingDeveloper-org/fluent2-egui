//! Fluent 2 themes, controls and Office-style application chrome for egui.
//!
//! Not affiliated with or endorsed by Microsoft. See `docs/ARCHITECTURE.md` for the API
//! conventions every component follows.

#![warn(missing_docs)]

pub use fluent2_tokens as tokens;

pub mod foundation;
pub mod theme;

pub use foundation::text::{TypeRamp, font_id, install_system_fonts, install_text_styles, rich, style};
pub use theme::{Theme, ThemeKind};

/// Install the default Fluent theme, including text styles and the OS UI font when one can be
/// read.
pub fn install(ctx: &egui::Context) {
    theme::install(ctx, &Theme::default());
}

/// Names a caller usually wants in scope.
pub mod prelude {
    pub use crate::{Theme, ThemeKind};
}
