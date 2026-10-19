//! Docking behaviour (feature `docking`), without the wgpu snapshot feature.
#![cfg(feature = "docking")]

use std::cell::RefCell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::data::SplitSize;
use fluent2_egui::surfaces::{
    DockArea, DockEdge, DockNode, DockSide, DockTree, DockViewer, SideDock, SideDockState, StripTab,
};
use fluent2_egui::{Theme, install};

struct Viewer;

impl DockViewer<String> for Viewer {
    fn tab(&mut self, tab: &String) -> StripTab {
        StripTab::new(tab.clone())
    }

    fn ui(&mut self, ui: &mut egui::Ui, tab: &mut String) {
        ui.label(format!("Body of {tab}"));
    }
}

struct App {
    tree: DockTree<String>,
    closed: Vec<String>,
}

fn two_panes() -> &'static RefCell<App> {
    let mut tree = DockTree::new(vec!["a.rs".to_owned(), "b.rs".to_owned()]);
    let first = tree.pane_ids()[0];
    let _ = tree.split(first, DockSide::Right, "c.rs".to_owned());
    Box::leak(Box::new(RefCell::new(App {
        tree,
        closed: Vec::new(),
    })))
}

fn harness(state: &'static RefCell<App>) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let mut app = state.borrow_mut();
            let response = DockArea::new("dock").show(ui, &mut app.tree, &mut Viewer);
            app.closed.extend(response.closed);
        });
    harness.run();
    harness.run();
    harness
}

fn layout(state: &RefCell<App>) -> Vec<Vec<String>> {
    let app = state.borrow();
    app.tree
        .pane_ids()
        .into_iter()
        .filter_map(|id| app.tree.pane(id).map(|pane| pane.tabs.clone()))
        .collect()
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
fn each_pane_has_a_strip_and_shows_its_selected_tab() {
    let state = two_panes();
    let harness = harness(state);
    assert_eq!(harness.get_all_by_role(Role::TabList).count(), 2);
    let _ = harness.get_by_label("Body of a.rs");
    let _ = harness.get_by_label("Body of c.rs");
}

#[test]
fn closing_the_last_tab_of_a_pane_removes_the_pane() {
    let state = two_panes();
    let mut harness = harness(state);
    harness.get_by_role_and_label(Role::Button, "Close c.rs").click();
    harness.run();
    harness.run();
    assert_eq!(layout(state), [vec!["a.rs", "b.rs"]]);
    assert_eq!(state.borrow().closed, ["c.rs"]);
    assert_eq!(harness.get_all_by_role(Role::TabList).count(), 1);
}

#[test]
fn dragging_a_tab_into_another_pane_moves_it() {
    let state = two_panes();
    let mut harness = harness(state);
    let from = harness.get_by_role_and_label(Role::Tab, "b.rs").rect().center();
    let body = harness.get_by_label("Body of c.rs").rect();
    // The middle of the right pane: join its tabs.
    let to = egui::pos2(600.0, 200.0);
    assert!(body.min.x < to.x);
    drag(&mut harness, from, to);
    assert_eq!(layout(state), [vec!["a.rs"], vec!["c.rs", "b.rs"]]);
}

#[test]
fn dragging_a_tab_to_a_pane_edge_splits_it() {
    let state = two_panes();
    let mut harness = harness(state);
    let from = harness.get_by_role_and_label(Role::Tab, "b.rs").rect().center();
    // Near the bottom edge of the right pane.
    let to = egui::pos2(600.0, 390.0);
    drag(&mut harness, from, to);
    assert_eq!(layout(state), [vec!["a.rs"], vec!["c.rs"], vec!["b.rs"]]);
    assert_eq!(harness.get_all_by_role(Role::TabList).count(), 3);
}

#[test]
fn dropping_a_tab_on_another_strip_joins_that_pane() {
    let state = two_panes();
    let mut harness = harness(state);
    let from = harness.get_by_role_and_label(Role::Tab, "b.rs").rect().center();
    // Just left of c.rs in the right pane's strip: insert before it, no split.
    let c = harness.get_by_role_and_label(Role::Tab, "c.rs").rect();
    let to = egui::pos2(c.min.x + 4.0, c.center().y);
    drag(&mut harness, from, to);
    assert_eq!(layout(state), [vec!["a.rs"], vec!["b.rs", "c.rs"]]);
    assert_eq!(harness.get_all_by_role(Role::TabList).count(), 2);
}

/// `[a]`, split a right (c), then split a down (d): two splits whose first pane is a.
fn nested() -> &'static RefCell<App> {
    let mut tree = DockTree::new(vec!["a.rs".to_owned()]);
    let a = tree.pane_ids()[0];
    let _ = tree.split(a, DockSide::Right, "c.rs".to_owned());
    let _ = tree.split(a, DockSide::Bottom, "d.rs".to_owned());
    Box::leak(Box::new(RefCell::new(App {
        tree,
        closed: Vec::new(),
    })))
}

