//! Ribbon groups and controls (spec §4.1, §4.2, §9.1), without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::accesskit::{self, Role};
use egui::{Event, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::office::ribbon::{
    FileTab, GroupBuilder, Ribbon, RibbonCheckbox, RibbonCombobox, RibbonCustom, RibbonDialogLauncher,
    RibbonDisplay, RibbonDropdown, RibbonEvent, RibbonEventKind, RibbonLarge, RibbonMenu, RibbonSmall,
    RibbonSplit, RibbonState, RibbonTab, RibbonToggle, TabBuilder,
};
use fluent2_egui::{IconName, Theme, install};

/// The values the controls are bound to, and what the harness reports.
#[derive(Default)]
struct Values {
    bold: bool,
    ruler: bool,
    font: usize,
    size: String,
}

struct Shared {
    state: RefCell<RibbonState>,
    values: RefCell<Values>,
    events: RefCell<Vec<RibbonEvent>>,
    conflicts: RefCell<Vec<&'static str>>,
    custom: RefCell<Vec<&'static str>>,
    disabled: Cell<bool>,
    duplicate: Cell<bool>,
}

fn shared() -> &'static Shared {
    Box::leak(Box::new(Shared {
        state: RefCell::new(RibbonState::default()),
        values: RefCell::new(Values::default()),
        events: RefCell::new(Vec::new()),
        conflicts: RefCell::new(Vec::new()),
        custom: RefCell::new(Vec::new()),
        disabled: Cell::new(false),
        duplicate: Cell::new(false),
    }))
}

/// Home: Clipboard and Font groups. Insert: one group, optionally reusing the key `paste`.
fn build<'a>(values: &'a mut Values, duplicate: bool) -> Ribbon<'a> {
    let Values {
        bold,
        ruler,
        font,
        size,
    } = values;
    let mut ribbon = Ribbon::new("main")
        .file_tab(FileTab::new("file", "File"))
        .tab(RibbonTab::new("home", "Home", move |tab: &mut TabBuilder<'a>| {
            tab.group("clipboard", "Clipboard", |group: &mut GroupBuilder<'a>| {
                group.add(RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste"));
                group.add(RibbonSmall::new("cut", IconName::Cut, "Cut"));
                group.add(RibbonSplit::new(
                    "format",
                    IconName::ClipboardPaste,
                    "Format",
                    |ui| {
                        ui.label("Format item");
                    },
                ));
                group.dialog_launcher(RibbonDialogLauncher::new("clipboard_dialog"));
            });
            tab.group("font", "Font", |group: &mut GroupBuilder<'a>| {
                group.add(RibbonToggle::new("bold", IconName::TextBold, "Bold", bold));
                group.add(RibbonMenu::new("more", IconName::Settings, "More", |ui| {
                    ui.label("More item");
                }));
                group.add(RibbonCheckbox::new("ruler", "Ruler", ruler));
                group.add(RibbonDropdown::new("font_name", "Font", &["Sans", "Serif"], font));
                group.add(RibbonCombobox::new("size", "Size", &["8", "10"], size));
                group.add(RibbonCustom::new("zoom", |ui, _step| ui.button("Zoom")).widths(48.0, 32.0, 24.0));
            });
        }));
    if duplicate {
        ribbon = ribbon.tab(RibbonTab::new(
            "insert",
            "Insert",
            move |tab: &mut TabBuilder<'a>| {
                tab.group("extra", "Extra", |group: &mut GroupBuilder<'a>| {
                    group.add(RibbonSmall::new("paste", IconName::Copy, "Copy"));
                });
            },
        ));
    } else {
        ribbon = ribbon.tab(RibbonTab::new("insert", "Insert", |_tab: &mut TabBuilder<'a>| {}));
    }
    ribbon
}

/// The surface is wide enough for the whole Home tab, peek popup included.
fn ribbon_harness(shared: &'static Shared) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1600.0, 400.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            let mut state = shared.state.borrow_mut();
            let mut values = shared.values.borrow_mut();
            let ribbon = build(&mut values, shared.duplicate.get());
            let disabled = shared.disabled.get();
            let response = ui
                .add_enabled_ui(!disabled, |ui| ribbon.show(ui, &mut state))
                .inner;
            shared.events.borrow_mut().extend(response.events);
            shared.conflicts.borrow_mut().extend(response.conflicts);
            shared
                .custom
                .borrow_mut()
                .extend(response.custom.iter().map(|(key, _)| *key));
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

fn click_label(harness: &mut Harness<'static>, role: Role, label: &str) {
    let rect = harness.get_by_role_and_label(role, label).rect();
    click_at(harness, rect.center());
}

fn take_events(shared: &'static Shared) -> Vec<RibbonEvent> {
    shared.events.take()
}

fn event(key: &'static str, kind: RibbonEventKind) -> RibbonEvent {
    RibbonEvent { key, kind }
}

#[test]
fn large_button_reports_its_key_on_click() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::Button, "Paste");
    assert!(take_events(shared).contains(&event("paste", RibbonEventKind::Clicked)));
}

#[test]
fn small_button_reports_its_key_on_click() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::Button, "Cut");
    assert!(take_events(shared).contains(&event("cut", RibbonEventKind::Clicked)));
}

#[test]
fn toggle_reports_changed_and_updates_its_value() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::Button, "Bold");
    assert!(shared.values.borrow().bold, "the toggle writes its value");
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Bold")
            .accesskit_node()
            .toggled(),
        Some(accesskit::Toggled::True),
        "the toggle exposes its toggled state"
    );
    assert!(take_events(shared).contains(&event("bold", RibbonEventKind::Changed)));
}

