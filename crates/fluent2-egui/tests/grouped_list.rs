//! Grouped virtual list: only visible rows are built, headers stick, groups expand and collapse.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::RowId;
use fluent2_egui::data::grouped_list::{Group, GroupedList, GroupedListState};
use fluent2_egui::{Theme, install};

struct Probe {
    state: RefCell<GroupedListState>,
    drawn: Cell<usize>,
    sticky: Cell<Option<u64>>,
    activated: Cell<Option<usize>>,
    rows_per_group: usize,
    groups: usize,
}

fn probe(groups: usize, rows: usize) -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(GroupedListState::default()),
        drawn: Cell::new(usize::MAX),
        sticky: Cell::new(None),
        activated: Cell::new(None),
        rows_per_group: rows,
        groups,
    }))
}

fn groups_of(probe: &Probe) -> Vec<Group> {
    (0..probe.groups)
        .map(|index| Group {
            id: RowId(index as u64 + 1),
            label: format!("Day {index}"),
            row_count: probe.rows_per_group,
        })
        .collect()
}

fn harness(probe: &'static Probe) -> Harness<'static> {
    harness_sized(probe, Vec2::new(320.0, 220.0))
}

fn harness_sized(probe: &'static Probe, size: Vec2) -> Harness<'static> {
    Harness::builder().with_size(size).build_ui(move |ui| {
        install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
        let groups = groups_of(probe);
        let mut state = probe.state.borrow_mut();
        let response = GroupedList::new("grouped", &groups)
            .label("Threads")
            .header_height(28.0)
            .row_height(24.0)
            .estimate(24.0)
            .show(ui, &mut state, |cell, ui| {
                ui.label(format!("Row {}", cell.row));
            });
        probe.drawn.set(probe.drawn.get().min(response.rows_drawn));
        probe.sticky.set(response.sticky.map(|id| id.0));
        if let Some(cell) = response.activated {
            probe.activated.set(Some(cell.row));
        }
    })
}

#[test]
fn only_visible_rows_are_built() {
    let probe = probe(40, 50);
    let mut harness = harness(probe);
    harness.run_steps(3);
    let drawn = probe.drawn.get();
    assert!(drawn < 30, "drew {drawn} rows of {}", 40 * 50);
    let options = harness.query_all_by_role(Role::ListBoxOption).count();
    assert!(options > 0 && options < 30, "{options} options in the tree");
    assert!(harness.query_by_role(Role::ListBox).is_some());
    assert!(harness.query_all_by_role(Role::Group).next().is_some());
}

#[test]
fn header_sticks_after_scrolling() {
    let probe = probe(6, 20);
    let mut harness = harness(probe);
    harness.run_steps(2);
    assert_eq!(probe.sticky.get(), None, "the first header is still in place");
    // The wheel only scrolls the area under the pointer.
    harness.hover_at(Pos2::new(160.0, 110.0));
    for _ in 0..4 {
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -40.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
            source: egui::MouseWheelSource::Unknown,
        });
        harness.run_steps(1);
    }
    harness.run_steps(2);
    assert_eq!(
        probe.sticky.get(),
        Some(1),
        "day 0 stays stuck while its rows scroll"
    );
}

#[test]
fn click_collapses_and_expands_a_group() {
    let probe = probe(2, 4);
    // Tall enough that both groups and all eight rows are in the tree at once.
    let mut harness = harness_sized(probe, Vec2::new(320.0, 900.0));
    harness.run_steps(2);
    let before = harness.query_all_by_role(Role::ListBoxOption).count();
    assert_eq!(before, 8);
    let header = harness.get_by_label("Day 0");
    let pos = header.rect().center();
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);
    let after = harness.query_all_by_role(Role::ListBoxOption).count();
    assert_eq!(after, 4, "the first group's rows are gone");
    let node = harness.get_by_label("Day 0");
    assert_eq!(node.accesskit_node().data().is_expanded(), Some(false));
    // Click again to expand.
    let pos = node.rect().center();
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);
    assert_eq!(harness.query_all_by_role(Role::ListBoxOption).count(), 8);
}

#[test]
fn keyboard_moves_and_collapses() {
    let probe = probe(2, 3);
    let mut harness = harness(probe);
    harness.run_steps(2);
    // Focus the list by clicking the first row, then move to the header and collapse.
    let row = harness.get_by_label("Day 0 1");
    let pos = row.rect().center();
    harness.event(Event::PointerMoved(pos));
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(1);
    harness.key_press(Key::ArrowUp);
    harness.run_steps(1);
    harness.key_press(Key::ArrowLeft);
    harness.run_steps(2);
    assert!(
        !probe.state.borrow().is_expanded(RowId(1), false),
        "left collapsed the focused group"
    );
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    assert!(probe.state.borrow().is_expanded(RowId(1), false));
    harness.key_press(Key::ArrowDown);
    harness.run_steps(1);
    harness.key_press(Key::Enter);
    harness.run_steps(1);
    assert_eq!(probe.activated.get(), Some(0));
}
