//! KeyTips badges and activation (spec §6) and F6 region cycling (spec §9.3), without the wgpu
//! snapshot feature.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, PointerButton, Pos2, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::office::keytips::{self, KeyTipLevel, KeyTipState, KeyTipTarget, Region};
use fluent2_egui::office::qat::{Qat, QatItem, QatPosition, QatState};
use fluent2_egui::office::ribbon::{
    FileTab, GroupBuilder, Ribbon, RibbonDialogLauncher, RibbonEvent, RibbonEventKind, RibbonLarge,
    RibbonMenu, RibbonSmall, RibbonState, RibbonTab, RibbonToggle, TabBuilder,
};
use fluent2_egui::office::status_bar::StatusBar;
use fluent2_egui::{IconName, Theme, install};
use fluent2_icons::Icon;

struct Shared {
    ribbon: RefCell<RibbonState>,
    qat: RefCell<QatState>,
    bold: Cell<bool>,
    zoom: Cell<f32>,
    text: RefCell<String>,
    disabled: Cell<bool>,
    theme: RefCell<Theme>,
    events: RefCell<Vec<RibbonEvent>>,
    tab_changed: RefCell<Vec<&'static str>>,
    qat_clicked: RefCell<Vec<&'static str>>,
}

fn shared() -> &'static Shared {
    Box::leak(Box::new(Shared {
        ribbon: RefCell::new(RibbonState::default()),
        qat: RefCell::new(QatState {
            items: vec!["save", "undo"],
            position: QatPosition::AboveRibbon,
            customize_open: false,
        }),
        bold: Cell::new(false),
        zoom: Cell::new(100.0),
        text: RefCell::new(String::new()),
        disabled: Cell::new(false),
        theme: RefCell::new(Theme::default()),
        events: RefCell::new(Vec::new()),
        tab_changed: RefCell::new(Vec::new()),
        qat_clicked: RefCell::new(Vec::new()),
    }))
}

const OFFERED: [QatItem<'static>; 2] = [
    QatItem {
        key: "save",
        icon: Icon::Save,
        label: "Save",
        tooltip: "Save",
        keytip: Some("1"),
    },
    QatItem {
        key: "undo",
        icon: Icon::Print,
        label: "Undo",
        tooltip: "Undo",
        keytip: Some("2"),
    },
];

/// QAT above the ribbon, the ribbon, a text editor as content, and a status bar with zoom.
fn harness(shared: &'static Shared) -> Harness<'static> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(1400.0, 500.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &shared.theme.borrow());
            let qat = Qat::new("qat", &mut shared.qat.borrow_mut(), &OFFERED).show(ui);
            shared.qat_clicked.borrow_mut().extend(qat.clicked);

            let mut bold = shared.bold.get();
            let ribbon = Ribbon::new("main")
                .file_tab(FileTab::new("file", "File").keytip("F"))
                .tab(
                    RibbonTab::new("home", "Home", |tab: &mut TabBuilder<'_>| {
                        tab.group("clipboard", "Clipboard", |group: &mut GroupBuilder<'_>| {
                            group.add(
                                RibbonLarge::new("paste", IconName::ClipboardPaste, "Paste").keytip("V"),
                            );
                            group.add(RibbonSmall::new("cut", IconName::Cut, "Cut").keytip("X"));
                            group.dialog_launcher(RibbonDialogLauncher::new("clipboard_dialog").keytip("FO"));
                        });
                        tab.group("font", "Font", |group: &mut GroupBuilder<'_>| {
                            group.add(
                                RibbonToggle::new("bold", IconName::TextBold, "Bold", &mut bold).keytip("1"),
                            );
                            group.add(
                                RibbonMenu::new("more", IconName::Settings, "More", |ui| {
                                    ui.label("More item");
                                })
                                .keytip("M"),
                            );
                        });
                    })
                    .keytip("H"),
                )
                .tab(RibbonTab::new("insert", "Insert", |tab: &mut TabBuilder<'_>| {
                    tab.group("pages", "Pages", |group: &mut GroupBuilder<'_>| {
                        group.add(RibbonLarge::new("blank", IconName::Add, "Blank page"));
                    });
                }));
            let response = ui
                .add_enabled_ui(!shared.disabled.get(), |ui| {
                    ribbon.show(ui, &mut shared.ribbon.borrow_mut())
                })
                .inner;
            shared.bold.set(bold);
            shared.events.borrow_mut().extend(response.events);
            shared.tab_changed.borrow_mut().extend(response.tab_changed);

            let editor = ui.text_edit_singleline(&mut *shared.text.borrow_mut());
            keytips::mark_region(ui.ctx(), Region::Content, editor.rect, Some(editor.id));

            let mut zoom = shared.zoom.get();
            StatusBar::new("status").zoom(&mut zoom).show(ui);
            shared.zoom.set(zoom);
        });
    harness.run();
    harness.run();
    harness
}

