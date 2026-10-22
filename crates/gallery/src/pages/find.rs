//! Find: one find bar over a data grid, a list, a block of text and a log. Ctrl+F opens it
//! (the browser's own find is held off on the web), Enter and Shift+Enter step through the
//! matches, and each view scrolls to the current match and highlights it.

use fluent2_egui::controls::Button;
use fluent2_egui::data::data_grid::{Column, ColumnWidth, DataGrid, DataGridState};
use fluent2_egui::data::find::{FindBar, FindItems, FindProvider, FindState, TextLines};
use fluent2_egui::data::list::{List, ListState};

use super::Page;

const GRID_ROWS: usize = 500;
const GRID_COLUMNS: usize = 3;
const OWNERS: [&str; 4] = ["Ada", "Grace", "Linus", "Margaret"];
const TEXT: &str = "Find searches content the browser cannot see.\n\
    A canvas app paints its own text, so the browser's find has nothing to match.\n\
    Each view registers a provider: grid cells, list items, lines of text, log lines.\n\
    The bar searches every provider, counts the matches, and each view scrolls to the\n\
    current match and highlights it.";

/// Indexes of the providers in the slice passed to the bar.
const GRID: usize = 0;
const LIST: usize = 1;
const PROSE: usize = 2;
const LOG: usize = 3;

fn grid_cell(row: usize, column: usize) -> String {
    match column {
        0 => format!("report-{row:03}.xlsx"),
        1 => OWNERS.get(row % OWNERS.len()).copied().unwrap_or("").to_owned(),
        _ => format!("{} KB", (row * 37) % 900 + 12),
    }
}

pub struct FindPage {
    find: FindState,
    grid: DataGridState,
    list: ListState,
    items: Vec<String>,
    log: Vec<String>,
}

impl Default for FindPage {
    fn default() -> Self {
        Self {
            find: FindState::default(),
            grid: DataGridState::default(),
            list: ListState::default(),
            items: (0..300)
                .map(|i| format!("Task {i}: review report-{:03}", i * 3 % GRID_ROWS))
                .collect(),
            log: (0..400)
                .map(|i| {
                    let level = if i % 17 == 0 { "WARN" } else { "INFO" };
                    format!(
                        "12:{:02}:{:02} {level} worker {} finished report-{i:03}",
                        i / 60 % 60,
                        i % 60,
                        i % 4
                    )
                })
                .collect(),
        }
    }
}

impl Page for FindPage {
    fn name(&self) -> &'static str {
        "Find"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Find");
        ui.label("Press Ctrl+F (Cmd+F on macOS), or use the button. Try \"report-04\" or \"warn\".");
        if !self.find.is_open() && ui.add(Button::new("Find…")).clicked() {
            self.find.open();
        }

        let text = TextLines::new(TEXT);
        let grid_cells = FindItems::new(GRID_ROWS * GRID_COLUMNS, |item| {
            grid_cell(item / GRID_COLUMNS, item % GRID_COLUMNS)
        });
        let providers: [&dyn FindProvider; 4] = [&grid_cells, &self.items, &text, &self.log];
        ui.scope(|ui| {
            ui.set_max_width(520.0);
            FindBar::new("gallery_find")
                .match_case_toggle(true)
                .show(ui, &mut self.find, &providers);
        });
        let reveal = self.find.take_reveal();
        let find = &self.find;
        let reveals = |provider: usize, item: usize| {
            reveal
                .as_ref()
                .is_some_and(|m| m.provider == provider && m.item == item)
        };
        ui.add_space(8.0);

        ui.columns(2, |columns| {
            let [left, right] = columns else { return };

            left.strong("Data grid");
            left.allocate_ui(egui::vec2(left.available_width(), 200.0), |ui| {
                let columns = [
                    Column::new("name", "Name").width(ColumnWidth::Fixed(140.0)),
                    Column::new("owner", "Owner").width(ColumnWidth::Fixed(90.0)),
                    Column::new("size", "Size"),
                ];
                let mut grid = DataGrid::new("find_grid", &columns, GRID_ROWS).label("Reports");
                if let Some(m) = reveal.as_ref().filter(|m| m.provider == GRID) {
                    grid = grid.scroll_to_row(m.item / GRID_COLUMNS);
                }
                grid.show(ui, &mut self.grid, |cell, ui| {
                    let item = cell.row * GRID_COLUMNS + cell.column_index;
                    let text = grid_cell(cell.row, cell.column_index);
                    ui.add(egui::Label::new(find.layout_job(ui, GRID, item, &text)).truncate());
                });
            });

            left.add_space(8.0);
            left.strong("List");
            left.allocate_ui(egui::vec2(left.available_width(), 200.0), |ui| {
                let mut list = List::new("find_list", self.items.len()).label("Tasks");
                if let Some(m) = reveal.as_ref().filter(|m| m.provider == LIST) {
                    list = list.scroll_to_row(m.item);
                }
                let items = &self.items;
                list.show(ui, &mut self.list, |cell, ui| {
                    let text = items.get(cell.row).map_or("", String::as_str);
                    ui.label(find.layout_job(ui, LIST, cell.row, text));
                });
            });

            right.strong("Text");
            for (item, line) in text.lines().iter().enumerate() {
                right.label(find.layout_job(right, PROSE, item, line));
            }

            right.add_space(8.0);
            right.strong("Log");
            egui::ScrollArea::vertical()
                .id_salt("find_log")
                .max_height(240.0)
                .show(right, |ui| {
                    let mono = egui::TextFormat::simple(
                        egui::TextStyle::Monospace.resolve(ui.style()),
                        ui.visuals().text_color(),
                    );
                    for (item, line) in self.log.iter().enumerate() {
                        let job = find.layout_job_with(ui.ctx(), LOG, item, line, mono.clone());
                        let response = ui.label(job);
                        if reveals(LOG, item) {
                            response.scroll_to_me(Some(egui::Align::Center));
                        }
                    }
                });
        });
    }
}
