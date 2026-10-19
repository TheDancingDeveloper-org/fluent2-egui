//! Accessible text for DataGrid cells and rows and for List rows (WI-1848).
//!
//! AccessKit derives no name from a GridCell's, Row's or ListBoxOption's children, so the
//! focused node needs its own label for a screen reader to read anything.

use egui::accesskit::Role;
use egui::{Key, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable, by};
use fluent2_egui::data::SelectionMode;
use fluent2_egui::data::data_grid::{Column, DataGrid, DataGridState, FocusMode};
use fluent2_egui::data::list::{List, ListState};
use fluent2_egui::{Theme, install};

fn text(row: usize, column: &str) -> String {
    format!("{column} {row}")
}

fn grid_harness(mode: FocusMode) -> Harness<'static, DataGridState> {
    Harness::builder()
        .with_size(Vec2::new(600.0, 400.0))
        .build_ui_state(
            move |ui, state: &mut DataGridState| {
                install(ui.ctx(), &Theme::default());
                let columns = [Column::new("name", "Name"), Column::new("kind", "Kind")];
                DataGrid::new("grid", &columns, 5)
                    .label("Files")
                    .focus_mode(mode)
                    .cell_text(text)
                    .show(ui, state, |cell, ui| {
                        ui.label(text(cell.row, cell.column));
                    });
            },
            DataGridState::default(),
        )
}

/// Role and label of the node AccessKit reports as focused.
fn focused(harness: &Harness<'_, impl Sized>) -> Option<(Role, String)> {
    harness
        .query_all(by().predicate(|node| node.is_focused()))
        .map(|node| {
            let node = node.accesskit_node();
            (node.role(), node.label().unwrap_or_default())
        })
        .next()
}

#[test]
fn focused_grid_cell_reads_its_text() {
    let mut harness = grid_harness(FocusMode::Cell);
    harness.run_steps(2);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    let Some((role, label)) = focused(&harness) else {
        panic!("nothing is focused");
    };
    assert_eq!(role, Role::GridCell, "focus is on a cell");
    assert!(!label.is_empty(), "the focused cell has a label");
    assert!(
        label.starts_with("kind "),
        "the cell reads its own text: {label:?}"
    );
}

#[test]
fn focused_grid_row_reads_its_cells_joined() {
    let mut harness = grid_harness(FocusMode::Row);
    harness.run_steps(2);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    let Some((role, label)) = focused(&harness) else {
        panic!("nothing is focused");
    };
    assert_eq!(role, Role::Row, "focus is on a row");
    let Some(row) = label.strip_prefix("name ") else {
        panic!("the row reads its cells: {label:?}");
    };
    let index = row.split(',').next().unwrap_or_default();
    assert_eq!(label, format!("name {index}, kind {index}"));
}

#[test]
fn focused_list_option_reads_its_label() {
    let rows = ["Inbox", "Sent", "Drafts"];
    let mut harness = Harness::builder()
        .with_size(Vec2::new(400.0, 300.0))
        .build_ui_state(
            move |ui, state: &mut ListState| {
                install(ui.ctx(), &Theme::default());
                List::new("folders", rows.len())
                    .label("Folders")
                    .selection_mode(SelectionMode::Single)
                    .row_label(|index| rows.get(index).copied().unwrap_or_default().to_owned())
                    .show(ui, state, |cell, ui| {
                        ui.label(rows.get(cell.row).copied().unwrap_or_default());
                    });
            },
            ListState::default(),
        );
    harness.run_steps(2);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    let Some((role, label)) = focused(&harness) else {
        panic!("nothing is focused");
    };
    assert_eq!(role, Role::ListBoxOption, "focus is on an option");
    assert!(
        rows.contains(&label.as_str()),
        "the option reads its row: {label:?}"
    );
}
