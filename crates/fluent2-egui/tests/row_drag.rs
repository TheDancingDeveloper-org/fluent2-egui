//! Row drag: DataGrid rows dragged onto external drop targets, Escape, a refusing target and
//! the keyboard menu, without the wgpu feature.

use std::cell::RefCell;

use egui::accesskit::Role;
use egui::{CursorIcon, Event, Key, Modifiers, PointerButton, Pos2, Sense, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::data_grid::{Column, DataGrid, DataGridState};
use fluent2_egui::data::row_drag::{DropChoice, DropTarget, RowDrop, drop_choices};
use fluent2_egui::data::{MoveSource, RowId, SelectionMode};
use fluent2_egui::{Theme, install};

/// What a test reads back from the harness. Each test leaks its own.
struct Probe {
    state: RefCell<DataGridState>,
    drops: RefCell<Vec<(&'static str, RowDrop)>>,
    ghost: RefCell<Option<usize>>,
}

fn probe(selected: &[u64]) -> &'static Probe {
    let mut state = DataGridState::default();
    state.selection.selected = selected.iter().copied().map(RowId).collect();
    Box::leak(Box::new(Probe {
        state: RefCell::new(state),
        drops: RefCell::new(Vec::new()),
        ghost: RefCell::new(None),
    }))
}

/// A sidebar ("Movies" accepts, "Archive" refuses) beside a ten-row grid that drags rows.
fn harness(probe: &'static Probe, theme: Theme) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(600.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(120.0);
                    for (name, accepts) in [("Movies", true), ("Archive", false)] {
                        let label = ui.add(egui::Label::new(name).sense(Sense::hover()));
                        if let Some(drop) = DropTarget::new(&label).accepts(accepts).show(ui).dropped {
                            probe.drops.borrow_mut().push((name, drop));
                        }
                    }
                });
                let columns = [Column::new("name", "Name")];
                let mut state = probe.state.borrow_mut();
                let response = DataGrid::new("torrents", &columns, 10)
                    .label("Torrents")
                    .selection_mode(SelectionMode::Multi)
                    .drag_source(true)
                    .drag_label(|n| format!("{n} torrents"))
                    .cell_text(|row, _| format!("Torrent {row}"))
                    .show(ui, &mut state, |cell, ui| {
                        ui.label(format!("Torrent {}", cell.row));
                    });
                *probe.ghost.borrow_mut() = response.dragging.map(|drag| drag.rows.len());
            });
        })
}

fn row_centre(harness: &Harness<'_>, row: usize) -> Pos2 {
    harness
        .get_by_role_and_label(Role::Row, &format!("Torrent {row}"))
        .rect()
        .center()
}

fn label_centre(harness: &Harness<'_>, label: &str) -> Pos2 {
    harness.get_by_label(label).rect().center()
}

fn button(harness: &mut Harness<'_>, pos: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(1);
}

/// Press on `from` and move in steps to `to`, without releasing.
fn drag_to(harness: &mut Harness<'_>, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.run_steps(1);
    button(harness, from, true);
    for step in 1..=6 {
        let t = step as f32 / 6.0;
        harness.event(Event::PointerMoved(from + (to - from) * t));
        harness.run_steps(1);
    }
}

fn light() -> Theme {
    Theme::light(fluent2_tokens::FLUENT)
}

