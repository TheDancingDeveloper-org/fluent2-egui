//! DataGrid copy (issue #105): the TSV payloads, and what Ctrl+C copies from the grid's focus.
//! The clipboard write is `Context::copy_text`; these tests read the text back from the response.

use std::cell::RefCell;

use egui::{Event, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::data_grid::{
    Column, ColumnWidth, DataGrid, DataGridState, FocusMode, RowId, SelectionMode, rows_to_copy, tsv_field,
    tsv_rows,
};
use fluent2_egui::{Theme, install};

fn columns() -> Vec<Column> {
    vec![
        Column::new("name", "Name"),
        Column::new("kind", "Kind").width(ColumnWidth::Fixed(120.0)),
        Column::new("size", "Size").width(ColumnWidth::Fixed(96.0)),
    ]
}

/// Cell text in the tests: "name 1", "kind 1", and so on, so a copy reads as its cell.
fn cell_text(row: usize, column: &'static str) -> String {
    format!("{column} {row}")
}

/// What a test reads back: the grid state, and the text the last Ctrl+C copied.
struct Probe {
    state: RefCell<DataGridState>,
    copied: RefCell<Option<String>>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(DataGridState::default()),
        copied: RefCell::new(None),
    }))
}

/// A grid of `rows` rows. `with_text` gives the grid its `cell_text`, which copy needs.
fn harness(
    probe: &'static Probe,
    mode: SelectionMode,
    focus: FocusMode,
    rows: usize,
    with_text: bool,
) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let columns = columns();
            let mut state = probe.state.borrow_mut();
            let grid = DataGrid::new("items", &columns, rows)
                .label("Items")
                .selection_mode(mode)
                .focus_mode(focus);
            let grid = if with_text {
                grid.cell_text(cell_text)
            } else {
                grid
            };
            let response = grid.show(ui, &mut state, |_, _| {});
            if response.copied.is_some() {
                *probe.copied.borrow_mut() = response.copied;
            }
        })
}

/// Click the centre of the node labelled `label`, with `modifiers` held down.
fn click(harness: &mut Harness<'static>, label: &str, modifiers: Modifiers) {
    let pos = harness.get_by_label(label).rect().center();
    click_at(harness, pos, modifiers);
}

/// Click row `row` (data index, unscrolled) at `x` points from the grid's left edge. Rows and
/// cells have no rect in the harness, so the click is placed from the grid's rect and the
/// theme's header and row heights.
fn click_grid(harness: &mut Harness<'static>, row: usize, x: f32, modifiers: Modifiers) {
    let grid = harness.get_by_label("Items").rect();
    let metrics = Theme::light(fluent2_tokens::FLUENT).data;
    let y = grid.top() + metrics.grid_header_height + (row as f32 + 0.5) * metrics.grid_row_height_medium;
    click_at(harness, Pos2::new(grid.left() + x, y), modifiers);
}

/// Click row `row` in its first column.
fn click_row(harness: &mut Harness<'static>, row: usize, modifiers: Modifiers) {
    click_grid(harness, row, 20.0, modifiers);
}

/// Click at `pos` with `modifiers` held down.
fn click_at(harness: &mut Harness<'static>, pos: Pos2, modifiers: Modifiers) {
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::ModifiersChanged(modifiers));
    harness.run_steps(1);
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers,
    });
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers,
    });
    harness.run_steps(3);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run_steps(1);
}

#[test]
fn tsv_field_quotes_only_fields_that_need_it() {
    assert_eq!(tsv_field("plain text"), "plain text");
    assert_eq!(tsv_field(""), "");
    assert_eq!(tsv_field("a\tb"), "\"a\tb\"");
    assert_eq!(tsv_field("line\nbreak"), "\"line\nbreak\"");
    assert_eq!(tsv_field("carriage\rreturn"), "\"carriage\rreturn\"");
    assert_eq!(tsv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
}

#[test]
fn tsv_rows_joins_cells_with_tabs_and_rows_with_newlines() {
    let text = tsv_rows(&columns(), &[0, 2], cell_text);
    assert_eq!(text, "name 0\tkind 0\tsize 0\nname 2\tkind 2\tsize 2");
    assert_eq!(tsv_rows(&columns(), &[], cell_text), "");
}

#[test]
fn tsv_rows_quotes_a_cell_with_a_tab_in_it() {
    let text = tsv_rows(&columns(), &[0], |_, column| {
        if column == "kind" {
            "a\tb".to_owned()
        } else {
            "x".to_owned()
        }
    });
    assert_eq!(text, "x\t\"a\tb\"\tx");
}

#[test]
fn rows_to_copy_prefers_the_selection_in_display_order() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, FocusMode::Row, 20, true);
    harness.run_steps(2);
    click_row(&mut harness, 3, Modifiers::NONE);
    click_row(&mut harness, 1, Modifiers::CTRL);
    harness.run_steps(1);
    let state = probe.state.borrow();
    assert_eq!(rows_to_copy(&state, 20, |row| RowId(row as u64)), vec![1, 3]);
}

#[test]
fn rows_to_copy_falls_back_to_the_focused_row_then_to_nothing() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::None, FocusMode::Row, 20, true);
    harness.run_steps(2);
    assert!(rows_to_copy(&probe.state.borrow(), 20, |row| RowId(row as u64)).is_empty());
    click_row(&mut harness, 4, Modifiers::NONE);
    assert_eq!(
        rows_to_copy(&probe.state.borrow(), 20, |row| RowId(row as u64)),
        vec![4]
    );
}

#[test]
fn ctrl_c_on_a_focused_row_copies_it_as_tsv() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, FocusMode::Row, 20, true);
    harness.run_steps(2);
    click_row(&mut harness, 1, Modifiers::NONE);
    harness.event(Event::Copy);
    harness.run_steps(1);
    assert_eq!(probe.copied.borrow().as_deref(), Some("name 1\tkind 1\tsize 1"));
}

#[test]
fn ctrl_c_copies_every_selected_row_in_order() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, FocusMode::Row, 20, true);
    harness.run_steps(2);
    click_row(&mut harness, 3, Modifiers::NONE);
    click_row(&mut harness, 1, Modifiers::CTRL);
    harness.event(Event::Copy);
    harness.run_steps(1);
    assert_eq!(
        probe.copied.borrow().as_deref(),
        Some("name 1\tkind 1\tsize 1\nname 3\tkind 3\tsize 3"),
    );
}

#[test]
fn ctrl_c_in_cell_focus_copies_only_the_focused_cell() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, FocusMode::Cell, 20, true);
    harness.run_steps(2);
    // Column 0 of row 2: the "name" cell.
    click_grid(&mut harness, 2, 20.0, Modifiers::NONE);
    harness.event(Event::Copy);
    harness.run_steps(1);
    assert_eq!(probe.copied.borrow().as_deref(), Some("name 2"));
}

#[test]
fn ctrl_c_copies_nothing_without_cell_text() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, FocusMode::Row, 20, false);
    harness.run_steps(2);
    // Without cell_text the rows have no labels, so focus the grid by clicking its own label.
    click(&mut harness, "Items", Modifiers::NONE);
    harness.event(Event::Copy);
    harness.run_steps(1);
    assert_eq!(*probe.copied.borrow(), None);
}

#[test]
fn ctrl_c_copies_nothing_without_focus() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, FocusMode::Row, 20, true);
    harness.run_steps(2);
    harness.event(Event::Copy);
    harness.run_steps(1);
    assert_eq!(*probe.copied.borrow(), None);
}