/// Alt pressed and released with nothing in between.
fn alt(harness: &mut Harness<'static>) {
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

fn press(harness: &mut Harness<'static>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn shown(harness: &Harness<'static>) -> Vec<KeyTipTarget> {
    keytips::shown(&harness.ctx)
}

fn shown_key(harness: &Harness<'static>, target: &str) -> Option<String> {
    shown(harness)
        .into_iter()
        .find(|shown| shown.target == target)
        .map(|shown| shown.key)
}

fn state(harness: &Harness<'static>) -> KeyTipState {
    keytips::current_state(&harness.ctx)
}

fn clicked(shared: &Shared, key: &str) -> bool {
    shared
        .events
        .borrow()
        .iter()
        .any(|event| event.key == key && event.kind == RibbonEventKind::Clicked)
}

#[test]
fn alt_shows_the_root_badges() {
    let shared = shared();
    let mut harness = harness(shared);
    assert!(shown(&harness).is_empty());
    alt(&mut harness);
    assert_eq!(state(&harness), KeyTipState::Root);
    assert_eq!(shown_key(&harness, "home").as_deref(), Some("H"));
    assert_eq!(shown_key(&harness, "file").as_deref(), Some("F"));
    assert_eq!(shown_key(&harness, "save").as_deref(), Some("1"));
    // Insert has no keytip: its label's first letter is used.
    assert_eq!(shown_key(&harness, "insert").as_deref(), Some("I"));
    assert!(
        shown(&harness)
            .iter()
            .all(|target| target.level == KeyTipLevel::Root)
    );
}

#[test]
fn f10_toggles_keytips() {
    let shared = shared();
    let mut harness = harness(shared);
    press(&mut harness, Key::F10);
    assert_eq!(state(&harness), KeyTipState::Root);
    press(&mut harness, Key::F10);
    assert_eq!(state(&harness), KeyTipState::Idle);
    assert!(shown(&harness).is_empty());
}

#[test]
fn a_tab_letter_shows_that_tabs_control_badges() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::H);
    assert_eq!(state(&harness), KeyTipState::Tab { tab: "home" });
    assert_eq!(shown_key(&harness, "paste").as_deref(), Some("V"));
    assert_eq!(shown_key(&harness, "bold").as_deref(), Some("1"));
    assert_eq!(shown_key(&harness, "clipboard_dialog").as_deref(), Some("FO"));
    assert_eq!(
        shown_key(&harness, "home"),
        None,
        "root badges are hidden at the tab level"
    );
}

#[test]
fn a_tab_keytip_selects_the_tab() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::I);
    assert_eq!(state(&harness), KeyTipState::Tab { tab: "insert" });
    assert_eq!(shared.ribbon.borrow().active, Some("insert"));
    assert!(shared.tab_changed.borrow().contains(&"insert"));
    // The newly selected tab's controls get badges.
    assert_eq!(shown_key(&harness, "blank").as_deref(), Some("B"));
}

#[test]
fn a_control_letter_activates_it_through_ribbon_response() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::H);
    press(&mut harness, Key::V);
    assert!(clicked(shared, "paste"));
    assert_eq!(state(&harness), KeyTipState::Idle);
    assert!(shown(&harness).is_empty());
}

