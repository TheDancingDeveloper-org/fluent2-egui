//! DataGrid: 100,000 synthetic rows in twenty columns, virtualised. Right-click a header for the
//! column chooser; drag a header, or press Alt+Left/Right on a focused one, to reorder. Ctrl+C
//! copies the focused cell or the selected rows as TSV, and the right-click menu offers the same
//! copies (issue #105).

use fluent2_egui::data::data_grid::{
    CellCtx, Column, ColumnWidth, DataGrid, DataGridSize, DataGridState, RowId, column_menu, rows_to_copy,
    tsv_rows,
};
use fluent2_egui::surfaces::{MenuItem, context_menu, menu_frame};
use fluent2_icons::Icon;

use super::Page;

const ROWS: usize = 100_000;

/// Column ids and headers. Name is required, so the chooser cannot hide it.
const COLUMNS: [(&str, &str); 20] = [
    ("name", "Name"),
    ("kind", "Kind"),
    ("size", "Size"),
    ("modified", "Modified"),
    ("owner", "Owner"),
    ("tags", "Tags"),
    ("created", "Created"),
    ("accessed", "Accessed"),
    ("path", "Path"),
    ("extension", "Extension"),
    ("version", "Version"),
    ("status", "Status"),
    ("priority", "Priority"),
    ("ratio", "Ratio"),
    ("seeds", "Seeds"),
    ("peers", "Peers"),
    ("down", "Down speed"),
    ("up", "Up speed"),
    ("eta", "ETA"),
    ("label", "Label"),
];

#[derive(Default)]
pub struct DataGridPage {
    state: DataGridState,
    size: DataGridSize,
    /// The column whose header menu is open, and where it opened.
    menu: Option<(&'static str, egui::Pos2)>,
    /// The cell the last right-click landed on. The menu stays open over later frames, which
    /// report no new cell, so the page keeps it here.
    menu_cell: Option<CellCtx>,
}

impl Page for DataGridPage {
    fn name(&self) -> &'static str {
        "DataGrid"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("DataGrid");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.size, DataGridSize::ExtraSmall, "Extra small");
            ui.selectable_value(&mut self.size, DataGridSize::Small, "Small");
            ui.selectable_value(&mut self.size, DataGridSize::Medium, "Medium");
        });
        ui.label("Right-click a header to choose columns. Drag a header, or press Alt+Left or Alt+Right on a focused one, to reorder. Ctrl+C copies the focused cell or the selected rows.");
        ui.add_space(8.0);
        let columns: Vec<Column> = COLUMNS
            .iter()
            .enumerate()
            .map(|(index, (id, header))| {
                let column = Column::new(id, *header);
                match index {
                    0 => column.width(ColumnWidth::Fixed(200.0)).required(true),
                    3 | 5 => column.width(ColumnWidth::Fraction(1.0)),
                    _ => column.width(ColumnWidth::Fixed(110.0)),
                }
            })
            .collect();
        let response = DataGrid::new("gallery_data_grid", &columns, ROWS)
            .label("Synthetic rows")
            .size(self.size)
            // Screen readers read the focused cell and row from this text (WI-1848). Ctrl+C
            // copies the same text (issue #105).
            .cell_text(cell_text)
            .show(ui, &mut self.state, |cell, ui| {
                ui.add(egui::Label::new(cell_text(cell.row, cell.column)).truncate());
            });

        let mut just_opened = false;
        if let Some(column) = response.header_context_menu {
            let pos = ui
                .ctx()
                .pointer_latest_pos()
                .unwrap_or(response.response.rect.left_top());
            self.menu = Some((column, pos));
            just_opened = true;
        }
        if let Some((column, pos)) = self.menu {
            let area = egui::Area::new(egui::Id::new("gallery_data_grid_menu"))
                .order(egui::Order::Foreground)
                .fixed_pos(pos)
                .show(ui.ctx(), |ui| {
                    menu_frame(ui)
                        .show(ui, |ui| {
                            ui.set_min_width(200.0);
                            column_menu(ui, &columns, &mut self.state, Some(column))
                        })
                        .inner
                });
            let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
            if area.inner.close || escape || (!just_opened && area.response.clicked_elsewhere()) {
                self.menu = None;
            }
        }

        if let Some(cell) = response.context_menu {
            self.menu_cell = Some(cell);
        }
        let rows = rows_to_copy(&self.state, ROWS, |row| RowId(row as u64));
        let menu_cell = self.menu_cell;
        context_menu(&response.response, |ui| {
            let Some(cell) = menu_cell else {
                return;
            };
            if ui
                .add(MenuItem::new("Copy cell").icon(Icon::Copy).shortcut("Ctrl+C"))
                .clicked()
            {
                ui.ctx().copy_text(cell_text(cell.row, cell.column));
                ui.close();
            }
            let label = if rows.len() > 1 { "Copy rows" } else { "Copy row" };
            if ui
                .add_enabled(
                    !rows.is_empty(),
                    MenuItem::new(label).icon(Icon::Copy).shortcut("Ctrl+C"),
                )
                .clicked()
            {
                ui.ctx().copy_text(tsv_rows(&columns, &rows, cell_text));
                ui.close();
            }
        });
    }
}

/// Synthetic text for one cell. Deterministic, so a row always reads the same.
fn cell_text(row: usize, column: &str) -> String {
    match column {
        "name" => format!("Document {row:06}"),
        "kind" => ["Folder", "Document", "Spreadsheet", "Image"][row % 4].to_owned(),
        "size" => format!("{} KB", (row * 37) % 9_999),
        "modified" | "created" | "accessed" => {
            format!("2026-{:02}-{:02}", row % 12 + 1, (row + column.len()) % 28 + 1)
        }
        "owner" => ["Ada", "Grace", "Linus", "Edsger"][row % 4].to_owned(),
        "tags" | "label" => format!("tag-{}", row % 17),
        "path" => format!("/data/{}/", row % 50),
        "extension" => ["txt", "md", "xlsx", "png"][row % 4].to_owned(),
        "version" => format!("1.{}", row % 9),
        "status" => ["Seeding", "Paused", "Downloading"][row % 3].to_owned(),
        "priority" => ["Low", "Normal", "High"][row % 3].to_owned(),
        "ratio" => format!("{:.2}", (row % 300) as f32 / 100.0),
        "seeds" | "peers" => format!("{}", (row * 7 + column.len()) % 120),
        "down" | "up" => format!("{} KiB/s", (row * 13 + column.len()) % 4_096),
        _ => format!("{}m", row % 90),
    }
}
