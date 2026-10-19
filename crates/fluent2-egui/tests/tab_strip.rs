//! TabStrip behaviour, without the wgpu snapshot feature.

use std::cell::RefCell;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::surfaces::{StripTab, TabMove, TabStrip};
use fluent2_egui::{Theme, install};

/// What the app keeps: its documents, the selection and the last move seen.
struct App {
    docs: Vec<String>,
    dirty: Vec<String>,
    selected: usize,
    moves: Vec<TabMove>,
    focus: bool,
    shortcuts: bool,
}

fn app(names: &[&str]) -> &'static RefCell<App> {
    // Each test owns its own state; a leaked box gives the closure a 'static handle.
    Box::leak(Box::new(RefCell::new(App {
        docs: names.iter().map(|name| (*name).to_owned()).collect(),
        dirty: Vec::new(),
        selected: 0,
        moves: Vec::new(),
        focus: true,
        shortcuts: false,
    })))
}

fn harness(state: &'static RefCell<App>, width: f32) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(width, 120.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let mut app = state.borrow_mut();
            let tabs: Vec<StripTab> = app
                .docs
                .iter()
                .map(|name| StripTab::new(name.clone()).dirty(app.dirty.contains(name)))
                .collect();
            let mut selected = app.selected;
            let response = TabStrip::new("docs", &mut selected)
                .tabs(tabs)
                .shortcuts(app.shortcuts)
                .show(ui);
            if app.focus {
                response.response.request_focus();
            }
            app.selected = selected;
            if let Some(moved) = response.moved {
                app.moves.push(moved);
            }
            let App { docs, selected, .. } = &mut *app;
            response.apply(docs, selected);
            let _ = ui.button("After");
        });
    harness.run();
    harness.run();
    harness
}

fn labels(state: &RefCell<App>) -> Vec<String> {
    state.borrow().docs.clone()
}

#[test]
fn clicking_a_tab_selects_it() {
    let state = app(&["a.rs", "b.rs", "c.rs"]);
    let mut harness = harness(state, 800.0);
    harness.get_by_role_and_label(Role::Tab, "c.rs").click();
    harness.run();
    assert_eq!(state.borrow().selected, 2);
}

#[test]
fn close_button_closes_that_tab() {
    let state = app(&["a.rs", "b.rs", "c.rs"]);
    state.borrow_mut().selected = 2;
    let mut harness = harness(state, 800.0);
    harness.get_by_role_and_label(Role::Button, "Close a.rs").click();
    harness.run();
    assert_eq!(labels(state), ["b.rs", "c.rs"]);
    // The selection stays on the same document.
    assert_eq!(state.borrow().selected, 1);
}

#[test]
fn middle_click_closes() {
    let state = app(&["a.rs", "b.rs"]);
    let mut harness = harness(state, 800.0);
    harness
        .get_by_role_and_label(Role::Tab, "b.rs")
        .click_button(egui::PointerButton::Middle);
    harness.run();
    assert_eq!(labels(state), ["a.rs"]);
}

#[test]
fn ctrl_w_closes_the_selected_tab() {
    let state = app(&["a.rs", "b.rs", "c.rs"]);
    state.borrow_mut().selected = 1;
    let mut harness = harness(state, 800.0);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::W);
    harness.run();
    assert_eq!(labels(state), ["a.rs", "c.rs"]);
    assert_eq!(state.borrow().selected, 1, "the next tab takes the place");
}

#[test]
fn ctrl_tab_cycles_and_wraps() {
    let state = app(&["a.rs", "b.rs", "c.rs"]);
    let mut harness = harness(state, 800.0);
    harness.key_press_modifiers(Modifiers::CTRL, Key::Tab);
    harness.run();
    assert_eq!(state.borrow().selected, 1);
    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab);
    harness.run();
    harness.key_press_modifiers(Modifiers::CTRL | Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(state.borrow().selected, 2, "Ctrl+Shift+Tab wraps to the end");
    harness.key_press_modifiers(Modifiers::CTRL, Key::Tab);
    harness.run();
    assert_eq!(state.borrow().selected, 0, "Ctrl+Tab wraps to the start");
}

#[test]
fn shortcuts_work_without_focus_only_when_asked() {
    let state = app(&["a.rs", "b.rs"]);
    state.borrow_mut().focus = false;
    let mut harness = harness(state, 800.0);
    harness.key_press_modifiers(Modifiers::CTRL, Key::Tab);
    harness.run();
    assert_eq!(state.borrow().selected, 0, "an unfocused strip ignores Ctrl+Tab");
    state.borrow_mut().shortcuts = true;
    harness.run();
    harness.key_press_modifiers(Modifiers::CTRL, Key::Tab);
    harness.run();
    assert_eq!(state.borrow().selected, 1);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::W);
    harness.run();
    assert_eq!(labels(state), ["a.rs"]);
}

#[test]
fn arrows_select_and_ctrl_shift_arrows_move() {
    let state = app(&["a.rs", "b.rs", "c.rs"]);
    let mut harness = harness(state, 800.0);
    harness.key_press(Key::End);
    harness.run();
    assert_eq!(state.borrow().selected, 2);
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(state.borrow().selected, 1);
    harness.key_press_modifiers(Modifiers::COMMAND | Modifiers::SHIFT, Key::ArrowLeft);
    harness.run();
    assert_eq!(labels(state), ["b.rs", "a.rs", "c.rs"]);
    assert_eq!(state.borrow().selected, 0, "the moved tab stays selected");
}

