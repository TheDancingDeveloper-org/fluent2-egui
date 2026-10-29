//! Pane drag page: drag a message card from one pane onto a folder in another, or move it
//! with the card's menu. The page applies every drop it is given.

use fluent2_egui::controls::Switch;
use fluent2_egui::data::MoveSource;
use fluent2_egui::data::pane_drag::{
    DragSource, DropTarget, MoveChoice, PaneDrop, announce, end_frame, move_menu,
};
use fluent2_egui::surfaces::menu_frame;

use super::Page;

const MESSAGES: [&str; 3] = ["Invoice 12", "Weekly notes", "Receipt"];

/// Gallery page for the inter-pane drag primitive.
pub struct PaneDragPage {
    /// Folder index of each message, `None` while it is still in the list.
    folder: [Option<usize>; MESSAGES.len()],
    folders: [&'static str; 2],
    archive_locked: bool,
    last: Option<String>,
}

impl Default for PaneDragPage {
    fn default() -> Self {
        Self {
            folder: [None; MESSAGES.len()],
            folders: ["Inbox", "Archive"],
            archive_locked: true,
            last: None,
        }
    }
}

impl PaneDragPage {
    fn accepts(&self, folder: usize) -> bool {
        !(self.archive_locked && folder == 1)
    }

    fn apply(&mut self, index: usize, drop: &PaneDrop<usize>) {
        if let Some(slot) = self.folder.get_mut(drop.payload) {
            *slot = Some(index);
        }
        let how = match drop.how {
            MoveSource::Pointer => "dropped",
            MoveSource::Keyboard => "moved from the menu",
        };
        let name = MESSAGES.get(drop.payload).copied().unwrap_or("item");
        self.last = Some(format!("{name} {how} on {}", drop.target));
    }
}

impl Page for PaneDragPage {
    fn name(&self) -> &'static str {
        "Pane drag"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Pane drag");
        ui.label(
            "Drag a message onto a folder in the other pane. A locked folder refuses the drop \
             and shows the not-allowed cursor. Escape cancels. From the keyboard, secondary-click \
             or Shift+F10 on a message and choose \"Move to\". A polite live region announces \
             the drag.",
        );
        ui.add_space(8.0);
        ui.add(Switch::new(&mut self.archive_locked, "Lock Archive"));
        ui.add_space(8.0);
        if let Some(last) = &self.last {
            ui.label(last);
        }

        let mut dropped: Option<(usize, PaneDrop<usize>)> = None;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(220.0);
                ui.strong("Messages");
                for (index, name) in MESSAGES.iter().enumerate() {
                    let where_now = self.folder[index].map_or("list", |folder| self.folders[folder]);
                    let card = ui.add(
                        egui::Label::new(format!("{name} — {where_now}"))
                            .sense(egui::Sense::click_and_drag()),
                    );
                    DragSource::new(&card, index, *name).show(ui);
                    let choices: Vec<MoveChoice<'_>> = self
                        .folders
                        .iter()
                        .enumerate()
                        .map(|(folder, label)| MoveChoice {
                            label,
                            accepts: self.accepts(folder),
                        })
                        .collect();
                    let open = card
                        .secondary_clicked()
                        .then_some(egui::SetOpenCommand::Bool(true));
                    let chosen = egui::Popup::menu(&card)
                        .open_memory(open)
                        .frame(menu_frame(ui))
                        .show(|ui| move_menu(ui, "Move to", &choices))
                        .and_then(|inner| inner.inner);
                    if let Some(folder) = chosen {
                        dropped = Some((
                            folder,
                            PaneDrop {
                                source: card.id,
                                payload: index,
                                target: self.folders[folder].to_owned(),
                                how: MoveSource::Keyboard,
                            },
                        ));
                    }
                }
            });
            ui.vertical(|ui| {
                ui.set_width(180.0);
                ui.strong("Folders");
                for (index, name) in self.folders.iter().enumerate() {
                    let count = self.folder.iter().filter(|slot| **slot == Some(index)).count();
                    let label =
                        ui.add(egui::Label::new(format!("{name} ({count})")).sense(egui::Sense::hover()));
                    if let Some(drop) = DropTarget::<usize>::new(&label, *name)
                        .accepts(self.accepts(index))
                        .show(ui)
                        .dropped
                    {
                        dropped = Some((index, drop));
                    }
                }
            });
        });
        end_frame::<usize>(ui);
        announce(ui);
        if let Some((index, drop)) = dropped {
            self.apply(index, &drop);
        }
    }
}
