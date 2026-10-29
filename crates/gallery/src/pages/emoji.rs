//! Emoji picker: search, categories, recent row and a colour-emoji grid.

use fluent2_egui::controls::EmojiPicker;

use super::Page;

#[derive(Default)]
pub struct EmojiPage {
    picked: String,
}

impl Page for EmojiPage {
    fn name(&self) -> &'static str {
        "Emoji"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Emoji");
        ui.label("Search, categories and recent emoji. Missing glyphs draw tofu, never panic.");
        let response = EmojiPicker::new("gallery-emoji").show(ui);
        if let Some(emoji) = response.picked {
            self.picked = emoji;
        }
        if !self.picked.is_empty() {
            ui.add_space(8.0);
            ui.label(format!("Picked {}", self.picked));
        }
    }
}
