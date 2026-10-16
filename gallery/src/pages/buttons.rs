//! Gallery page for [`fluent2_egui::controls::Button`].
//!
//! `docs/ARCHITECTURE.md` puts the gallery crate at `gallery` and says it comes later, so this
//! is not a workspace member yet. The button tests compile this page.

use egui::Ui;
use fluent2_egui::controls::{Appearance, Button, ButtonSize, CompoundButton, Shape, ToggleButton};
use fluent2_icons::Icon;

const APPEARANCES: [Appearance; 5] = [
    Appearance::Primary,
    Appearance::Secondary,
    Appearance::Outline,
    Appearance::Subtle,
    Appearance::Transparent,
];

const SIZES: [ButtonSize; 3] = [ButtonSize::Small, ButtonSize::Medium, ButtonSize::Large];

/// One row per appearance, one button per size.
pub fn page(ui: &mut Ui) {
    ui.label("Buttons");
    for appearance in APPEARANCES {
        ui.horizontal(|ui| {
            for size in SIZES {
                let _ = ui.add(
                    Button::new(format!("{appearance:?} {size:?}"))
                        .appearance(appearance)
                        .size(size)
                        .icon(Icon::Save),
                );
            }
        });
    }
    ui.horizontal(|ui| {
        let _ = ui.add(Button::new("Square").shape(Shape::Square));
        let _ = ui.add(Button::new("Circular").shape(Shape::Circular));
        let _ = ui.add(Button::icon_only(Icon::Dismiss, "Close"));
        let _ = ui.add_enabled(false, Button::new("Disabled"));
    });
    ui.label("Toggle and compound");
    ui.horizontal(|ui| {
        let mut bold = true;
        let _ = ui.add(ToggleButton::new(&mut bold, "Bold").appearance(Appearance::Primary));
        let mut italic = false;
        let _ = ui.add(ToggleButton::new(&mut italic, "Italic"));
    });
    let _ = ui.add(
        CompoundButton::new("Meeting")
            .secondary_content("Starts in 5 minutes")
            .icon(Icon::Save),
    );
}
