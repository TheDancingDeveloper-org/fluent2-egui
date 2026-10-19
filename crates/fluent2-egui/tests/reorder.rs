//! Reorder: pointer drag and drop within and between containers, the keyboard path, disabled
//! containers and Escape, without the wgpu feature.

use std::cell::RefCell;

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::reorder::{MoveSource, Reorder, ReorderAxis, ReorderMove, Slot};
use fluent2_egui::{Theme, install};

/// What a test reads back from the harness. Each test leaks its own.
struct Probe {
    columns: RefCell<Vec<Vec<&'static str>>>,
    moves: RefCell<Vec<ReorderMove>>,
    enabled: bool,
    theme: Theme,
}

fn probe(enabled: bool, theme: Theme) -> &'static Probe {
    Box::leak(Box::new(Probe {
        columns: RefCell::new(vec![vec!["Alpha", "Bravo", "Charlie"], vec!["Delta"]]),
        moves: RefCell::new(Vec::new()),
        enabled,
        theme,
    }))
}

/// Two side-by-side columns of labelled items. The app applies every move it is given.
fn harness(probe: &'static Probe) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(400.0, 300.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &probe.theme);
            let mut columns = probe.columns.borrow_mut();
            let mut reorder = Reorder::new("board");
            ui.add_enabled_ui(probe.enabled, |ui| {
                ui.horizontal_top(|ui| {
                    for (col, cards) in columns.iter().enumerate() {
                        ui.allocate_ui(Vec2::new(150.0, 250.0), |ui| {
                            ui.set_min_size(Vec2::new(150.0, 250.0));
                            reorder.container(ui, col, ReorderAxis::Vertical, |list, ui| {
                                ui.set_min_size(Vec2::new(150.0, 250.0));
                                for card in cards {
                                    list.item(ui, card, *card, |ui| {
                                        ui.set_min_size(Vec2::new(140.0, 30.0));
                                        ui.label(*card);
                                    });
                                }
                            });
                        });
                    }
                });
            });
            if let Some(mv) = reorder.finish(ui).moved {
                assert!(mv.apply_between(&mut columns));
                probe.moves.borrow_mut().push(mv);
            }
        })
}

fn centre(harness: &Harness<'_>, label: &str) -> Pos2 {
    harness
        .get_by_role_and_label(Role::ListItem, label)
        .rect()
        .center()
}

/// Press on `from`, move in steps to `to`, and release.
fn drag(harness: &mut Harness<'_>, from: Pos2, to: Pos2) {
    harness.event(Event::PointerMoved(from));
    harness.run_steps(1);
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(1);
    for step in 1..=6 {
        let t = step as f32 / 6.0;
        harness.event(Event::PointerMoved(from + (to - from) * t));
        harness.run_steps(1);
    }
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);
}

fn columns(probe: &Probe) -> Vec<Vec<&'static str>> {
    probe.columns.borrow().clone()
}

#[test]
fn renders_in_every_theme() {
    for theme in [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let probe = probe(true, theme);
        let mut harness = harness(probe);
        harness.run_steps(2);
        assert_eq!(harness.get_all_by_role(Role::ListItem).count(), 4);
    }
}

#[test]
fn pointer_drag_reorders_within_a_column() {
    let probe = probe(true, Theme::light(fluent2_tokens::FLUENT));
    let mut harness = harness(probe);
    harness.run_steps(2);
    let from = centre(&harness, "Alpha");
    let below_charlie = centre(&harness, "Charlie") + Vec2::new(0.0, 12.0);
    drag(&mut harness, from, below_charlie);
    assert_eq!(
        probe.moves.borrow().as_slice(),
        [ReorderMove {
            from: Slot {
                container: 0,
                index: 0
            },
            to: Slot {
                container: 0,
                index: 2
            },
            source: MoveSource::Pointer,
        }]
    );
    assert_eq!(columns(probe), [vec!["Bravo", "Charlie", "Alpha"], vec!["Delta"]]);
}

