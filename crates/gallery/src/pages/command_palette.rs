//! Command palette page: a button (or Ctrl+Shift+P) opens the palette over sample commands.
//!
//! The page keeps the recents, as an app would, and shows the last chosen id. Nothing is run.

use fluent2_egui::IconName;
use fluent2_egui::controls::Button;
use fluent2_egui::surfaces::{CommandPalette, PaletteCommand};

use super::Page;

/// Most recents the page keeps.
const RECENTS_KEPT: usize = 5;

/// Gallery page for the CommandPalette.
pub struct CommandPalettePage {
    commands: Vec<PaletteCommand>,
    open: bool,
    recents: Vec<String>,
    last: Option<String>,
}

impl Default for CommandPalettePage {
    fn default() -> Self {
        let commands = vec![
            PaletteCommand::new("file.new", "New document")
                .group("File")
                .icon(IconName::Document)
                .shortcut("Ctrl+N"),
            PaletteCommand::new("file.open", "Open file")
                .group("File")
                .icon(IconName::FolderOpen)
                .shortcut("Ctrl+O")
                .keyword("load"),
            PaletteCommand::new("file.save", "Save")
                .group("File")
                .icon(IconName::Save)
                .shortcut("Ctrl+S"),
            PaletteCommand::new("file.export", "Export as PDF")
                .group("File")
                .subtitle("Not available in the gallery")
                .disabled(true),
            PaletteCommand::new("edit.find", "Find")
                .group("Edit")
                .icon(IconName::Search)
                .shortcut("Ctrl+F")
                .keyword("search"),
            PaletteCommand::new("edit.replace", "Find and replace")
                .group("Edit")
                .shortcut("Ctrl+H"),
            PaletteCommand::new("view.zoom_in", "Zoom in")
                .group("View")
                .subtitle("Make everything larger")
                .shortcut("Ctrl++"),
            PaletteCommand::new("view.zoom_out", "Zoom out")
                .group("View")
                .subtitle("Make everything smaller")
                .shortcut("Ctrl+-"),
            PaletteCommand::new("view.theme", "Toggle dark theme")
                .group("View")
                .keyword("appearance"),
        ];
        Self {
            commands,
            open: false,
            recents: Vec::new(),
            last: None,
        }
    }
}

impl Page for CommandPalettePage {
    fn name(&self) -> &'static str {
        "Command palette"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Command palette");
        ui.label(
            "Type to filter. Up and Down move, Enter chooses, Escape closes. Chosen commands \
             are listed first next time.",
        );
        ui.add_space(8.0);
        let shortcut = ui.input_mut(|input| {
            input.consume_key(egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::P)
        });
        let button = ui.add(Button::new("Open palette (Ctrl+Shift+P)").id_salt("open_palette"));
        if button.clicked() || shortcut {
            // Focus the opener first, so closing the palette returns focus to it.
            button.request_focus();
            self.open = true;
        }
        ui.add_space(8.0);
        ui.label(match &self.last {
            Some(id) => format!("Last chosen: {id}"),
            None => "Nothing chosen yet.".to_owned(),
        });

        if let Some(id) = CommandPalette::new("gallery_palette", &self.commands)
            .recents(&self.recents)
            .show(ui.ctx(), &mut self.open)
        {
            self.recents.retain(|recent| recent != &id);
            self.recents.insert(0, id.clone());
            self.recents.truncate(RECENTS_KEPT);
            self.last = Some(id);
        }
    }
}