#[test]
fn renders_in_every_theme() {
    for theme in [
        light(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let probe = probe(&[1, 2, 3]);
        let mut harness = harness(probe, theme);
        harness.run_steps(2);
        let from = row_centre(&harness, 2);
        let to = label_centre(&harness, "Movies");
        drag_to(&mut harness, from, to);
        assert_eq!(*probe.ghost.borrow(), Some(3));
    }
}

#[test]
fn three_selected_rows_drop_on_a_sidebar_entry() {
    let probe = probe(&[1, 2, 3]);
    let mut harness = harness(probe, light());
    harness.run_steps(2);
    let from = row_centre(&harness, 2);
    let to = label_centre(&harness, "Movies");
    drag_to(&mut harness, from, to);
    assert_eq!(*probe.ghost.borrow(), Some(3), "the ghost counts the selection");
    button(&mut harness, to, false);
    harness.run_steps(1);
    let drops = probe.drops.borrow();
    assert_eq!(drops.len(), 1);
    let (target, drop) = &drops[0];
    assert_eq!(*target, "Movies");
    assert_eq!(drop.rows, [RowId(1), RowId(2), RowId(3)]);
    assert_eq!(drop.how, MoveSource::Pointer);
    assert_eq!(*probe.ghost.borrow(), None, "the drag ends on drop");
}

#[test]
fn dragging_an_unselected_row_carries_it_alone() {
    let probe = probe(&[1, 2, 3]);
    let mut harness = harness(probe, light());
    harness.run_steps(2);
    let from = row_centre(&harness, 6);
    let to = label_centre(&harness, "Movies");
    drag_to(&mut harness, from, to);
    button(&mut harness, to, false);
    let drops = probe.drops.borrow();
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].1.rows, [RowId(6)]);
    let selected: Vec<_> = probe.state.borrow().selection.selected.iter().copied().collect();
    assert_eq!(selected, [RowId(6)], "the dragged row becomes the selection");
}

#[test]
fn escape_cancels_the_drag() {
    let probe = probe(&[1, 2]);
    let mut harness = harness(probe, light());
    harness.run_steps(2);
    let from = row_centre(&harness, 1);
    let to = label_centre(&harness, "Movies");
    drag_to(&mut harness, from, to);
    assert_eq!(*probe.ghost.borrow(), Some(2));
    harness.key_press(Key::Escape);
    harness.run_steps(1);
    assert_eq!(*probe.ghost.borrow(), None);
    button(&mut harness, to, false);
    assert!(probe.drops.borrow().is_empty(), "nothing drops after Escape");
}

#[test]
fn refusing_target_shows_not_allowed_and_takes_no_drop() {
    let probe = probe(&[1]);
    let mut harness = harness(probe, light());
    harness.run_steps(2);
    let from = row_centre(&harness, 1);
    let to = label_centre(&harness, "Archive");
    drag_to(&mut harness, from, to);
    assert_eq!(
        harness.output().platform_output.cursor_icon,
        CursorIcon::NotAllowed
    );
    button(&mut harness, to, false);
    assert!(probe.drops.borrow().is_empty());
}

#[test]
fn a_click_still_selects_without_dragging() {
    let probe = probe(&[]);
    let mut harness = harness(probe, light());
    harness.run_steps(2);
    let at = row_centre(&harness, 4);
    harness.event(Event::PointerMoved(at));
    harness.run_steps(1);
    button(&mut harness, at, true);
    button(&mut harness, at, false);
    harness.run_steps(1);
    assert!(probe.state.borrow().selection.selected.contains(&RowId(4)));
    assert_eq!(*probe.ghost.borrow(), None);
}

#[test]
fn keyboard_menu_reports_the_chosen_target_and_skips_refusing_ones() {
    let chosen: &'static RefCell<Vec<usize>> = Box::leak(Box::new(RefCell::new(Vec::new())));
    let mut harness = Harness::builder()
        .with_size(Vec2::new(300.0, 200.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &light());
            let choices = [
                DropChoice {
                    label: "Movies",
                    accepts: true,
                },
                DropChoice {
                    label: "Archive",
                    accepts: false,
                },
            ];
            if let Some(index) = drop_choices(ui, &choices) {
                chosen.borrow_mut().push(index);
            }
        });
    harness.run_steps(2);
    harness.get_by_label("Archive").click();
    harness.run_steps(2);
    assert!(chosen.borrow().is_empty(), "a refusing target is disabled");
    harness.get_by_label("Movies").click();
    harness.run_steps(2);
    assert_eq!(*chosen.borrow(), [0]);
}
