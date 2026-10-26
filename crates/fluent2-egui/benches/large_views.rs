//! Frame time for large data views (issue #153).
//!
//! Not a required CI check. From the workspace root:
//!
//! ```sh
//! cargo bench -p fluent2-egui --bench large_views
//! ```
//!
//! Each case paints a 10 000-row view for 60 frames in a fixed-size harness and prints the
//! wall time per frame. The first frame is not timed: it pays for font setup and the first
//! layout. The numbers are a record, not a gate. A few percent either way between machines
//! is expected. Allocation counts are not measured here: the library forbids `unsafe`, and a
//! counting allocator needs it.

use std::cell::RefCell;
use std::time::Instant;

use egui::Vec2;
use egui_kittest::Harness;
use fluent2_egui::Combobox;
use fluent2_egui::data::data_grid::{Column, DataGrid, DataGridState};
use fluent2_egui::data::list::{List, ListState};
use fluent2_egui::{Theme, install};

const ROWS: usize = 10_000;
const FRAMES: u32 = 60;

fn theme(ui: &mut egui::Ui) {
    install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
}

fn bench(name: &str, mut step: impl FnMut()) {
    step();
    let start = Instant::now();
    for _ in 0..FRAMES {
        step();
    }
    let per_frame = start.elapsed() / FRAMES;
    println!("{name}: {per_frame:?}/frame over {FRAMES} frames ({ROWS} rows)");
}

fn main() {
    let options: Vec<String> = (0..ROWS).map(|i| format!("Option {i}")).collect();

    let list_state = RefCell::new(ListState::default());
    let mut list = Harness::builder()
        .with_size(Vec2::new(480.0, 640.0))
        .build_ui(move |ui| {
            theme(ui);
            let mut state = list_state.borrow_mut();
            List::new("bench-list", ROWS)
                .label("Bench list")
                .show(ui, &mut state, |cell, ui| {
                    ui.label(format!("Row {}", cell.row));
                });
        });

    let grid_state = RefCell::new(DataGridState::default());
    let mut grid = Harness::builder()
        .with_size(Vec2::new(640.0, 480.0))
        .build_ui(move |ui| {
            theme(ui);
            let columns = [Column::new("name", "Name"), Column::new("value", "Value")];
            let mut state = grid_state.borrow_mut();
            DataGrid::new("bench-grid", &columns, ROWS)
                .label("Bench grid")
                .show(ui, &mut state, |cell, ui| {
                    ui.label(format!("{} {}", cell.column, cell.row));
                });
        });

    let text = RefCell::new(String::new());
    let mut combo = Harness::builder()
        .with_size(Vec2::new(320.0, 480.0))
        .build_ui(move |ui| {
            theme(ui);
            let mut value = text.borrow_mut();
            ui.add(Combobox::new("bench-combo", &mut value, options.as_slice()).label("Bench"));
        });

    bench("list-10k", || list.step());
    bench("data-grid-10k", || grid.step());
    bench("combobox-10k", || combo.step());
}
