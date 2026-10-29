//! List with variable-height rows: measurement, the height cache, scroll anchoring and
//! virtualisation over 10,000 mixed rows (`docs/specs/data-views.md` §6.1).

use std::cell::{Cell, RefCell};
use std::collections::HashMap;

use egui::{Event, Key, Modifiers, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::RowId;
use fluent2_egui::data::list::{List, ListState};
use fluent2_egui::{Theme, install};

/// Row ids start here; inserting at the top takes ids below it.
const BASE: u64 = 1_000_000;

struct Probe {
    state: RefCell<ListState>,
    /// Rows inserted above the original first row.
    inserted: Cell<usize>,
    /// Screen top of each row id drawn in the last frame.
    tops: RefCell<HashMap<u64, f32>>,
    /// Rows drawn in the last frame.
    drawn: Cell<usize>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(ListState::default()),
        inserted: Cell::new(0),
        tops: RefCell::new(HashMap::new()),
        drawn: Cell::new(0),
    }))
}

/// Lines in row `id`: 1, 2 or 3, so heights are mixed.
fn lines(id: u64) -> u64 {
    id % 3 + 1
}

fn harness(probe: &'static Probe, rows: usize) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(400.0, 300.0))
        // Scroll animations measure new rows on each frame, so a run takes more than 4 steps.
        .with_max_steps(200)
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let inserted = probe.inserted.get();
            let count = rows + inserted;
            let id_of = move |index: usize| RowId(BASE + index as u64 - inserted as u64);
            let mut state = probe.state.borrow_mut();
            let mut tops = HashMap::new();
            List::new("variable", count)
                .label("Mixed")
                .row_label(|index| format!("Row {}", id_of(index).0))
                .row_id(id_of)
                .variable_height(40.0)
                .show(ui, &mut state, |cell, ui| {
                    tops.insert(cell.row_id.0, ui.max_rect().top());
                    for line in 0..lines(cell.row_id.0) {
                        ui.label(format!("Row {} line {line}", cell.row_id.0));
                    }
                });
            probe.drawn.set(tops.len());
            *probe.tops.borrow_mut() = tops;
        })
}

fn top(probe: &Probe, id: u64) -> Option<f32> {
    probe.tops.borrow().get(&id).copied()
}

fn press(harness: &mut Harness<'static>, key: Key) {
    harness.event(Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
}

#[test]
fn rows_take_their_measured_heights() {
    let probe = probe();
    let mut harness = harness(probe, 20);
    harness.run();
    // Row BASE has 2 lines, BASE+1 has 3, BASE+2 has 1.
    let (a, b, c, d) = (
        top(probe, BASE).unwrap_or_default(),
        top(probe, BASE + 1).unwrap_or_default(),
        top(probe, BASE + 2).unwrap_or_default(),
        top(probe, BASE + 3).unwrap_or_default(),
    );
    let (h0, h1, h2) = (b - a, c - b, d - c);
    assert!(h1 > h0 && h0 > h2, "heights {h0} {h1} {h2}");
    // One line is the 32 pt minimum row height at most; three lines are well over it.
    assert!(h2 >= 32.0 && h1 > 50.0, "heights {h0} {h1} {h2}");
}

#[test]
fn ten_thousand_mixed_rows_draw_only_the_visible_ones() {
    let probe = probe();
    let mut harness = harness(probe, 10_000);
    harness.run();
    assert!(probe.drawn.get() < 20, "drew {}", probe.drawn.get());
    // Focus the list, then End scrolls to the last row.
    harness.get_by_label("Row 1000000 line 0").click();
    harness.run();
    press(&mut harness, Key::End);
    harness.run();
    let mut seen: Vec<u64> = probe.tops.borrow().keys().copied().collect();
    seen.sort_unstable();
    assert!(
        top(probe, BASE + 9_999).is_some(),
        "last row not drawn: {seen:?} focus {:?}",
        probe.state.borrow().focus
    );
    assert!(probe.drawn.get() < 20, "drew {}", probe.drawn.get());
    assert_eq!(probe.state.borrow().focus, Some(RowId(BASE + 9_999)));
    press(&mut harness, Key::Home);
    assert!(top(probe, BASE).is_some(), "first row not drawn after Home");
}

#[test]
fn inserting_rows_above_keeps_the_view_still() {
    let probe = probe();
    let mut harness = harness(probe, 500);
    harness.run();
    harness.get_by_label("Row 1000000 line 0").click();
    harness.run();
    // Move focus down so the viewport scrolls away from the top.
    for _ in 0..40 {
        press(&mut harness, Key::ArrowDown);
    }
    harness.run();
    let watched = *probe
        .tops
        .borrow()
        .keys()
        .max()
        .unwrap_or_else(|| unreachable!("rows were drawn"));
    let before = top(probe, watched).unwrap_or_default();

    // Five rows arrive at the top, above the viewport.
    probe.inserted.set(5);
    harness.run();
    let after = top(probe, watched).unwrap_or_else(|| unreachable!("row left the view"));
    assert!((after - before).abs() < 1.0, "row moved from {before} to {after}");
}

#[test]
fn forgetting_heights_remeasures_without_panicking() {
    let probe = probe();
    let mut harness = harness(probe, 50);
    harness.run();
    let ctx = harness.ctx.clone();
    List::forget_heights(&ctx, "variable");
    harness.run();
    assert!(top(probe, BASE).is_some());
}
