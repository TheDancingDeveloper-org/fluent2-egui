//! One sample of each Fluent type-ramp step.

use fluent2_egui::foundation::text::{TypeRamp, rich};

use super::Page;

pub struct TypographyPage;

impl Page for TypographyPage {
    fn name(&self) -> &'static str {
        "Typography"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Type ramp");
        ui.label("Each TypeRamp step, drawn with the style install_text_styles registers.");
        ui.add_space(8.0);
        for step in TypeRamp::ALL {
            ui.label(rich(format!("{}  The quick brown fox", step.name()), step));
            ui.add_space(4.0);
        }
    }
}
