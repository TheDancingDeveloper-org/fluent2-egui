//! DataGrid: 100,000 synthetic rows in six columns, virtualised.

use fluent2_egui::data::data_grid::{Column, ColumnWidth, DataGrid, DataGridSize, DataGridState};

use super::Page;

const ROWS: usize = 100_000;

#[derive(Default)]
pub struct DataGridPage {
    state: DataGridState,
    size: DataGridSize,
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
        DataGrid::new("gallery_data_grid", &columns, ROWS)
            .label("Synthetic rows")
            .size(self.size)
            // Screen readers read the focused cell and row from this text (WI-1848).
            .cell_text(|row, column| {
                let index = columns.iter().position(|c| c.id() == column).unwrap_or(0);
                cell_text(row, index)
            })
            .show(ui, &mut self.state, |cell, ui| {
                ui.add(egui::Label::new(cell_text(cell.row, cell.column_index)).truncate());
            });
    }
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
