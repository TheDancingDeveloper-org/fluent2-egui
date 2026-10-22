//! Shortcuts page: one registry drives the key handling, menu captions, tooltips and the `?`
//! help overlay.

use egui::{Key, KeyboardShortcut, Modifiers};
use fluent2_egui::controls::Button;
use fluent2_egui::foundation::shortcuts::{self, Shortcut, ShortcutScope, Shortcuts, ShortcutsExt};
use fluent2_egui::surfaces::{MenuItem, ShortcutOverlay, menu_frame};

use super::Page;

/// Gallery page for the shortcut registry.
pub struct ShortcutsPage {
    registry: Shortcuts,
    help_open: bool,
    last: String,
    note: String,
}

impl Default for ShortcutsPage {
    fn default() -> Self {
        let mut registry = Shortcuts::new();
        let bindings = [
            Shortcut::new(
                "file.new",
                KeyboardShortcut::new(Modifiers::COMMAND, Key::N),
                "New",
            )
            .group("File"),
            Shortcut::new(
                "file.save",
                KeyboardShortcut::new(Modifiers::COMMAND, Key::S),
                "Save",
            )
            .group("File"),
            Shortcut::new(
                "file.save_as",
                KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::S),
                "Save as",
            )
            .group("File"),
            Shortcut::new(
                "nav.next",
                KeyboardShortcut::new(Modifiers::NONE, Key::J),
                "Next item",
            )
            .group("Navigation"),
            Shortcut::new(
                "nav.previous",
                KeyboardShortcut::new(Modifiers::NONE, Key::K),
                "Previous item",
            )
            .group("Navigation"),
            Shortcut::new(
                "help.shortcuts",
                KeyboardShortcut::new(Modifiers::NONE, Key::Questionmark),
                "Show keyboard shortcuts",
            )
            .group("Help")
            .scope(ShortcutScope::NotTyping),
        ];
        for binding in bindings {
            // The ids and keys above are distinct; a conflict would only drop that row.
            let _ = registry.register(binding);
        }
        Self {
            registry,
            help_open: false,
            last: "Nothing yet".to_owned(),
            note: String::new(),
        }
    }
}

impl Page for ShortcutsPage {
    fn name(&self) -> &'static str {
        "Shortcuts"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        shortcuts::install(&ctx, self.registry.clone());

        for id in [
            "file.new",
            "file.save",
            "file.save_as",
            "nav.next",
            "nav.previous",
        ] {
            if ctx.shortcut_pressed(id) {
                self.last = self.registry.get(id).map(|s| s.label.clone()).unwrap_or_default();
            }
        }
        if ctx.shortcut_pressed("help.shortcuts") {
            self.help_open = !self.help_open;
        }

        ui.label("Press Ctrl+S, J or K, or ? for the overlay. J and K do nothing while the field has focus.");
        ui.label(format!("Last shortcut: {}", self.last));
        let _ =
            ui.add(egui::TextEdit::singleline(&mut self.note).hint_text("Type here: J and K are letters"));
        ui.add_space(8.0);

        ui.horizontal(|ui| {
            let tip = ctx.shortcut_tooltip("file.save").unwrap_or_default();
            if ui.add(Button::new("Save")).on_hover_text(tip).clicked() {
                self.last = "Save (button)".to_owned();
            }
            if ui.add(Button::new("Keyboard shortcuts")).clicked() {
                self.help_open = true;
            }
        });
        ui.add_space(8.0);

        menu_frame(ui).show(ui, |ui| {
            ui.set_width(260.0);
            for (id, text) in [
                ("file.new", "New"),
                ("file.save", "Save"),
                ("file.save_as", "Save as"),
            ] {
                if ui.add(MenuItem::new(text).bound(id)).clicked() {
                    self.last = format!("{text} (menu)");
                }
            }
        });

        let _ = ShortcutOverlay::new("gallery-shortcut-help").show(&ctx, &mut self.help_open);
    }
}