#[test]
fn dragging_a_tab_reorders() {
    let state = app(&["a.rs", "b.rs", "c.rs", "d.rs"]);
    let mut harness = harness(state, 800.0);
    let from = harness.get_by_role_and_label(Role::Tab, "a.rs").rect().center();
    let over = harness.get_by_role_and_label(Role::Tab, "c.rs").rect();
    let to = egui::pos2(over.max.x - 2.0, over.center().y);
    harness.hover_at(from);
    harness.run();
    harness.drag_at(from);
    harness.run();
    // Move in steps so egui decides this is a drag, not a click.
    for step in 1..=5 {
        harness.hover_at(from.lerp(to, step as f32 / 5.0));
        harness.run();
    }
    harness.drop_at(to);
    harness.run();
    harness.run();
    assert_eq!(state.borrow().moves, [TabMove { from: 0, to: 2 }]);
    assert_eq!(labels(state), ["b.rs", "c.rs", "a.rs", "d.rs"]);
    assert_eq!(state.borrow().selected, 2, "the dragged tab is selected");
}

#[test]
fn overflow_keeps_the_selection_visible_and_lists_every_tab() {
    let names: Vec<String> = (0..12).map(|i| format!("document-{i}.txt")).collect();
    let refs: Vec<&str> = names.iter().map(String::as_str).collect();
    let state = app(&refs);
    let mut harness = harness(state, 400.0);
    let shown = harness.get_all_by_role(Role::Tab).count();
    assert!(shown < 12, "only {shown} of 12 tabs fit");
    assert!(
        harness
            .query_by_role_and_label(Role::Tab, "document-11.txt")
            .is_none(),
        "the last tab is hidden"
    );
    // Selecting a hidden tab with End scrolls it into view.
    harness.key_press(Key::End);
    harness.run();
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Tab, "document-11.txt");
    // The overflow menu lists every tab, hidden ones included; choosing one selects it.
    harness.get_by_role_and_label(Role::Button, "More tabs").click();
    harness.run();
    harness.get_by_label("document-0.txt").click();
    harness.run();
    harness.run();
    assert_eq!(state.borrow().selected, 0);
    let _ = harness.get_by_role_and_label(Role::Tab, "document-0.txt");
}

#[test]
fn no_overflow_button_when_everything_fits() {
    let state = app(&["a.rs", "b.rs"]);
    let harness = harness(state, 800.0);
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "More tabs")
            .is_none()
    );
    assert_eq!(harness.get_all_by_role(Role::Tab).count(), 2);
}

#[test]
fn dirty_tab_is_announced_and_still_closable() {
    let state = app(&["a.rs", "b.rs"]);
    state.borrow_mut().dirty.push("b.rs".to_owned());
    let mut harness = harness(state, 800.0);
    let tab = harness.get_by_role_and_label(Role::Tab, "b.rs");
    assert_eq!(tab.accesskit_node().description().as_deref(), Some("Modified"));
    harness.get_by_role_and_label(Role::Button, "Close b.rs").click();
    harness.run();
    assert_eq!(labels(state), ["a.rs"]);
}

#[test]
fn one_tab_stop_for_the_tabs() {
    let state = app(&["a.rs", "b.rs"]);
    let mut harness = harness(state, 800.0);
    // Stop re-requesting focus, then Tab once: focus goes from the strip straight to the
    // next control, not to a second tab or a close button.
    state.borrow_mut().focus = false;
    harness.key_press(Key::Tab);
    harness.run();
    assert!(harness.get_by_role_and_label(Role::Button, "After").is_focused());
    let strip = harness.get_by_role(Role::TabList);
    assert_eq!(strip.accesskit_node().label().as_deref(), Some("Open documents"));
}

#[test]
fn disabled_strip_ignores_input() {
    let state = app(&["a.rs", "b.rs"]);
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        let mut app = state.borrow_mut();
        let tabs: Vec<StripTab> = app.docs.iter().map(|n| StripTab::new(n.clone())).collect();
        let mut selected = app.selected;
        ui.add_enabled_ui(false, |ui| {
            let response = TabStrip::new("docs", &mut selected).tabs(tabs).show(ui);
            assert!(response.closed.is_none());
        });
        app.selected = selected;
    });
    harness.run();
    harness.get_by_role_and_label(Role::Tab, "b.rs").click();
    harness.run();
    assert_eq!(state.borrow().selected, 0);
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(200.0, 80.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &theme);
                let mut selected = 1;
                let _ = TabStrip::new("docs", &mut selected)
                    .tab(StripTab::new("one").icon(fluent2_icons::Icon::Document))
                    .tab(StripTab::new("two").dirty(true))
                    .tab(StripTab::new("pinned").closable(false))
                    .tab(StripTab::new("a very long document name that will be truncated"))
                    .show(ui);
                let mut none = 0;
                let _ = TabStrip::new("empty", &mut none).show(ui);
            });
        harness.run();
        assert_eq!(harness.get_all_by_role(Role::TabList).count(), 2);
    }
}
