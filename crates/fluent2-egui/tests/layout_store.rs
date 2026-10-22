//! LayoutStore (feature `serde`, issue #118): layout state survives a save and a fresh start.
#![cfg(all(feature = "serde", feature = "docking"))]

use std::cell::RefCell;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::data_grid::DataGridState;
use fluent2_egui::data::{SplitSize, SplitState};
use fluent2_egui::platform::LayoutStore;
use fluent2_egui::surfaces::{DockEdge, DockSide, DockTree, SideDock, SideDockState};
use fluent2_egui::theme::Density;
use fluent2_egui::{Theme, install};

#[test]
fn every_layout_state_round_trips_through_one_document() {
    let split = SplitState {
        size: SplitSize::Ratio(0.25),
    };
    let side = SideDockState {
        open: false,
        size: 310.0,
    };
    let mut tree = DockTree::new(vec!["a.rs".to_owned(), "b.rs".to_owned()]);
    let first = tree.pane_ids()[0];
    let _ = tree.split(first, DockSide::Right, "c.rs".to_owned());
    let mut grid = DataGridState::default();
    grid.widths.insert("name".to_owned(), 240.0);

    let mut store = LayoutStore::new();
    store.put("split", &split);
    store.put("explorer", &side);
    store.put("tree", &tree);
    store.put("grid", &grid);
    store.put("density", &Density::Compact);
    assert!(store.is_dirty());

    let loaded = LayoutStore::from_json(&store.to_json());
    assert!(!loaded.is_dirty());
    assert_eq!(
        loaded.keys().collect::<Vec<_>>(),
        ["density", "explorer", "grid", "split", "tree"]
    );
    assert_eq!(loaded.get::<SplitState>("split"), Some(split));
    assert_eq!(loaded.get::<SideDockState>("explorer"), Some(side));
    assert_eq!(loaded.get::<DockTree<String>>("tree"), Some(tree));
    assert_eq!(loaded.get::<DataGridState>("grid"), Some(grid));
    assert_eq!(loaded.get::<Density>("density"), Some(Density::Compact));
}

#[test]
fn unreadable_or_stale_entries_keep_defaults() {
    assert_eq!(LayoutStore::from_json("not json"), LayoutStore::new());
    assert_eq!(LayoutStore::from_json("[1, 2]"), LayoutStore::new());

    let store = LayoutStore::from_json(r#"{"split": {"size": "sideways"}, "other": 3}"#);
    let mut split = SplitState::default();
    assert!(!store.restore("split", &mut split));
    assert_eq!(split, SplitState::default());
    assert!(!store.restore("missing", &mut split));
    // Entries this app does not read survive a save.
    assert!(store.to_json().contains("\"other\":3"));
}

#[test]
fn put_marks_dirty_only_on_change_and_save_clears_it() {
    let mut store = LayoutStore::new();
    store.put("density", &Density::Touch);
    let mut saved = None;
    store.save(|key, value| saved = Some((key.to_owned(), value)));
    assert!(!store.is_dirty());
    store.put("density", &Density::Touch);
    assert!(!store.is_dirty(), "an unchanged value is not a change");
    store.put("density", &Density::Medium);
    assert!(store.is_dirty());

    let (key, json) = saved.unwrap_or_default();
    assert_eq!(key, LayoutStore::STORAGE_KEY);
    let loaded = LayoutStore::load(|key| (key == LayoutStore::STORAGE_KEY).then(|| json.clone()));
    assert_eq!(loaded.get::<Density>("density"), Some(Density::Touch));
    assert_eq!(LayoutStore::load(|_| None), LayoutStore::new());

    store.clear();
    assert_eq!(store.keys().count(), 0);
    assert!(!store.remove("density"));
}

fn side_dock_harness(state: &'static RefCell<SideDockState>) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let mut state = state.borrow_mut();
            SideDock::new("explorer", DockEdge::Left, "Explorer").show(
                ui,
                &mut state,
                |ui| ui.label("Files"),
                |ui| ui.label("Editor"),
            );
        });
    harness.run();
    harness.run();
    harness
}

fn drag(harness: &mut Harness<'static>, from: egui::Pos2, to: egui::Pos2) {
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    for step in 1..=8 {
        harness.hover_at(from.lerp(to, step as f32 / 8.0));
        harness.run();
    }
    harness.drop_at(to);
    harness.run();
    harness.run();
}

#[test]
fn a_resized_side_dock_comes_back_after_a_restart() {
    let first = Box::leak(Box::new(RefCell::new(SideDockState::default())));
    let mut harness = side_dock_harness(first);
    let divider = harness.get_by_label("Resize Explorer").rect().center();
    drag(&mut harness, divider, divider + egui::vec2(80.0, 0.0));
    let resized = first.borrow().size;
    assert!(resized > 300.0, "the drag resized the dock, got {resized}");
    let mut store = LayoutStore::new();
    store.put("explorer", &*first.borrow());
    let saved = store.to_json();
    drop(harness);

    // A fresh start: default state, then the saved layout.
    let mut restored = SideDockState::default();
    assert!(LayoutStore::from_json(&saved).restore("explorer", &mut restored));
    let second = Box::leak(Box::new(RefCell::new(restored)));
    let harness = side_dock_harness(second);
    let editor = harness.get_by_label("Editor").rect();
    assert!(
        editor.min.x > resized,
        "the dock opens at its saved width, the editor starts at {}",
        editor.min.x
    );
    assert_eq!(second.borrow().size, resized);
}

#[test]
fn a_collapsed_side_dock_stays_collapsed_after_a_restart() {
    let mut store = LayoutStore::new();
    store.put(
        "explorer",
        &SideDockState {
            open: false,
            size: 240.0,
        },
    );
    let mut restored = SideDockState::default();
    assert!(LayoutStore::from_json(&store.to_json()).restore("explorer", &mut restored));
    let state = Box::leak(Box::new(RefCell::new(restored)));
    let harness = side_dock_harness(state);
    assert!(harness.query_by_label("Files").is_none());
    let _ = harness.get_by_label("Editor");
}