/// Click the label of an option in an open popup. Options are not buttons, so they are found by label.
fn click_option(harness: &mut Harness<'static>, label: &str) {
    let rect = harness.get_by_label(label).rect();
    click_at(harness, rect.center());
}

#[test]
fn dropdown_pick_reports_changed_on_its_key_and_writes_the_index() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::ComboBox, "Font");
    harness.run();
    click_option(&mut harness, "Serif");
    harness.run();
    assert_eq!(shared.values.borrow().font, 1, "the picked index is written");
    assert!(take_events(shared).contains(&event("font_name", RibbonEventKind::Changed)));
}

#[test]
fn combobox_pick_reports_changed_on_its_key_and_copies_the_option() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    // Focus the field and type a filter. Typing opens the option list (WI-1549).
    click_label(&mut harness, Role::ComboBox, "Size");
    harness.run();
    harness
        .get_by_role_and_label(Role::ComboBox, "Size")
        .type_text("1");
    harness.run();
    click_option(&mut harness, "10");
    harness.run();
    assert_eq!(
        shared.values.borrow().size,
        "10",
        "the picked option is copied into the text"
    );
    assert!(take_events(shared).contains(&event("size", RibbonEventKind::Changed)));
}

#[test]
fn checkbox_reports_changed() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::CheckBox, "Ruler");
    assert!(shared.values.borrow().ruler);
    assert!(take_events(shared).contains(&event("ruler", RibbonEventKind::Changed)));
}

#[test]
fn split_main_action_reports_and_arrow_opens_menu_without_reporting() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);

    click_label(&mut harness, Role::Button, "Format");
    assert!(take_events(shared).contains(&event("format", RibbonEventKind::Clicked)));

    click_label(&mut harness, Role::Button, "Format menu");
    harness.run();
    assert!(
        harness.query_by_label("Format item").is_some(),
        "the arrow opens the menu"
    );
    assert!(
        !take_events(shared).iter().any(|event| event.key == "format"),
        "the arrow does not trigger the main action"
    );
}

#[test]
fn menu_button_reports_when_pressed_to_open() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::Button, "More");
    assert!(take_events(shared).contains(&event("more", RibbonEventKind::Clicked)));
}

#[test]
fn dialog_launcher_reports_its_key() {
    let shared = shared();
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::Button, "Clipboard dialog");
    assert!(take_events(shared).contains(&event("clipboard_dialog", RibbonEventKind::Clicked)));
}

#[test]
fn custom_widget_response_is_returned_under_its_key() {
    let shared = shared();
    let _harness = ribbon_harness(shared);
    assert!(shared.custom.borrow().contains(&"zoom"));
}

#[test]
fn disabled_controls_report_nothing() {
    let shared = shared();
    shared.disabled.set(true);
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::Button, "Paste");
    click_label(&mut harness, Role::Button, "Bold");
    assert!(take_events(shared).is_empty());
    assert!(!shared.values.borrow().bold, "a disabled toggle does not change");
}

#[test]
fn group_and_control_roles_follow_spec_9_1() {
    let shared = shared();
    let harness = ribbon_harness(shared);
    harness.get_by_role_and_label(Role::Group, "Clipboard");
    harness.get_by_role_and_label(Role::Button, "Paste");
    harness.get_by_role_and_label(Role::Button, "Clipboard dialog");
    harness.get_by_role_and_label(Role::TabList, "Ribbon tabs");
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Bold")
            .accesskit_node()
            .toggled(),
        Some(accesskit::Toggled::False),
        "an unpressed toggle is exposed as not toggled"
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "More")
            .accesskit_node()
            .has_popup(),
        Some(accesskit::HasPopup::Menu),
        "a menu button has a popup"
    );
    assert_eq!(
        harness
            .get_by_role_and_label(Role::Button, "Format menu")
            .accesskit_node()
            .has_popup(),
        Some(accesskit::HasPopup::Menu),
        "the split arrow has a popup"
    );
}

#[test]
fn repeated_control_key_is_reported_and_first_occurrence_keeps_its_control() {
    let shared = shared();
    shared.duplicate.set(true);
    let mut harness = ribbon_harness(shared);
    assert!(shared.conflicts.borrow().contains(&"paste"));

    click_label(&mut harness, Role::Button, "Paste");
    assert_eq!(
        take_events(shared)
            .iter()
            .filter(|event| event.key == "paste")
            .count(),
        1
    );
}

#[test]
fn pin_button_in_the_peek_pins_the_ribbon_open() {
    let shared = shared();
    shared.state.borrow_mut().display = RibbonDisplay::Peek;
    let mut harness = ribbon_harness(shared);
    click_label(&mut harness, Role::Button, "Pin the ribbon");
    let state = shared.state.borrow();
    assert!(state.pinned);
    assert_eq!(state.display, RibbonDisplay::Expanded);
}

#[test]
fn groups_render_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::light(fluent2_tokens::FLUENT),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut values = Values::default();
        let mut state = RibbonState::default();
        let mut harness = Harness::builder()
            .with_size(Vec2::new(1600.0, 400.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &theme);
                let _ = build(&mut values, false).show(ui, &mut state);
            });
        harness.run();
        harness.get_by_role_and_label(Role::Button, "Paste");
    }
}
