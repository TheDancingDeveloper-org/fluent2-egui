//! List: selection modes, keyboard focus, virtualisation and AccessKit, without the wgpu feature
//! (`docs/specs/data-views.md` §6).

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::SelectionMode;
use fluent2_egui::data::list::{List, ListState};
use fluent2_egui::{Theme, install};

/// What a test reads back from the harness. Each test leaks its own.
struct Probe {
    state: RefCell<ListState>,
    /// Row of the last `activated` report.
    activated: Cell<Option<usize>>,
    /// Row of the last `context_menu` report.
    context: Cell<Option<usize>>,
    /// Most rows the row closure was asked to draw in one frame.
    most_rows_drawn: Cell<usize>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(ListState::default()),
        activated: Cell::new(None),
        context: Cell::new(None),
        most_rows_drawn: Cell::new(0),
    }))
}

/// A list of `rows` rows reading "Item n". Tests find rows by that label.
fn harness(probe: &'static Probe, mode: SelectionMode, rows: usize) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(400.0, 300.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let mut state = probe.state.borrow_mut();
            let mut drawn = 0;
            let response = List::new("items", rows)
                .label("Items")
                .row_label(|row| format!("Row {row}"))
                .selection_mode(mode)
                .show(ui, &mut state, |cell, ui| {
                    drawn += 1;
                    ui.label(format!("Item {}", cell.row));
                });
            probe.most_rows_drawn.set(probe.most_rows_drawn.get().max(drawn));
            if let Some(cell) = response.activated {
                probe.activated.set(Some(cell.row));
            }
            if let Some(cell) = response.context_menu {
                probe.context.set(Some(cell.row));
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

/// Click the centre of the row labelled `label`, with `modifiers` held down.
fn click(harness: &mut Harness<'static>, label: &str, modifiers: Modifiers) {
    let pos = harness.get_by_label(label).rect().center();
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
fn ctrl_click_and_shift_click_select_as_in_the_grid() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, 20);
    harness.run_steps(2);
    click(&mut harness, "Item 1", Modifiers::NONE);
    assert_eq!(selected(probe), vec![1]);
    click(&mut harness, "Item 3", Modifiers::CTRL);
    assert_eq!(selected(probe), vec![1, 3]);
    // Ctrl+click made row 3 the anchor, so Shift+click ranges from 3 to 5 and replaces.
    click(&mut harness, "Item 5", Modifiers::SHIFT);
    assert_eq!(selected(probe), vec![3, 4, 5]);
}

#[test]
fn single_mode_selection_follows_a_click() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 10);
    harness.run_steps(2);
    click(&mut harness, "Item 2", Modifiers::NONE);
    click(&mut harness, "Item 4", Modifiers::CTRL);
    assert_eq!(selected(probe), vec![4], "Single mode has no Ctrl selection");
}

#[test]
fn none_mode_selects_nothing_but_enter_activates_the_focused_row() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::None, 10);
    harness.run_steps(2);
    click(&mut harness, "Item 2", Modifiers::NONE);
    assert!(selected(probe).is_empty());
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(probe.activated.get(), Some(2));
}

#[test]
fn arrow_keys_move_focus_and_select_in_single_mode() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 50);
    harness.run_steps(2);
    click(&mut harness, "Item 0", Modifiers::NONE);
    for _ in 0..5 {
        harness.key_press(Key::ArrowDown);
        harness.run_steps(1);
    }
    assert_eq!(selected(probe), vec![5]);
    harness.key_press(Key::End);
    harness.run_steps(2);
    assert_eq!(selected(probe), vec![49]);
    assert!(
        harness.query_by_label("Item 49").is_some(),
        "the focused row was scrolled into view"
    );
}

#[test]
fn shift_arrow_extends_the_selection_in_multi_mode() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Multi, 10);
    harness.run_steps(2);
    click(&mut harness, "Item 2", Modifiers::NONE);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    harness.run_steps(1);
    harness.key_press_modifiers(Modifiers::SHIFT, Key::ArrowDown);
    harness.run_steps(2);
    assert_eq!(selected(probe), vec![2, 3, 4]);
}

#[test]
fn only_visible_rows_are_drawn_and_listed() {
    let probe = probe();
    let mut harness = harness(probe, SelectionMode::Single, 10_000);
    harness.run_steps(3);
    assert!(
        probe.most_rows_drawn.get() < 50,
        "drew {} rows",
        probe.most_rows_drawn.get()
    );
    let listed = harness.query_all_by_role(Role::ListBoxOption).count();
    assert!(listed > 0 && listed < 50, "{listed} options in the tree");
}

#[test]
fn accesskit_roles_follow_the_selection_mode() {
    let multi_probe = probe();
    let mut multi = harness(multi_probe, SelectionMode::Multi, 5);
    multi.run_steps(2);
    assert!(multi.query_by_role(Role::ListBox).is_some());
    let options: Vec<_> = multi.query_all_by_role(Role::ListBoxOption).collect();
    assert_eq!(options.len(), 5, "every row is in view");
    let mut positions: Vec<usize> = options
        .iter()
        .filter_map(|option| option.accesskit_node().data().position_in_set())
        .collect();
    positions.sort_unstable();
    assert_eq!(positions, vec![1, 2, 3, 4, 5]);
    assert!(
        options
            .iter()
            .all(|option| option.accesskit_node().data().size_of_set() == Some(5))
    );
    let none_probe = probe();
    let mut none = harness(none_probe, SelectionMode::None, 5);
    none.run_steps(2);
    assert!(none.query_by_role(Role::List).is_some());
    assert!(none.query_all_by_role(Role::ListItem).next().is_some());
}
