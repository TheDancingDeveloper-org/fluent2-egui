//! DataGrid core behaviour (§1), without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::{Event, MouseWheelUnit, Pos2, TouchPhase, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::data_grid::{Column, ColumnWidth, DataGrid, DataGridSize, DataGridState};
use fluent2_egui::{Theme, install};

/// What a test reads back from the harness. Each test leaks its own.
struct Probe {
    state: RefCell<DataGridState>,
    calls: Cell<usize>,
    last_row_seen: Cell<usize>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(DataGridState::default()),
        calls: Cell::new(0),
        last_row_seen: Cell::new(0),
    }))
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("name", "Name"),
        Column::new("kind", "Kind").width(ColumnWidth::Fixed(120.0)),
        Column::new("size", "Size").width(ColumnWidth::Fixed(96.0)),
        Column::new("modified", "Modified"),
        Column::new("owner", "Owner").width(ColumnWidth::Fixed(120.0)),
        Column::new("tags", "Tags"),
    ]
}

/// A grid with `rows` synthetic rows, counting every cell call in `probe`.
fn grid_harness(probe: &'static Probe, rows: usize, theme: Theme) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            let columns = columns();
            let mut state = probe.state.borrow_mut();
            DataGrid::new("items", &columns, rows)
                .label("Items")
                .size(DataGridSize::Medium)
                .show(ui, &mut state, |cell, ui| {
                    probe.calls.set(probe.calls.get() + 1);
                    probe.last_row_seen.set(probe.last_row_seen.get().max(cell.row));
                    ui.label(format!("{} {}", cell.column, cell.row));
                });
        })
}

#[test]
fn renders_without_panic_in_every_theme() {
    let themes = [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ];
    for theme in themes {
        let kind = theme.kind;
        let probe = probe();
        let mut harness = grid_harness(probe, 20, theme);
        harness.run_steps(2);
        assert!(probe.calls.get() > 0, "{kind:?} drew no cells");
    }
}

#[test]
fn hundred_thousand_rows_call_the_cell_closure_only_for_visible_rows() {
    let probe = probe();
    let mut harness = grid_harness(probe, 100_000, Theme::light(fluent2_tokens::FLUENT));
    probe.calls.set(0);
    harness.run_steps(1);
    let calls = probe.calls.get();
    assert!(calls > 0, "no cells drawn");
    assert!(calls < 2_000, "one frame called the cell closure {calls} times");
}

#[test]
fn scrolling_to_the_end_shows_the_last_row() {
    let probe = probe();
    let mut harness = grid_harness(probe, 100_000, Theme::light(fluent2_tokens::FLUENT));
    harness.run_steps(2);
    harness.hover_at(Pos2::new(300.0, 300.0));
    harness.event(Event::MouseWheel {
        unit: MouseWheelUnit::Point,
        delta: Vec2::new(0.0, -1.0e9),
        modifiers: Default::default(),
        phase: TouchPhase::Move,
    });
    harness.run_steps(120);
    assert_eq!(probe.last_row_seen.get(), 99_999);
}

#[test]
fn zero_rows_shows_the_header_and_no_cells() {
    let probe = probe();
    let mut harness = grid_harness(probe, 0, Theme::light(fluent2_tokens::FLUENT));
    harness.run_steps(2);
    assert_eq!(probe.calls.get(), 0);
    let _ = harness.get_by_label("Name");
}