#[test]
fn a_toggle_keytip_flips_its_value() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::H);
    press(&mut harness, Key::Num1);
    assert!(shared.bold.get());
    assert!(
        shared
            .events
            .borrow()
            .iter()
            .any(|event| event.key == "bold" && event.kind == RibbonEventKind::Changed)
    );
}

#[test]
fn a_two_letter_keytip_runs_after_its_second_letter() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::H);
    press(&mut harness, Key::F);
    assert!(matches!(state(&harness), KeyTipState::Prefix { first: 'F', .. }));
    assert_eq!(shown_key(&harness, "clipboard_dialog").as_deref(), Some("FO"));
    assert_eq!(
        shown_key(&harness, "paste"),
        None,
        "only keys with the prefix stay"
    );
    press(&mut harness, Key::O);
    assert!(clicked(shared, "clipboard_dialog"));
}

#[test]
fn a_menu_keytip_opens_its_popup_and_escape_steps_back() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::H);
    press(&mut harness, Key::M);
    assert_eq!(
        state(&harness),
        KeyTipState::Popup {
            tab: "home",
            control: "more"
        }
    );
    assert!(clicked(shared, "more"));
    assert!(harness.query_by_label("More item").is_some(), "the menu is open");
    press(&mut harness, Key::Escape);
    assert_eq!(state(&harness), KeyTipState::Tab { tab: "home" });
    assert!(harness.query_by_label("More item").is_none(), "the menu closed");
}

#[test]
fn escape_steps_back_one_level_at_a_time() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::H);
    press(&mut harness, Key::Escape);
    assert_eq!(state(&harness), KeyTipState::Root);
    assert_eq!(shown_key(&harness, "home").as_deref(), Some("H"));
    press(&mut harness, Key::Escape);
    assert_eq!(state(&harness), KeyTipState::Idle);
    assert!(shown(&harness).is_empty());
}

#[test]
fn alt_as_a_modifier_does_not_enter_keytips() {
    let shared = shared();
    let mut harness = harness(shared);
    harness.key_press_modifiers(Modifiers::ALT, Key::F);
    harness.run();
    assert_eq!(state(&harness), KeyTipState::Idle);
    assert!(!clicked(shared, "file"));
    assert!(shown(&harness).is_empty());
}

#[test]
fn root_keytips_invoke_the_file_tab_and_qat_items() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    press(&mut harness, Key::F);
    assert!(clicked(shared, "file"));
    assert_eq!(state(&harness), KeyTipState::Idle);

    alt(&mut harness);
    press(&mut harness, Key::Num2);
    assert_eq!(*shared.qat_clicked.borrow(), vec!["undo"]);
}

#[test]
fn keytip_letters_do_not_reach_the_focused_editor() {
    let shared = shared();
    let mut harness = harness(shared);
    let editor = harness.get_by_role(Role::TextInput).rect();
    harness.event(Event::PointerMoved(editor.center()));
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: editor.center(),
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    alt(&mut harness);
    harness.event(Event::Text("q".to_owned()));
    press(&mut harness, Key::Q);
    assert_eq!(*shared.text.borrow(), "");
    assert_eq!(
        state(&harness),
        KeyTipState::Root,
        "an unknown letter keeps the level"
    );
}

