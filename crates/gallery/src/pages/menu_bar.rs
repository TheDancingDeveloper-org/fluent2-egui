//! MenuBar: File, Edit, View, Tools and Help menus with checkable items, a submenu and
//! shortcut captions. Alt or F10 focuses the bar; Alt+letter opens a menu.

use fluent2_egui::office::menu_bar::MenuBar;
use fluent2_egui::surfaces::{MenuItem, divider, submenu};
use fluent2_icons::Icon;

use super::Page;

pub struct MenuBarPage {
    status: String,
    word_wrap: bool,
    status_bar: bool,
}

impl Default for MenuBarPage {
    fn default() -> Self {
        Self {
            status: String::new(),
            word_wrap: true,
            status_bar: true,
        }
    }
}

impl Page for MenuBarPage {
    fn name(&self) -> &'static str {
        "Menu bar"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Menu bar");
        ui.label(
            "Press Alt or F10 to focus the bar, Alt+F to open File, then the arrow keys to move. \
             With a menu open, hovering another title switches to it.",
        );
        ui.add_space(8.0);

        let mut picked: Option<&'static str> = None;
        MenuBar::new("gallery_menu_bar").show(ui, |bar| {
            bar.menu("&File", |ui| {
                if ui
                    .add(MenuItem::new("New").icon(Icon::Add).shortcut("Ctrl+N"))
                    .clicked()
                {
                    picked = Some("New");
                }
                if ui
                    .add(MenuItem::new("Open…").icon(Icon::Open).shortcut("Ctrl+O"))
                    .clicked()
                {
                    picked = Some("Open");
                }
                if ui
                    .add(MenuItem::new("Save").icon(Icon::Save).shortcut("Ctrl+S"))
                    .clicked()
                {
                    picked = Some("Save");
                }
                submenu(ui, "Recent", None, |ui| {
                    for name in ["report.txt", "notes.md"] {
                        if ui.add(MenuItem::new(name)).clicked() {
                            picked = Some("Recent file");
                        }
                    }
                });
                divider(ui);
                if ui.add(MenuItem::new("Exit")).clicked() {
                    picked = Some("Exit");
                }
            });
            bar.menu("&Edit", |ui| {
                for (label, shortcut) in [("Undo", "Ctrl+Z"), ("Cut", "Ctrl+X"), ("Copy", "Ctrl+C")] {
                    if ui.add(MenuItem::new(label).shortcut(shortcut)).clicked() {
                        picked = Some(label);
                    }
                }
                let paste = ui.add_enabled(
                    false,
                    MenuItem::new("Paste")
                        .shortcut("Ctrl+V")
                        .disabled_reason("The clipboard is empty"),
                );
                if paste.clicked() {
                    picked = Some("Paste");
                }
            });
            bar.menu("&View", |ui| {
                let wrap = MenuItem::new("Word wrap")
                    .checked(self.word_wrap)
                    .check_column(true);
                if ui.add(wrap).clicked() {
                    self.word_wrap = !self.word_wrap;
                }
                let status = MenuItem::new("Status bar")
                    .checked(self.status_bar)
                    .check_column(true);
                if ui.add(status).clicked() {
                    self.status_bar = !self.status_bar;
                }
            });
            bar.menu("&Tools", |ui| {
                if ui.add(MenuItem::new("Options…")).clicked() {
                    picked = Some("Options");
                }
            });
            bar.menu("&Help", |ui| {
                if ui.add(MenuItem::new("About")).clicked() {
                    picked = Some("About");
                }
            });
        });
        if let Some(picked) = picked {
            self.status = format!("Picked: {picked}");
        }

        ui.add_space(12.0);
        ui.label(format!(
            "Word wrap {}, status bar {}.",
            on_off(self.word_wrap),
            on_off(self.status_bar)
        ));
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
    }
}

fn on_off(value: bool) -> &'static str {
    if value { "on" } else { "off" }
}
