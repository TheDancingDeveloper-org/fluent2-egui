//! Row drag page: drag torrents from a DataGrid onto sidebar categories, or move them with the
//! grid's context menu. The page applies every drop it is given and shows the last one.

use fluent2_egui::controls::Switch;
use fluent2_egui::data::data_grid::{Column, ColumnWidth, DataGrid, DataGridState};
use fluent2_egui::data::row_drag::{DropChoice, DropTarget, RowDrop, drop_menu};
use fluent2_egui::data::{MoveSource, RowId, SelectionMode};
use fluent2_egui::surfaces::menu_frame;

use super::Page;

const NAMES: [&str; 8] = [
    "ubuntu-24.04.iso",
    "debian-13.iso",
    "Big Buck Bunny",
    "Sintel",
    "Tears of Steel",
    "Night of the Living Dead",
    "Project Gutenberg",
    "Wikipedia dump",
];

/// Gallery page for row drag onto external targets.
pub struct RowDragPage {
    state: DataGridState,
    categories: [&'static str; 3],
    /// Category index of each torrent, `None` for uncategorised.
    category: [Option<usize>; NAMES.len()],
    archive_locked: bool,
    menu_rows: Vec<RowId>,
    last: Option<String>,
}

impl Default for RowDragPage {
    fn default() -> Self {
        Self {
            state: DataGridState::default(),
            categories: ["Linux", "Movies", "Archive"],
            category: [None; NAMES.len()],
            archive_locked: true,
            menu_rows: Vec::new(),
            last: None,
        }
    }
}

impl RowDragPage {
    fn accepts(&self, category: usize) -> bool {
        !(self.archive_locked && category == 2)
    }

    fn apply(&mut self, category: usize, drop: &RowDrop) {
        for row in &drop.rows {
            if let Some(slot) = usize::try_from(row.0).ok().and_then(|i| self.category.get_mut(i)) {
                *slot = Some(category);
            }
        }
        let how = match drop.how {
            MoveSource::Pointer => "dropped",
            MoveSource::Keyboard => "moved from the menu",
        };
        self.last = Some(format!(
            "{} torrent(s) {how} to {}",
            drop.rows.len(),
            self.categories[category]
        ));
    }
}

impl Page for RowDragPage {
    fn name(&self) -> &'static str {
        "Row drag"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Row drag");
        ui.label(
            "Select torrents and drag them onto a category; Escape cancels. A locked category \
             refuses the drop. From the keyboard, Shift+F10 on the grid opens a menu with \
             \"Move to\". The helper only reports drops; this page applies them.",
        );
        ui.add_space(8.0);
        ui.add(Switch::new(&mut self.archive_locked, "Lock Archive"));
        ui.add_space(8.0);

        let mut dropped = None;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(140.0);
                ui.strong("Categories");
                for (index, name) in self.categories.iter().enumerate() {
                    let count = self.category.iter().filter(|c| **c == Some(index)).count();
                    let label =
                        ui.add(egui::Label::new(format!("{name} ({count})")).sense(egui::Sense::hover()));
                    if let Some(drop) = DropTarget::new(&label)
                        .accepts(self.accepts(index))
                        .source("gallery_row_drag")
                        .show(ui)
                        .dropped
                    {
                        dropped = Some((index, drop));
                    }
                }
            });
            ui.vertical(|ui| {
                ui.set_height(300.0);
                let columns = [
                    Column::new("name", "Name").width(ColumnWidth::Fixed(220.0)),
                    Column::new("category", "Category"),
                ];
                let cell = |row: usize, column: &str| match column {
                    "name" => NAMES[row].to_owned(),
                    _ => self.category[row]
                        .map_or("—", |index| self.categories[index])
                        .to_owned(),
                };
                let response = DataGrid::new("gallery_row_drag", &columns, NAMES.len())
                    .label("Torrents")
                    .selection_mode(SelectionMode::Multi)
                    .drag_source(true)
                    .drag_label(|n| {
                        if n == 1 {
                            "1 torrent".to_owned()
                        } else {
                            format!("{n} torrents")
                        }
                    })
                    .cell_text(cell)
                    .show(ui, &mut self.state, |ctx, ui| {
                        ui.add(egui::Label::new(cell(ctx.row, ctx.column)).truncate());
                    });
                if response.context_menu.is_some() {
                    self.menu_rows = self.state.selection.selected.iter().copied().collect();
                }
                let choices: Vec<DropChoice<'_>> = self
                    .categories
                    .iter()
                    .enumerate()
                    .map(|(index, label)| DropChoice {
                        label,
                        accepts: self.accepts(index),
                    })
                    .collect();
                // Row right-click and Shift+F10 both arrive as `context_menu`.
                let open = response
                    .context_menu
                    .is_some()
                    .then_some(egui::SetOpenCommand::Bool(true));
                let chosen = egui::Popup::menu(&response.response)
                    .open_memory(open)
                    .frame(menu_frame(ui))
                    .show(|ui| drop_menu(ui, "Move to", &choices))
                    .and_then(|inner| inner.inner);
                if let Some(index) = chosen {
                    dropped = Some((
                        index,
                        RowDrop {
                            source: response.response.id,
                            rows: self.menu_rows.clone(),
                            how: MoveSource::Keyboard,
                        },
                    ));
                }
            });
        });
        if let Some((index, drop)) = dropped {
            self.apply(index, &drop);
        }
        if let Some(last) = &self.last {
            ui.label(last);
        }
    }
}
