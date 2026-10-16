//! Gallery page for [`fluent2_egui::controls::SplitButton`] and [`MenuButton`].
//!
//! Compiled by the split-button tests. The gallery crate is not a workspace member yet.

use egui::Ui;
use fluent2_egui::controls::{Appearance, MenuButton, SplitButton};
use fluent2_icons::Icon;

/// Split and menu buttons across the appearances.
pub fn page(ui: &mut Ui) {
    ui.label("Split buttons");
    for appearance in [
        Appearance::Primary,
        Appearance::Secondary,
        Appearance::Outline,
        Appearance::Subtle,
        Appearance::Transparent,
    ] {
        let _ = SplitButton::new(format!("Split {appearance:?}"))
            .appearance(appearance)
            .icon(Icon::Save)
            .show(ui, |ui| {
                ui.label("Save as…");
            });
        let _ = MenuButton::new(format!("Menu {appearance:?}"))
            .appearance(appearance)
            .show(ui, |ui| {
                ui.label("Recent");
            });
    }
    let _ = ui.add_enabled_ui(false, |ui| SplitButton::new("Disabled").show(ui, |_| {}));
}
