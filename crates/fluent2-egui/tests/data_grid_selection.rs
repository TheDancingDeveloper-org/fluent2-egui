//! DataGrid selection, keyboard focus and accessibility (§3 and §4), without the wgpu feature.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::data_grid::{Column, ColumnWidth, DataGrid, DataGridState, FocusMode, SelectionMode};
use fluent2_egui::{Theme, install};

/// What a test reads back from the harness. Each test leaks its own.
struct Probe {
    state: RefCell<DataGridState>,
    last_row_seen: Cell<usize>,
    /// Row of the last `activated` report.
    activated: Cell<Option<usize>>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(DataGridState::default()),
        last_row_seen: Cell::new(0),
        activated: Cell::new(None),
    }))
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("name", "Name"),
        Column::new("kind", "Kind").width(ColumnWidth::Fixed(120.0)),
        Column::new("size", "Size").width(ColumnWidth::Fixed(96.0)),
    ]
}

/// A grid of `rows` rows in `mode`. Cells read "column row", so tests find rows by label.
fn harness(probe: &'static Probe, mode: SelectionMode, rows: usize) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let columns = columns();
            let mut state = probe.state.borrow_mut();
            let response = DataGrid::new("items", &columns, rows)
                .label("Items")
                .selection_mode(mode)
                .show(ui, &mut state, |cell, ui| {
                    probe.last_row_seen.set(probe.last_row_seen.get().max(cell.row));
                    ui.label(format!("{} {}", cell.column, cell.row));
                });
            if let Some(cell) = response.activated {
                probe.activated.set(Some(cell.row));
            }
        })
}

fn selected(probe: &Probe) -> Vec<u64> {
    probe
        .state
        .borrow()
        .selection
        .selected
        .iter()
        .map(|id| id.0)
        .collect()
}

/// Click the centre of the cell labelled `label`, with `modifiers` held down.
fn click(harness: &mut Harness<'static>, label: &str, modifiers: Modifiers) {
    let pos = harness.get_by_label(label).rect().center();
    harness.event(Event::PointerMoved(pos));
    // The grid reads the modifiers from the frame's input, so hold them across the click.
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
fn click_ctrl_click_and_shift_click_select_as_specified() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, 20);
    harness.run_steps(2);
    click(&mut harness, "name 1", Modifiers::NONE);
    assert_eq!(selected(probe), vec![1]);
    click(&mut harness, "name 3", Modifiers::CTRL);
    assert_eq!(selected(probe), vec![1, 3]);
    // Ctrl+click made row 3 the anchor, so Shift+click ranges from 3 to 6 and replaces.
    click(&mut harness, "name 6", Modifiers::SHIFT);
    assert_eq!(selected(probe), vec![3, 4, 5, 6]);
}

#[test]
fn plain_click_in_none_mode_selects_nothing() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::None, 5);
    harness.run_steps(2);
    click(&mut harness, "name 2", Modifiers::NONE);
    assert!(selected(probe).is_empty());
}

#[test]
fn keyboard_down_past_the_viewport_scrolls_and_selects_the_next_row() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 100);
    harness.run_steps(2);
    click(&mut harness, "name 0", Modifiers::NONE);
    // The 600 pt viewport shows about 14 rows, so 30 presses leave it.
    for _ in 0..30 {
        harness.key_press(Key::ArrowDown);
        harness.run_steps(1);
    }
    assert_eq!(selected(probe), vec![30]);
    assert!(
        harness.query_by_label("name 30").is_some(),
        "the focused row was not scrolled into view"
    );
}

#[test]
fn home_and_end_select_the_first_and_last_row() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 50);
    harness.run_steps(2);
    click(&mut harness, "name 5", Modifiers::NONE);
    harness.key_press(Key::End);
    harness.run_steps(2);
    assert_eq!(selected(probe), vec![49]);
    harness.key_press(Key::Home);
    harness.run_steps(2);
    assert_eq!(selected(probe), vec![0]);
}

#[test]
fn enter_reports_activation_for_the_focused_row() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::None, 10);
    harness.run_steps(2);
    click(&mut harness, "name 2", Modifiers::NONE);
    assert_eq!(probe.activated.get(), None);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(probe.activated.get(), Some(2));
}

#[test]
fn header_checkbox_selects_all_then_clears() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, 5);
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::CheckBox, "Select all rows")
        .click();
    harness.run_steps(3);
    assert_eq!(selected(probe).len(), 5);
    harness
        .get_by_role_and_label(Role::CheckBox, "Select all rows")
        .click();
    harness.run_steps(3);
    assert!(selected(probe).is_empty());
}

#[test]
fn accesskit_tree_has_a_grid_with_absolute_counts_and_visible_row_indices() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 100);
    harness.run_steps(2);
    let grid = harness.get_by_role_and_label(Role::Grid, "Items");
    let node = grid.accesskit_node();
    // Header counts as a row, and the grid has the three data columns.
    assert_eq!(node.data().row_count(), Some(101));
    assert_eq!(node.data().column_count(), Some(3));

    let indices: Vec<usize> = harness
        .query_all(egui_kittest::kittest::by().role(Role::Row))
        .filter_map(|row| row.accesskit_node().data().row_index())
        .collect();
    assert!(
        indices.contains(&0),
        "the header row (index 0) is missing: {indices:?}"
    );
    assert!(indices.contains(&1), "data row 1 is missing: {indices:?}");
    assert!(
        !indices.contains(&100),
        "a row far below the viewport got a node: {indices:?}"
    );
}

