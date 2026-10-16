//! Theme installation.
//!
//! `Theme` (Visuals, spacing, `Theme::get`) lands with the theme-module item. This module's
//! `install` currently installs the type ramp and the OS UI font; that item extends it.

use crate::foundation::text;

/// Install Fluent 2 text styles and, where available, the OS UI font.
///
/// Safe to call more than once: text styles are overwritten in place, and a font already
/// registered under the same name is left as it is.
pub fn install(ctx: &egui::Context) {
    text::install_system_fonts(ctx);
    text::install_text_styles(ctx);
}