#[test]
fn pointer_drag_moves_between_columns() {
    let probe = probe(true, Theme::light(fluent2_tokens::FLUENT));
    let mut harness = harness(probe);
    harness.run_steps(2);
    let from = centre(&harness, "Bravo");
    let above_delta = centre(&harness, "Delta") - Vec2::new(0.0, 10.0);
    drag(&mut harness, from, above_delta);
    let moves = probe.moves.borrow();
    assert_eq!(moves.len(), 1);
    assert!(moves[0].between_containers());
    assert_eq!(
        moves[0].to,
        Slot {
            container: 1,
            index: 0
        }
    );
    assert_eq!(columns(probe), [vec!["Alpha", "Charlie"], vec!["Bravo", "Delta"]]);
}

#[test]
fn dropping_in_place_reports_nothing() {
    let probe = probe(true, Theme::light(fluent2_tokens::FLUENT));
    let mut harness = harness(probe);
    harness.run_steps(2);
    let from = centre(&harness, "Bravo");
    drag(&mut harness, from, from + Vec2::new(30.0, 2.0));
    assert!(probe.moves.borrow().is_empty(), "{:?}", probe.moves.borrow());
}

#[test]
fn escape_cancels_a_drag() {
    let probe = probe(true, Theme::light(fluent2_tokens::FLUENT));
    let mut harness = harness(probe);
    harness.run_steps(2);
    let from = centre(&harness, "Alpha");
    let to = centre(&harness, "Delta");
    harness.event(Event::PointerMoved(from));
    harness.run_steps(1);
    harness.event(Event::PointerButton {
        pos: from,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(1);
    for step in 1..=6 {
        harness.event(Event::PointerMoved(from + (to - from) * (step as f32 / 6.0)));
        harness.run_steps(1);
    }
    harness.key_press(Key::Escape);
    harness.run_steps(1);
    harness.event(Event::PointerButton {
        pos: to,
        button: PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run_steps(2);
    assert!(probe.moves.borrow().is_empty());
}

#[test]
fn alt_arrows_move_within_and_between_columns() {
    let probe = probe(true, Theme::light(fluent2_tokens::FLUENT));
    let mut harness = harness(probe);
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::ListItem, "Alpha").focus();
    harness.run_steps(2);

    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run_steps(2);
    assert_eq!(columns(probe)[0], ["Bravo", "Alpha", "Charlie"]);
    assert!(
        harness
            .get_by_role_and_label(Role::ListItem, "Alpha")
            .is_focused(),
        "focus follows the moved item"
    );

    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    harness.run_steps(2);
    assert_eq!(columns(probe), [vec!["Bravo", "Charlie"], vec!["Delta", "Alpha"]]);
    assert_eq!(
        probe.moves.borrow().last().copied(),
        Some(ReorderMove {
            from: Slot {
                container: 0,
                index: 1
            },
            to: Slot {
                container: 1,
                index: 1
            },
            source: MoveSource::Keyboard,
        })
    );

    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    harness.run_steps(2);
    assert_eq!(columns(probe)[1], ["Alpha", "Delta"]);
}

#[test]
fn keyboard_at_an_edge_reports_nothing() {
    let probe = probe(true, Theme::light(fluent2_tokens::FLUENT));
    let mut harness = harness(probe);
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::ListItem, "Alpha").focus();
    harness.run_steps(2);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowUp);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run_steps(2);
    assert!(probe.moves.borrow().is_empty());
}

#[test]
fn disabled_items_do_not_move() {
    let probe = probe(false, Theme::light(fluent2_tokens::FLUENT));
    let mut harness = harness(probe);
    harness.run_steps(2);
    let from = centre(&harness, "Alpha");
    let to = centre(&harness, "Delta");
    drag(&mut harness, from, to);
    harness.get_by_role_and_label(Role::ListItem, "Alpha").focus();
    harness.run_steps(1);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowDown);
    harness.run_steps(2);
    assert!(probe.moves.borrow().is_empty());
    assert_eq!(columns(probe)[0], ["Alpha", "Bravo", "Charlie"]);
}
