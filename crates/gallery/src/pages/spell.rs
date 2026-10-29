//! Spell check: a textarea and a rich text editor with a tiny word list the gallery owns.
//!
//! The library ships no dictionary. This page is the fake checker an app would replace with
//! its own (hunspell, a service, a user dictionary).

use fluent2_egui::controls::{RichText, RichTextEditor, Textarea, WordListChecker};

use super::Page;

pub struct SpellPage {
    notes: String,
    doc: RichText,
}

impl Default for SpellPage {
    fn default() -> Self {
        Self {
            notes: "teh quick brown fox".to_owned(),
            doc: RichText::from_plain("teh release notes are redy."),
        }
    }
}

impl Page for SpellPage {
    fn name(&self) -> &'static str {
        "Spell check"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Spell check");
        ui.label(
            "The library does not ship a dictionary. This page flags two words, \
             \"teh\" and \"redy\", and offers replacements. Right-click a squiggle, \
             or press Shift+F10 with the caret in the word.",
        );
        let mut checker = demo_checker();
        ui.add(
            Textarea::new(&mut self.notes)
                .label("Notes")
                .rows(3)
                .spell_checker(&mut checker),
        );
        ui.add_space(12.0);
        let mut checker = demo_checker();
        ui.add(
            RichTextEditor::new("spell_compose", &mut self.doc)
                .label("Message")
                .rows(4)
                .spell_checker(&mut checker),
        );
    }
}

fn demo_checker() -> WordListChecker {
    WordListChecker {
        misspelled: vec!["teh".to_owned(), "redy".to_owned()],
        suggestions: vec!["the".to_owned(), "ready".to_owned()],
    }
}
