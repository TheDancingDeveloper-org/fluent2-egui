//! DataGrid: 100,000 synthetic rows in six columns, virtualised. Ctrl+C copies the focused cell or
//! the selected rows as TSV, and the right-click menu offers the same copies (issue #105).

use fluent2_egui::data::data_grid::{
    CellCtx, Column, ColumnWidth, DataGrid, DataGridSize, DataGridState, RowId, rows_to_copy, tsv_rows,
};
use fluent2_egui::surfaces::{MenuItem, context_menu};
use fluent2_icons::Icon;

use super::Page;

const ROWS: usize = 100_000;

#[derive(Default)]
pub struct DataGridPage {
    state: DataGridState,
    size: DataGridSize,
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
        ui.add_space(8.0);
        let columns = [
            Column::new("name", "Name").width(ColumnWidth::Fixed(200.0)),
            Column::new("kind", "Kind").width(ColumnWidth::Fixed(120.0)),
            Column::new("size", "Size").width(ColumnWidth::Fixed(96.0)),
            Column::new("modified", "Modified"),
            Column::new("owner", "Owner").width(ColumnWidth::Fixed(120.0)),
            Column::new("tags", "Tags"),
        ];
        let response = DataGrid::new("gallery_data_grid", &columns, ROWS)
            .label("Synthetic rows")
            .size(self.size)
            // Screen readers read the focused cell and row from this text (WI-1848). Ctrl+C
            // copies the same text (issue #105).
            .cell_text(|row, column| column_text(&columns, row, column))
            .show(ui, &mut self.state, |cell, ui| {
                ui.add(egui::Label::new(column_text(&columns, cell.row, cell.column)).truncate());
            });
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
                ui.ctx().copy_text(column_text(&columns, cell.row, cell.column));
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
                ui.ctx().copy_text(tsv_rows(&columns, &rows, |row, column| {
                    column_text(&columns, row, column)
                }));
                ui.close();
            }
        });
    }
}

/// The text of one cell, found by its column id. Unknown ids read as the first column.
fn column_text(columns: &[Column], row: usize, column: &str) -> String {
    let index = columns.iter().position(|c| c.id() == column).unwrap_or(0);
    cell_text(row, index)
}

/// Synthetic text for one cell. Deterministic, so a row always reads the same.
fn cell_text(row: usize, column: usize) -> String {
    match column {
        0 => format!("Document {row:06}"),
        1 => ["Folder", "Document", "Spreadsheet", "Image"][row % 4].to_owned(),
        2 => format!("{} KB", (row * 37) % 9_999),
        3 => format!("2026-{:02}-{:02}", row % 12 + 1, row % 28 + 1),
        4 => ["Ada", "Grace", "Linus", "Edsger"][row % 4].to_owned(),
        _ => format!("tag-{}", row % 17),
    }
}
