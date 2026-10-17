//! Ribbon tab strip behaviour, without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::office::ribbon::{
    ContextualAccent, FileTab, Ribbon, RibbonDisplay, RibbonEventKind, RibbonState, RibbonTab,
};
use fluent2_egui::{Theme, install};

/// Ribbon state and the flags a test flips between frames. Leaked so the harness closure
/// can hold a `'static` handle.
struct Shared {
    state: RefCell<RibbonState>,
    context_visible: Cell<bool>,
    duplicate_key: Cell<bool>,
    conflicts: RefCell<Vec<&'static str>>,
    tab_changed: Cell<Option<&'static str>>,
    file_clicks: Cell<u32>,
}

fn shared() -> &'static Shared {
    Box::leak(Box::new(Shared {
        state: RefCell::new(RibbonState::default()),
        context_visible: Cell::new(false),
        duplicate_key: Cell::new(false),
        conflicts: RefCell::new(Vec::new()),
        tab_changed: Cell::new(None),
        file_clicks: Cell::new(0),
    }))
}

fn ribbon_harness(shared: &'static Shared) -> Harness<'static> {
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        let mut state = shared.state.borrow_mut();
        let mut ribbon = Ribbon::new("main")
            .tab(RibbonTab::new("home", "Home", |_tab| {}))
            .tab(RibbonTab::new("insert", "Insert", |_tab| {}))
            .tab(RibbonTab::new("table", "Table design", |_tab| {}).contextual(
                "table",
                "Table tools",
                ContextualAccent::Brand,
                shared.context_visible.get(),
            ));
        ribbon = ribbon.file_tab(FileTab::new("file", "File"));
        if shared.duplicate_key.get() {
            ribbon = ribbon.tab(RibbonTab::new("home", "Home again", |_tab| {}));
        }
        let response = ribbon.show(ui, &mut state);
        let file_clicks = response
            .events
            .iter()
            .filter(|event| event.key == "file" && event.kind == RibbonEventKind::Clicked)
            .count();
        shared
            .file_clicks
            .set(shared.file_clicks.get() + file_clicks as u32);
        shared.conflicts.replace(response.conflicts);
        if response.tab_changed.is_some() {
            shared.tab_changed.set(response.tab_changed);
        }
    });
    harness.run();
    harness.run();
    harness
}

/// A pointer click at `pos`, then a frame to process it.
fn click_at(harness: &mut Harness<'static>, pos: Pos2) {
    harness.event(Event::PointerMoved(pos));
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
}

/// Centre-left point of the tab strip. Clicks here land on the first tab.
fn strip_first_tab(harness: &Harness<'static>) -> Pos2 {
    let rect = harness.get_by_role_and_label(Role::TabList, "Ribbon tabs").rect();
    Pos2::new(rect.left() + 4.0, rect.center().y)
}

fn strip_centre(harness: &Harness<'static>) -> Pos2 {
    harness
        .get_by_role_and_label(Role::TabList, "Ribbon tabs")
        .rect()
        .center()
}

fn outside_point() -> Pos2 {
    Pos2::new(5.0, 500.0)
}

/// Two clicks in consecutive frames, 0.1 s apart on the input clock, so egui reads the
/// second as a double-click. `at` is the time of the first click and must be well after
/// the previous pair, so the triple-click window does not apply.
fn double_click_at(harness: &mut Harness<'static>, pos: Pos2, at: f64) {
    harness.run();
    harness.event(Event::PointerMoved(pos));
    for time in [at, at + 0.1] {
        harness.input_mut().time = Some(time);
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        harness.step();
    }
    harness.input_mut().time = None;
    harness.run();
}

#[test]
fn tabs_select_by_click_and_arrow_keys() {
    let shared = shared();
    shared.state.borrow_mut().active = Some("insert");
    let mut harness = ribbon_harness(shared);
    let pos = strip_first_tab(&harness);
    click_at(&mut harness, pos);
    assert_eq!(shared.state.borrow().active, Some("home"));
    assert_eq!(shared.tab_changed.get(), Some("home"));

    harness.key_press(Key::ArrowRight);
    harness.run();
    assert_eq!(shared.state.borrow().active, Some("insert"));
    harness.key_press(Key::ArrowLeft);
    harness.run();
    assert_eq!(shared.state.borrow().active, Some("home"));
}

