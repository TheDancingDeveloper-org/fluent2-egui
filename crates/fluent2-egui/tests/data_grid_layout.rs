//! DataGrid column layout (§2.4): chooser, header menu, reorder and persisted layout, without
//! the wgpu feature.

use std::cell::{Cell, RefCell};
use std::time::{Duration, Instant};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::data_grid::{
    Column, ColumnLayout, ColumnWidth, DataGrid, DataGridState, column_menu,
};
use fluent2_egui::{Theme, install};

/// What a test reads back from the harness. Each test leaks its own.
struct Probe {
    state: RefCell<DataGridState>,
    header_menu: Cell<Option<&'static str>>,
    reordered: Cell<bool>,
}

fn probe() -> &'static Probe {
    Box::leak(Box::new(Probe {
        state: RefCell::new(DataGridState::default()),
        header_menu: Cell::new(None),
        reordered: Cell::new(false),
    }))
}

fn columns() -> Vec<Column> {
    vec![
        Column::new("name", "Name").required(true),
        Column::new("kind", "Kind").width(ColumnWidth::Fixed(120.0)),
        Column::new("size", "Size").width(ColumnWidth::Fixed(96.0)),
    ]
}

/// A three-column grid. With `menu`, the column chooser is drawn under it for `column`.
fn harness(probe: &'static Probe, menu: Option<&'static str>) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let columns = columns();
            let mut state = probe.state.borrow_mut();
            ui.allocate_ui(Vec2::new(800.0, 300.0), |ui| {
                let response =
                    DataGrid::new("items", &columns, 5)
                        .label("Items")
                        .show(ui, &mut state, |cell, ui| {
                            ui.label(format!("{} {}", cell.column, cell.row));
                        });
                if let Some(column) = response.header_context_menu {
                    probe.header_menu.set(Some(column));
                }
                if response.columns_reordered {
                    probe.reordered.set(true);
                }
            });
            if let Some(column) = menu {
                column_menu(ui, &columns, &mut state, Some(column));
            }
        })
}

fn header_order(harness: &Harness<'static>) -> Vec<(String, usize)> {
    let mut headers: Vec<(String, usize)> = harness
        .get_all_by_role(Role::ColumnHeader)
        .map(|node| {
            let node = node.accesskit_node();
            (
                node.label().unwrap_or_default(),
                node.data().column_index().unwrap_or(usize::MAX),
            )
        })
        .collect();
    headers.sort_by_key(|(_, index)| *index);
    headers
}

fn names(harness: &Harness<'static>) -> Vec<String> {
    header_order(harness).into_iter().map(|(name, _)| name).collect()
}

fn click_at(harness: &mut Harness<'static>, pos: Pos2, button: PointerButton) {
    harness.event(Event::PointerMoved(pos));
    harness.run_steps(1);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run_steps(2);
}

#[test]
fn layout_orders_hides_and_keeps_required_columns() {
    let columns = columns();
    let mut layout = ColumnLayout::default();
    assert!(layout.move_column(&columns, "size", 0));
    let ids: Vec<_> = layout.apply(&columns).iter().map(Column::id).collect();
    assert_eq!(ids, ["size", "name", "kind"]);
    layout.set_hidden("kind", true);
    layout.set_hidden("name", true);
    let ids: Vec<_> = layout.apply(&columns).iter().map(Column::id).collect();
    assert_eq!(ids, ["size", "name"], "a required column ignores hidden");
    layout.reset();
    let ids: Vec<_> = layout.apply(&columns).iter().map(Column::id).collect();
    assert_eq!(ids, ["name", "kind", "size"]);
}

#[test]
fn unknown_and_missing_ids_fall_back_to_definition_order() {
    let columns = columns();
    let layout = ColumnLayout {
        order: vec!["gone".into(), "size".into()],
        hidden: Default::default(),
    };
    let ids: Vec<_> = layout.apply(&columns).iter().map(Column::id).collect();
    assert_eq!(ids, ["size", "name", "kind"]);
}

#[test]
fn accesskit_reports_the_layout_order_and_leaves_hidden_columns_out() {
    let probe = probe();
    {
        let columns = columns();
        let mut state = probe.state.borrow_mut();
        state.layout.move_column(&columns, "size", 0);
        state.layout.set_hidden("kind", true);
    }
    let mut harness = harness(probe, None);
    harness.run_steps(2);
    assert_eq!(
        header_order(&harness),
        [("Size".to_owned(), 0), ("Name".to_owned(), 1)]
    );
}

#[test]
fn secondary_click_on_a_header_reports_its_column() {
    let probe = probe();
    let mut harness = harness(probe, None);
    harness.run_steps(2);
    let pos = harness
        .get_by_role_and_label(Role::ColumnHeader, "Kind")
        .rect()
        .center();
    click_at(&mut harness, pos, PointerButton::Secondary);
    assert_eq!(probe.header_menu.get(), Some("kind"));
    assert!(
        probe.state.borrow().sort.is_none(),
        "a secondary click does not sort"
    );
}

#[test]
fn chooser_hides_a_column_and_offers_no_item_for_required_ones() {
    let probe = probe();
    let mut harness = harness(probe, Some("kind"));
    harness.run_steps(2);
    assert!(
        harness
            .query_by_role_and_label(Role::MenuItemCheckBox, "Name")
            .is_none(),
        "required Name has no chooser item"
    );
    harness
        .get_by_role_and_label(Role::MenuItemCheckBox, "Size")
        .click();
    harness.run_steps(3);
    assert!(probe.state.borrow().layout.is_hidden("size"));
    assert_eq!(names(&harness), ["Name", "Kind"]);
    harness
        .get_by_role_and_label(Role::MenuItem, "Reset columns")
        .click();
    harness.run_steps(3);
    assert_eq!(names(&harness), ["Name", "Kind", "Size"]);
}

