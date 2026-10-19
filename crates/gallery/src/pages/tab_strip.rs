//! Tab strip page: document tabs that close, reorder, overflow and show a dirty marker.
//!
//! The page owns the documents, as an app would, and applies what the strip reports.

use fluent2_egui::controls::Button;
use fluent2_egui::surfaces::{StripTab, TabStrip};

use super::Page;

/// One open document.
struct Doc {
    name: String,
    dirty: bool,
}

/// Gallery page for the TabStrip.
pub struct TabStripPage {
    docs: Vec<Doc>,
    selected: usize,
    opened: usize,
    last: String,
}

impl Default for TabStripPage {
    fn default() -> Self {
        let names = ["main.rs", "lib.rs", "README.md", "Cargo.toml", "notes.txt"];
        let docs = names
            .iter()
            .enumerate()
            .map(|(index, name)| Doc {
                name: (*name).to_owned(),
                dirty: index == 1,
            })
            .collect();
        Self {
            docs,
            selected: 0,
            opened: names.len(),
            last: "Nothing yet.".to_owned(),
        }
    }
}

impl Page for TabStripPage {
    fn name(&self) -> &'static str {
        "Tab strip"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Tab strip");
        ui.label(
            "Drag a tab to move it. The close button, a middle click or Ctrl+W closes it. \
             Ctrl+Tab and Ctrl+Shift+Tab cycle; with the strip focused, arrows select and \
             Ctrl+Shift+arrows move. Open more documents than fit to see the overflow menu.",
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.add(Button::new("New document").id_salt("new_doc")).clicked() {
                self.opened += 1;
                self.docs.push(Doc {
                    name: format!("untitled-{}.txt", self.opened),
                    dirty: false,
                });
                self.selected = self.docs.len() - 1;
            }
            if ui
                .add(Button::new("Toggle unsaved").id_salt("toggle_dirty"))
                .clicked()
                && let Some(doc) = self.docs.get_mut(self.selected)
            {
                doc.dirty = !doc.dirty;
            }
        });
        ui.add_space(8.0);

        let tabs = self
            .docs
            .iter()
            .map(|doc| StripTab::new(doc.name.clone()).dirty(doc.dirty));
        let response = TabStrip::new("gallery_tab_strip", &mut self.selected)
            .tabs(tabs)
            .shortcuts(true)
            .show(ui);
        if let Some(index) = response.closed
            && let Some(doc) = self.docs.get(index)
        {
            self.last = format!("Closed {}", doc.name);
        }
        if let Some(moved) = response.moved {
            self.last = format!("Moved tab {} to {}", moved.from + 1, moved.to + 1);
        }
        response.apply(&mut self.docs, &mut self.selected);

        ui.add_space(12.0);
        match self.docs.get(self.selected) {
            Some(doc) => ui.label(format!(
                "Showing {}{}",
                doc.name,
                if doc.dirty { " (unsaved)" } else { "" }
            )),
            None => ui.label("No documents open."),
        };
        ui.label(format!("Last action: {}", self.last));
    }
}