#[test]
fn accesskit_rows_report_selection_in_single_mode() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 10);
    harness.run_steps(2);
    click(&mut harness, "name 4", Modifiers::NONE);
    let selected_rows: Vec<usize> = harness
        .query_all(egui_kittest::kittest::by().role(Role::Row))
        .filter(|row| row.accesskit_node().is_selected() == Some(true))
        .filter_map(|row| row.accesskit_node().data().row_index())
        .collect();
    assert_eq!(selected_rows, vec![5]);
}

#[test]
fn row_checkbox_toggles_its_row_and_click_on_the_row_still_selects() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, 10);
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::CheckBox, "Select row 4")
        .click();
    harness.run_steps(3);
    assert_eq!(selected(probe), vec![3]);
    harness
        .get_by_role_and_label(Role::CheckBox, "Select row 4")
        .click();
    harness.run_steps(3);
    assert!(selected(probe).is_empty());
    click(&mut harness, "name 7", Modifiers::NONE);
    assert_eq!(selected(probe), vec![7]);
}

#[test]
fn column_headers_are_named_by_their_text() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::None, 3);
    harness.run_steps(2);
    for name in ["Name", "Kind", "Size"] {
        let _ = harness.get_by_role_and_label(Role::ColumnHeader, name);
    }
}

#[test]
fn tab_from_the_grid_does_not_stop_on_a_row_or_header_checkbox() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, 3);
    harness.run_steps(2);
    click(&mut harness, "name 1", Modifiers::NONE);
    harness.key_press(Key::Tab);
    harness.run_steps(2);
    for label in ["Select all rows", "Select row 1", "Select row 2", "Select row 3"] {
        let checkbox = harness
            .get_by_role_and_label(Role::CheckBox, label)
            .accesskit_node();
        assert!(!checkbox.is_focused(), "Tab stopped on {label:?}");
    }
}

/// A cell-focus grid whose first column holds a button, labelled "Open N". Enter on that cell
/// moves focus into the button.
fn harness_with_buttons(probe: &'static Probe, rows: usize) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let columns = columns();
            let mut state = probe.state.borrow_mut();
            let response = DataGrid::new("items", &columns, rows)
                .label("Items")
                .focus_mode(FocusMode::Cell)
                .show(ui, &mut state, |cell, ui| {
                    if cell.column_index == 0 {
                        Some(ui.button(format!("Open {}", cell.row)))
                    } else {
                        ui.label(format!("{} {}", cell.column, cell.row));
                        None
                    }
                });
            if let Some(cell) = response.activated {
                probe.activated.set(Some(cell.row));
            }
        })
}

#[test]
fn enter_moves_focus_into_a_cell_child_and_escape_returns_it() {
    let probe = probe();
    let mut harness = harness_with_buttons(probe, 5);
    harness.run_steps(2);
    // Click the Kind cell of row 1, then move left onto the cell with the button.
    click(&mut harness, "kind 1", Modifiers::NONE);
    harness.key_press(Key::ArrowLeft);
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert!(
        harness.get_by_label("Open 1").accesskit_node().is_focused(),
        "Enter did not move focus into the button"
    );
    assert_eq!(
        probe.activated.get(),
        None,
        "Enter on a cell with a child must not activate the row"
    );

    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(
        harness
            .get_by_role_and_label(Role::Grid, "Items")
            .accesskit_node()
            .is_focused(),
        "Escape did not return focus to the grid"
    );
}

#[test]
fn enter_on_a_cell_without_a_child_still_activates() {
    let probe = probe();
    let mut harness = harness_with_buttons(probe, 5);
    harness.run_steps(2);
    click(&mut harness, "kind 2", Modifiers::NONE);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(probe.activated.get(), Some(2));
}

#[test]
fn header_keys_sort_resize_and_autofit_the_focused_column() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 10);
    harness.run_steps(2);
    click(&mut harness, "name 4", Modifiers::NONE);
    harness.key_press(Key::Home);
    harness.run_steps(2);
    harness.key_press(Key::ArrowUp);
    harness.run_steps(2);
    assert!(probe.state.borrow().focus.is_some_and(|focus| focus.header));

    harness.key_press(Key::Enter);
    harness.run_steps(2);
    let sort = probe.state.borrow().sort.clone();
    assert_eq!(sort.map(|spec| spec.column), Some("name".to_owned()));

    harness.key_press_modifiers(Modifiers::CTRL, Key::ArrowRight);
    harness.run_steps(2);
    let widened = probe.state.borrow().widths.get("name").copied();
    assert!(widened.is_some(), "Ctrl+Right did not resize the header column");

    harness.key_press_modifiers(Modifiers::CTRL, Key::ArrowLeft);
    harness.run_steps(2);
    let narrowed = probe.state.borrow().widths.get("name").copied();
    assert_eq!(narrowed.zip(widened).map(|(n, w)| (w - n).round()), Some(8.0));

    let sort_before = probe.state.borrow().sort.clone();
    harness.key_press_modifiers(Modifiers::CTRL, Key::Enter);
    harness.run_steps(2);
    assert!(
        probe.state.borrow().widths.contains_key("name"),
        "Ctrl+Enter did not autofit"
    );
    // Ctrl+Enter autofits and does not sort.
    assert_eq!(probe.state.borrow().sort, sort_before);
}
