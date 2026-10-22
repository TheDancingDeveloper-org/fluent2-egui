//! Rich text editor: a formatted compose box with its HTML and plain-text output below.

use fluent2_egui::controls::{BlockKind, InlineFormat, RichText, RichTextEditor};

use super::Page;

pub struct RichTextEditorPage {
    doc: RichText,
}

impl Default for RichTextEditorPage {
    fn default() -> Self {
        let mut doc = RichText::from_plain(
            "Hi team,\nThe release notes are ready for review.\nCheck the wording\nConfirm the dates\nShip it on Friday.",
        );
        doc.toggle_inline(9..12, InlineFormat::Bold);
        doc.set_link(13..26, Some("https://example.com/notes"));
        doc.set_block(49..84, BlockKind::Bulleted);
        doc.set_block(85..90, BlockKind::Quote);
        Self { doc }
    }
}

impl Page for RichTextEditorPage {
    fn name(&self) -> &'static str {
        "Rich text editor"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Rich text editor");
        ui.add(
            RichTextEditor::new("compose", &mut self.doc)
                .label("Message")
                .placeholder("Write a message")
                .rows(7),
        );

        ui.add_space(12.0);
        ui.strong("HTML");
        ui.monospace(self.doc.to_html());
        ui.add_space(8.0);
        ui.strong("Plain text");
        ui.monospace(self.doc.to_plain());
    }
}