/// The first-child sizes of the outer and inner splits.
fn split_sizes(state: &RefCell<App>) -> Vec<SplitSize> {
    fn walk(node: &DockNode<String>, out: &mut Vec<SplitSize>) {
        if let DockNode::Split {
            state, first, second, ..
        } = node
        {
            out.push(state.size);
            walk(first, out);
            walk(second, out);
        }
    }
    let mut out = Vec::new();
    if let Some(root) = state.borrow().tree.root() {
        walk(root, &mut out);
    }
    out
}

#[test]
fn nested_splits_with_the_same_first_pane_have_their_own_dividers() {
    let state = nested();
    let mut harness = harness(state);
    let dividers: Vec<egui::Rect> = harness
        .get_all_by_label("Resize panes")
        .map(|node| node.rect())
        .collect();
    assert_eq!(dividers.len(), 2);
    let before = split_sizes(state);
    let tall = dividers.iter().find(|rect| rect.height() > rect.width()).copied();
    let wide = dividers.iter().find(|rect| rect.width() > rect.height()).copied();
    let (Some(tall), Some(wide)) = (tall, wide) else {
        unreachable!("one horizontal and one vertical divider");
    };

    drag(
        &mut harness,
        tall.center(),
        tall.center() + egui::vec2(-80.0, 0.0),
    );
    let after_outer = split_sizes(state);
    assert_ne!(
        after_outer[0], before[0],
        "the outer divider resizes the outer split"
    );
    assert_eq!(after_outer[1], before[1], "and leaves the inner split alone");

    let wide = egui::Rect::from_center_size(wide.center() + egui::vec2(-40.0, 0.0), wide.size());
    drag(&mut harness, wide.center(), wide.center() + egui::vec2(0.0, 60.0));
    let after_inner = split_sizes(state);
    assert_eq!(
        after_inner[0], after_outer[0],
        "the inner divider leaves the outer split alone"
    );
    assert_ne!(after_inner[1], after_outer[1], "and resizes the inner split");
}

#[test]
fn side_dock_closes_and_resizes() {
    let dock = Box::leak(Box::new(RefCell::new(SideDockState::default()))) as &RefCell<SideDockState>;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(800.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let mut state = dock.borrow_mut();
            SideDock::new("explorer", DockEdge::Left, "Explorer").show(
                ui,
                &mut state,
                |ui| ui.label("Files"),
                |ui| ui.label("Editor"),
            );
        });
    harness.run();
    harness.run();
    let files = harness.get_by_label("Files").rect();
    assert!(files.max.x < 260.0, "the dock is its stored width");
    let divider = harness.get_by_label("Resize Explorer").rect().center();
    drag(&mut harness, divider, divider + egui::vec2(60.0, 0.0));
    let size = dock.borrow().size;
    assert!(
        (size - 300.0).abs() < 4.0,
        "dragging the divider resizes, got {size}"
    );
    harness
        .get_by_role_and_label(Role::Button, "Close Explorer")
        .click();
    harness.run();
    harness.run();
    assert!(!dock.borrow().open);
    assert!(harness.query_by_label("Files").is_none());
    let _ = harness.get_by_label("Editor");
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let state = two_panes();
        let mut harness = Harness::builder()
            .with_size(egui::vec2(600.0, 300.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &theme);
                let mut side = SideDockState::default();
                SideDock::new("side", DockEdge::Bottom, "Output").show(
                    ui,
                    &mut side,
                    |ui| ui.label("log"),
                    |ui| {
                        let mut app = state.borrow_mut();
                        DockArea::new("dock").show(ui, &mut app.tree, &mut Viewer);
                    },
                );
                let mut empty: DockTree<String> = DockTree::default();
                DockArea::new("empty").show(ui, &mut empty, &mut Viewer);
            });
        harness.run();
        assert_eq!(harness.get_all_by_role(Role::TabList).count(), 2);
    }
}