#[test]
fn a_click_dismisses_keytips() {
    let shared = shared();
    let mut harness = harness(shared);
    alt(&mut harness);
    let pos = Pos2::new(700.0, 450.0);
    harness.event(Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    assert_eq!(state(&harness), KeyTipState::Idle);
}

#[test]
fn disabled_controls_get_no_badges_and_are_not_invoked() {
    let shared = shared();
    shared.disabled.set(true);
    let mut harness = harness(shared);
    alt(&mut harness);
    assert_eq!(shown_key(&harness, "home"), None);
    assert_eq!(shown_key(&harness, "save").as_deref(), Some("1"));
    press(&mut harness, Key::H);
    assert_eq!(state(&harness), KeyTipState::Root);
    assert!(shared.events.borrow().is_empty());
}

#[test]
fn f6_cycles_focus_across_regions() {
    let shared = shared();
    let mut harness = harness(shared);
    let focused = |harness: &Harness<'static>, role: Role, label: &str| {
        harness.get_by_role_and_label(role, label).is_focused()
    };

    press(&mut harness, Key::F6);
    assert!(focused(&harness, Role::Button, "Save"), "QAT first");
    press(&mut harness, Key::F6);
    assert!(focused(&harness, Role::Button, "File"), "then the ribbon");
    press(&mut harness, Key::F6);
    assert!(
        harness.get_by_role(Role::TextInput).is_focused(),
        "then the content"
    );
    press(&mut harness, Key::F6);
    assert!(harness.get_by_label("Zoom").is_focused(), "then the status bar");
    press(&mut harness, Key::F6);
    assert!(focused(&harness, Role::Button, "Save"), "and round again");

    harness.key_press_modifiers(Modifiers::SHIFT, Key::F6);
    harness.run();
    assert!(harness.get_by_label("Zoom").is_focused(), "Shift+F6 goes back");
}

#[test]
fn f6_returns_to_the_last_focused_control_of_a_region() {
    let shared = shared();
    let mut harness = harness(shared);
    press(&mut harness, Key::F6);
    // Move within the QAT, then leave and come back.
    let undo = harness.get_by_role_and_label(Role::Button, "Undo");
    undo.focus();
    harness.run();
    assert!(harness.get_by_role_and_label(Role::Button, "Undo").is_focused());
    for _ in 0..4 {
        press(&mut harness, Key::F6);
    }
    assert!(harness.get_by_role_and_label(Role::Button, "Undo").is_focused());
}

#[test]
fn badges_render_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let shared = shared();
        *shared.theme.borrow_mut() = theme;
        let mut harness = harness(shared);
        alt(&mut harness);
        assert!(!shown(&harness).is_empty());
        press(&mut harness, Key::H);
        assert!(shown_key(&harness, "paste").is_some());
    }
}

/// The access key AccessKit carries on the node with `role` and `label`.
fn access_key(harness: &Harness<'static>, role: Role, label: &str) -> Option<String> {
    use egui_kittest::kittest::NodeT;
    harness
        .get_by_role_and_label(role, label)
        .accesskit_node()
        .data()
        .access_key()
        .map(str::to_owned)
}

#[test]
fn ribbon_targets_carry_their_full_key_sequence_as_access_key() {
    // WI-1849: screen readers read the KeyTip of a control from its AccessKit access key.
    let shared = shared();
    let harness = harness(shared);
    assert_eq!(access_key(&harness, Role::Tab, "Home").as_deref(), Some("Alt, H"));
    assert_eq!(
        access_key(&harness, Role::Button, "Paste").as_deref(),
        Some("Alt, H, V")
    );
    assert_eq!(
        access_key(&harness, Role::Button, "File").as_deref(),
        Some("Alt, F")
    );
    assert_eq!(
        access_key(&harness, Role::Button, "Save").as_deref(),
        Some("Alt, 1")
    );
}

#[test]
fn entering_and_leaving_keytips_is_announced_politely() {
    use egui_kittest::kittest::NodeT;
    let shared = shared();
    let mut harness = harness(shared);
    let live = |harness: &Harness<'static>| {
        harness.query_by_role(Role::Status).map(|node| {
            let node = node.accesskit_node();
            (node.data().live(), node.label().unwrap_or_default())
        })
    };
    assert_eq!(
        live(&harness),
        None,
        "no live region before KeyTips are first used"
    );
    alt(&mut harness);
    harness.run();
    let Some((politeness, text)) = live(&harness) else {
        panic!("no live region while KeyTips are on");
    };
    assert_eq!(politeness, Some(egui::accesskit::Live::Polite));
    assert!(text.starts_with("KeyTips on"), "{text:?}");
    press(&mut harness, Key::Escape);
    harness.run();
    let Some((_, text)) = live(&harness) else {
        panic!("the live region went away");
    };
    assert_eq!(text, "KeyTips off.");
}
