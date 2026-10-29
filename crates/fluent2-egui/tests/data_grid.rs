//! DataGrid core behaviour (§1), without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::{Event, MouseWheelUnit, Pos2, Rect, Sense, TouchPhase, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::data_grid::{
    Column, ColumnWidth, DataGrid, DataGridSize, DataGridState, SortDirection, SortSpec,
};
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
        source: egui::MouseWheelSource::Unknown,
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
    // The header label and its sort button both carry the name, so query for all of them.
    assert!(harness.get_all_by_label("Name").next().is_some());
}

/// What a §2 test reads back: the state, the grid's rect, and what the grid reported.
struct SortProbe {
    state: RefCell<DataGridState>,
    rect: Cell<Rect>,
    sort_changed: Cell<bool>,
    resized: Cell<Option<&'static str>>,
}

fn sort_probe() -> &'static SortProbe {
    Box::leak(Box::new(SortProbe {
        state: RefCell::new(DataGridState::default()),
        rect: Cell::new(Rect::NOTHING),
        sort_changed: Cell::new(false),
        resized: Cell::new(None),
    }))
}

/// Horizontal padding a cell gets (`spacingHorizontalS`), as autofit adds it.
fn pad() -> f32 {
    fluent2_egui::tokens::generated::global::spacing_horizontal::S
}

/// A grid whose every cell is `cell_width` wide, with the columns from `columns`.
fn sort_harness(
    probe: &'static SortProbe,
    columns: fn() -> Vec<Column>,
    rows: usize,
    cell_width: f32,
) -> Harness<'static> {
    Harness::builder()
        .with_size(Vec2::new(800.0, 600.0))
        // egui's double-click window is 0.3 s; the default step is 0.25 s, so use 60 fps.
        .with_step_dt(1.0 / 60.0)
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::light(fluent2_tokens::FLUENT));
            let columns = columns();
            let mut state = probe.state.borrow_mut();
            let response = DataGrid::new("sorted", &columns, rows).show(ui, &mut state, |_, ui| {
                ui.allocate_exact_size(Vec2::new(cell_width, 10.0), Sense::hover());
            });
            probe.rect.set(response.response.rect);
            probe
                .sort_changed
                .set(probe.sort_changed.get() || response.sort_changed);
            if response.column_resized.is_some() {
                probe.resized.set(response.column_resized);
            }
        })
}

fn sortable_columns() -> Vec<Column> {
    vec![
        Column::new("name", "Name").width(ColumnWidth::Fixed(120.0)),
        Column::new("kind", "Kind")
            .width(ColumnWidth::Fixed(120.0))
            .sortable(false),
    ]
}

fn one_fixed_column() -> Vec<Column> {
    vec![
        Column::new("a", "A")
            .width(ColumnWidth::Fixed(60.0))
            .max_width(200.0),
    ]
}

fn one_auto_column() -> Vec<Column> {
    vec![Column::new("a", "A").width(ColumnWidth::Auto)]
}

/// A point in the header, `x` from the grid's left edge.
fn header_point(probe: &SortProbe, x: f32) -> Pos2 {
    let rect = probe.rect.get();
    Pos2::new(rect.min.x + x, rect.min.y + 16.0)
}

/// A press and release at `pos` with no cursor reset after it, so two in a row are a
/// double-click.
fn raw_click(harness: &mut Harness<'static>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    harness.run_steps(1);
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: Default::default(),
        });
        harness.run_steps(1);
    }
    harness.run_steps(2);
}

/// A press and release at `pos`, with frames between, so egui sees one click.
fn click(harness: &mut Harness<'static>, pos: Pos2) {
    harness.hover_at(pos);
    harness.run_steps(2);
    harness.drag_at(pos);
    harness.run_steps(2);
    harness.drop_at(pos);
    harness.run_steps(2);
}

#[test]
fn clicking_a_header_cycles_none_asc_desc_none() {
    let probe = sort_probe();
    let mut harness = sort_harness(probe, sortable_columns, 20, 100.0);
    harness.run_steps(2);
    let name = header_point(probe, 60.0);

    click(&mut harness, name);
    assert_eq!(
        probe.state.borrow().sort,
        Some(SortSpec {
            column: "name".into(),
            direction: SortDirection::Ascending
        })
    );
    assert!(probe.sort_changed.get());

    click(&mut harness, name);
    assert_eq!(
        probe.state.borrow().sort.as_ref().map(|s| s.direction),
        Some(SortDirection::Descending)
    );

    click(&mut harness, name);
    assert_eq!(probe.state.borrow().sort, None);
}

