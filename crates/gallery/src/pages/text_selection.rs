//! Text selection across read-only text: a drag or Shift+arrows select it, Ctrl+C copies it, and
//! the button beside it keeps working.

use fluent2_egui::Card;
use fluent2_egui::controls::{Appearance, Button};

use super::Page;

#[derive(Default)]
pub struct TextSelectionPage {
    saves: u32,
}

impl Page for TextSelectionPage {
    fn name(&self) -> &'static str {
        "Text selection"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Text selection");
        ui.label("Drag across the text below, or click in it and press Shift+Arrow, then Ctrl+C.");
        ui.label("The selection runs across the separate labels in the card. The button still works.");
        ui.add_space(8.0);
        Card::new().id_salt("text selection card").show(ui, |ui| {
            ui.label("Summary");
            ui.label(
                "The first paragraph and the second are separate labels, so one selection can span both.",
            );
            ui.label("Copy it with Ctrl+C after you select.");
            ui.add_space(8.0);
            if ui
                .add(Button::new("Save").appearance(Appearance::Primary))
                .clicked()
            {
                self.saves += 1;
            }
        });
        ui.add_space(4.0);
        ui.label(format!("Save clicked {} times", self.saves));
    }
}