#[test]
fn contextual_tabs_appear_only_when_visible() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    assert_eq!(harness.query_all_by_role(Role::Tab).count(), 2);

    shared.context_visible.set(true);
    harness.run();
    assert_eq!(harness.query_all_by_role(Role::Tab).count(), 3);
}

#[test]
fn active_contextual_tab_hands_over_to_first_content_tab() {
    let shared = shared();
    shared.context_visible.set(true);
    shared.state.borrow_mut().active = Some("table");
    let mut harness = ribbon_harness(shared);
    assert_eq!(shared.state.borrow().active, Some("table"));

    shared.context_visible.set(false);
    harness.run();
    assert_eq!(shared.state.borrow().active, Some("home"));
    assert_eq!(shared.tab_changed.get(), Some("home"));
}

#[test]
fn duplicate_keys_are_reported_and_the_first_wins() {
    let shared = shared();
    shared.duplicate_key.set(true);
    let _harness = ribbon_harness(shared);
    assert_eq!(*shared.conflicts.borrow(), vec!["home"]);
}

#[test]
fn double_click_toggles_collapsed() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    let pos = strip_centre(&harness);
    double_click_at(&mut harness, pos, 100.0);
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Collapsed);

    let pos = strip_centre(&harness);
    double_click_at(&mut harness, pos, 200.0);
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Expanded);
}

#[test]
fn ctrl_f1_toggles() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    harness.key_press_modifiers(Modifiers::CTRL, Key::F1);
    harness.run();
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Collapsed);

    harness.key_press_modifiers(Modifiers::CTRL, Key::F1);
    harness.run();
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Expanded);
}

#[test]
fn collapsed_tab_click_peeks_and_outside_click_closes() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    harness.key_press_modifiers(Modifiers::CTRL, Key::F1);
    harness.run();

    let pos = strip_first_tab(&harness);
    click_at(&mut harness, pos);
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Peek);
    assert_eq!(shared.state.borrow().active, Some("home"));

    click_at(&mut harness, outside_point());
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Collapsed);
}

#[test]
fn escape_closes_the_peek() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    harness.key_press_modifiers(Modifiers::CTRL, Key::F1);
    harness.run();
    let pos = strip_first_tab(&harness);
    click_at(&mut harness, pos);
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Peek);

    harness.key_press(Key::Escape);
    harness.run();
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Collapsed);
}

#[test]
fn pinned_peek_stays_open_on_outside_click() {
    let shared = shared();
    shared.state.borrow_mut().display = RibbonDisplay::Peek;
    shared.state.borrow_mut().pinned = true;
    let mut harness = ribbon_harness(shared);
    click_at(&mut harness, outside_point());
    assert_eq!(shared.state.borrow().display, RibbonDisplay::Peek);
}

#[test]
fn ribbon_state_defaults_to_expanded_first_tab() {
    let state = RibbonState::default();
    assert_eq!(state.display, RibbonDisplay::Expanded);
    assert_eq!(state.active, None);
    assert!(!state.pinned);
}

#[test]
fn file_tab_click_reports_clicked_and_enter_activates() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    harness.get_by_role_and_label(Role::Button, "File").click();
    harness.run();
    assert_eq!(shared.file_clicks.get(), 1);

    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(shared.file_clicks.get(), 2);
}

#[test]
fn contextual_header_shows_only_while_the_set_is_visible() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    assert!(!header_shown(&harness, "Table tools"));

    shared.context_visible.set(true);
    harness.run();
    assert!(header_shown(&harness, "Table tools"));
}

/// Whether the contextual header with `text` is in the accessibility tree. A header is a
/// `Label` node, which keeps its text in `value`.
fn header_shown(harness: &Harness<'static>, text: &str) -> bool {
    harness
        .query_all(egui_kittest::kittest::by().role(Role::Label).value(text))
        .next()
        .is_some()
}