#[test]
fn clicking_a_different_column_starts_it_ascending() {
    let probe = sort_probe();
    let mut harness = sort_harness(probe, two_sortable_columns, 20, 100.0);
    harness.run_steps(2);
    click(&mut harness, header_point(probe, 60.0));
    click(&mut harness, header_point(probe, 60.0));
    assert_eq!(
        probe.state.borrow().sort.as_ref().map(|s| s.direction),
        Some(SortDirection::Descending)
    );

    // `size` starts ascending, and `name` loses the sort.
    click(&mut harness, header_point(probe, 180.0));
    assert_eq!(
        probe.state.borrow().sort,
        Some(SortSpec {
            column: "size".into(),
            direction: SortDirection::Ascending
        })
    );
}

fn two_sortable_columns() -> Vec<Column> {
    vec![
        Column::new("name", "Name").width(ColumnWidth::Fixed(120.0)),
        Column::new("size", "Size").width(ColumnWidth::Fixed(120.0)),
    ]
}

#[test]
fn non_sortable_columns_ignore_clicks() {
    let probe = sort_probe();
    let mut harness = sort_harness(probe, sortable_columns, 20, 100.0);
    harness.run_steps(2);
    // `kind` sits after the 120 pt `name` column.
    click(&mut harness, header_point(probe, 180.0));
    assert_eq!(probe.state.borrow().sort, None);
    assert!(!probe.sort_changed.get());
}

#[test]
fn dragging_a_handle_changes_width_within_min_and_max() {
    let probe = sort_probe();
    let mut harness = sort_harness(probe, one_fixed_column, 20, 100.0);
    harness.run_steps(2);
    // Each drag starts on the handle where the column's right border is now.
    let border = |probe: &SortProbe| {
        let width = probe.state.borrow().widths.get("a").copied().unwrap_or(60.0);
        header_point(probe, width)
    };

    drag_by_from(&mut harness, border(probe), 60.0);
    assert!((probe.state.borrow().widths["a"] - 120.0).abs() < 1.0);
    assert_eq!(probe.resized.get(), Some("a"));

    drag_by_from(&mut harness, border(probe), 500.0);
    assert!(
        (probe.state.borrow().widths["a"] - 200.0).abs() < 1.0,
        "max is 200"
    );

    drag_by_from(&mut harness, border(probe), -500.0);
    assert!((probe.state.borrow().widths["a"] - 44.0).abs() < 1.0, "min is 44");
}

/// Drags from `from` by `dx`, with the pointer held down between two frames.
fn drag_by_from(harness: &mut Harness<'static>, from: Pos2, dx: f32) {
    harness.hover_at(from);
    harness.run_steps(2);
    harness.drag_at(from);
    harness.run_steps(2);
    let to = Pos2::new(from.x + dx, from.y);
    harness.event(Event::PointerMoved(to));
    harness.run_steps(2);
    harness.drop_at(to);
    harness.run_steps(2);
}

#[test]
fn double_clicking_a_handle_autofits_to_the_widest_visible_cell() {
    let probe = sort_probe();
    // Cells are 150 pt wide; the column starts at 60 pt.
    let mut harness = sort_harness(probe, one_fixed_column, 20, 150.0);
    harness.run_steps(2);
    let handle = header_point(probe, 60.0);

    // Two raw clicks with no cursor reset between them, so egui sees a double-click.
    raw_click(&mut harness, handle);
    raw_click(&mut harness, handle);
    harness.run_steps(2);

    let width = probe.state.borrow().widths["a"];
    assert!(
        (width - (150.0 + 2.0 * pad())).abs() < 1.0,
        "autofit gave {width}"
    );
    assert_eq!(probe.resized.get(), Some("a"));
}

#[test]
fn an_auto_column_autofits_on_first_show() {
    let probe = sort_probe();
    let mut harness = sort_harness(probe, one_auto_column, 20, 150.0);
    harness.run_steps(3);
    let width = probe.state.borrow().widths["a"];
    assert!(
        (width - (150.0 + 2.0 * pad())).abs() < 1.0,
        "autofit gave {width}"
    );
}

#[cfg(feature = "serde")]
#[test]
fn state_round_trips_through_serde() {
    use fluent2_egui::data::RowId;
    use fluent2_egui::data::data_grid::{GridFocus, Selection};

    let mut state = DataGridState {
        sort: Some(SortSpec {
            column: "name".into(),
            direction: SortDirection::Descending,
        }),
        ..Default::default()
    };
    state.widths.insert("name".into(), 132.5);
    state.selection = Selection {
        selected: [RowId(3), RowId(9)].into_iter().collect(),
        anchor: Some(RowId(3)),
    };
    state.focus = Some(GridFocus {
        row: RowId(9),
        row_index_hint: 4,
        column: Some(1),
        header: false,
    });

    let json = serde_json::to_string(&state).unwrap_or_default();
    let back: Option<DataGridState> = serde_json::from_str(&json).ok();
    assert_eq!(back.as_ref(), Some(&state), "json was {json}");
}