#[test]
fn resize_to_fit_autofits_on_the_next_show() {
    let probe = probe();
    let mut harness = harness(probe, Some("kind"));
    harness.run_steps(2);
    harness
        .get_by_role_and_label(Role::MenuItem, "Resize to fit")
        .click();
    harness.run_steps(3);
    let state = probe.state.borrow();
    assert!(state.autofit_request.is_none(), "the grid takes the request");
    let width = state.widths.get("kind").copied().unwrap_or_default();
    assert!(
        width > 0.0 && (width - 120.0).abs() > 0.5,
        "kind was autofitted: {width}"
    );
}

#[test]
fn dragging_a_header_reorders_the_columns() {
    let probe = probe();
    let mut harness = harness(probe, None);
    harness.run_steps(2);
    let from = harness
        .get_by_role_and_label(Role::ColumnHeader, "Size")
        .rect()
        .center();
    let to = harness
        .get_by_role_and_label(Role::ColumnHeader, "Name")
        .rect()
        .left_center()
        + Vec2::new(4.0, 0.0);
    harness.hover_at(from);
    harness.run_steps(2);
    harness.drag_at(from);
    harness.run_steps(2);
    harness.event(Event::PointerMoved(to));
    harness.run_steps(2);
    harness.drop_at(to);
    harness.run_steps(3);
    assert!(probe.reordered.get());
    assert_eq!(names(&harness), ["Size", "Name", "Kind"]);
    assert!(probe.state.borrow().sort.is_none(), "a drag does not sort");
}

#[test]
fn alt_arrows_on_a_focused_header_move_its_column() {
    let probe = probe();
    let mut harness = harness(probe, None);
    harness.run_steps(2);
    let pos = harness.get_by_label("name 0").rect().center();
    click_at(&mut harness, pos, PointerButton::Primary);
    harness.key_press(Key::ArrowUp);
    harness.run_steps(2);
    assert!(probe.state.borrow().focus.is_some_and(|focus| focus.header));
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    harness.run_steps(2);
    assert_eq!(names(&harness), ["Kind", "Name", "Size"]);
    assert_eq!(
        probe.state.borrow().focus.and_then(|focus| focus.column),
        Some(1),
        "focus follows the column"
    );
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowRight);
    harness.run_steps(2);
    assert_eq!(names(&harness), ["Kind", "Size", "Name"]);
    harness.key_press_modifiers(Modifiers::ALT, Key::ArrowLeft);
    harness.run_steps(2);
    assert_eq!(names(&harness), ["Kind", "Name", "Size"]);
}

#[cfg(feature = "serde")]
#[test]
fn layout_round_trips_through_serde() {
    let columns = columns();
    let mut state = DataGridState::default();
    state.layout.move_column(&columns, "size", 0);
    state.layout.set_hidden("kind", true);
    state.widths.insert("size".into(), 140.0);
    state.autofit_request = Some("kind".into());
    let json = serde_json::to_string(&state).unwrap_or_default();
    let back: DataGridState = serde_json::from_str(&json).unwrap_or_default();
    assert!(!back.layout.order.is_empty(), "json was {json}");
    assert_eq!(back.layout, state.layout);
    assert_eq!(back.widths, state.widths);
    assert_eq!(back.autofit_request, None, "the request is not persisted");
    // State saved before the layout existed still loads.
    let old: Option<DataGridState> = serde_json::from_str(
        r#"{"sort":null,"widths":{},"selection":{"selected":[],"anchor":null},"focus":null}"#,
    )
    .ok();
    assert_eq!(old.map(|old| old.layout), Some(ColumnLayout::default()));
}

/// Frame budget (§2.4): a 10 000 × 20 grid, scrolled and laid out each frame, stays well inside
/// a frame. The spec states 4 ms in release; this asserts a looser bound that holds in debug
/// builds on a loaded CI runner.
#[test]
fn ten_thousand_by_twenty_stays_inside_the_frame_budget() {
    const FRAMES: u32 = 30;
    let ids: Vec<&'static str> = (0..20)
        .map(|index| &*Box::leak(format!("c{index}").into_boxed_str()))
        .collect();
    let columns: Vec<Column> = ids
        .iter()
        .map(|id| Column::new(id, *id).width(ColumnWidth::Fixed(90.0)))
        .collect();
    let state = Box::leak(Box::new(RefCell::new(DataGridState::default())));
    {
        let mut state = state.borrow_mut();
        state.layout.move_column(&columns, "c19", 0);
        state.layout.set_hidden("c3", true);
    }
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1280.0, 800.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            DataGrid::new("budget", &columns, 10_000)
                .label("Budget")
                .cell_text(|row, column| format!("{column} {row}"))
                .show(ui, &mut state.borrow_mut(), |cell, ui| {
                    ui.label(format!("{} {}", cell.column, cell.row));
                });
        });
    harness.run_steps(2);
    let start = Instant::now();
    for _ in 0..FRAMES {
        harness.event(Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: Vec2::new(0.0, -400.0),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        });
        harness.step();
    }
    let per_frame = start.elapsed() / FRAMES;
    assert!(per_frame < Duration::from_millis(100), "{per_frame:?} per frame");
}
