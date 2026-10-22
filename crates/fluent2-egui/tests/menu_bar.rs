//! MenuBar (issue #115): roles, mouse, hover-to-switch, Alt and F10, mnemonics and arrows.

use std::cell::Cell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui::{Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::office::menu_bar::MenuBar;
use fluent2_egui::surfaces::{MenuItem, divider};
use fluent2_egui::theme::{self, Theme};

fn harness_with(theme: Theme, picked: Rc<Cell<Option<&'static str>>>) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(move |ui| {
            theme::install(ui.ctx(), &theme);
            MenuBar::new("menu").show(ui, |bar| {
                bar.menu("&File", |ui| {
                    if ui.add(MenuItem::new("Open").shortcut("Ctrl+O")).clicked() {
                        picked.set(Some("open"));
                    }
                    divider(ui);
                    if ui.add(MenuItem::new("Exit")).clicked() {
                        picked.set(Some("exit"));
                    }
                });
                bar.menu("&Edit", |ui| {
                    if ui.add(MenuItem::new("Undo")).clicked() {
                        picked.set(Some("undo"));
                    }
                });
                bar.menu("&View", |ui| {
                    let _ = ui.add(MenuItem::new("Zoom in"));
                });
                bar.menu("&Help", |ui| {
                    let _ = ui.add(MenuItem::new("About"));
                });
            });
            ui.label("Document");
        })
}

fn harness(picked: Rc<Cell<Option<&'static str>>>) -> Harness<'static> {
    harness_with(Theme::default(), picked)
}

fn is_open(harness: &Harness<'_>, title: &str) -> bool {
    harness
        .get_by_role_and_label(Role::MenuItem, title)
        .accesskit_node()
        .data()
        .is_expanded()
        == Some(true)
}

fn has_item(harness: &Harness<'_>, label: &str) -> bool {
    harness.query_by_role_and_label(Role::MenuItem, label).is_some()
}

#[test]
fn bar_and_titles_have_menu_roles() {
    let mut harness = harness(Rc::default());
    harness.run();
    assert!(
        harness.query_by_role(Role::MenuBar).is_some(),
        "the bar is a menubar"
    );
    for title in ["File", "Edit", "View", "Help"] {
        let node = harness.get_by_role_and_label(Role::MenuItem, title);
        assert_eq!(node.accesskit_node().data().is_expanded(), Some(false));
    }
    assert_eq!(
        harness
            .get_by_role_and_label(Role::MenuItem, "File")
            .accesskit_node()
            .data()
            .access_key(),
        Some("F")
    );
}

#[test]
fn renders_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = harness_with(theme, Rc::default());
        harness.run();
        harness.get_by_role_and_label(Role::MenuItem, "File").click();
        harness.run();
        assert!(has_item(&harness, "Open"));
    }
}

#[test]
fn click_opens_and_an_item_click_reports_and_closes() {
    let picked = Rc::new(Cell::new(None));
    let mut harness = harness(Rc::clone(&picked));
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "File").click();
    harness.run();
    assert!(is_open(&harness, "File"));
    assert!(has_item(&harness, "Open"));

    harness.get_by_role_and_label(Role::MenuItem, "Exit").click();
    harness.run();
    harness.run();
    assert_eq!(picked.get(), Some("exit"));
    assert!(!has_item(&harness, "Exit"), "picking an item closes the menu");
    assert!(!is_open(&harness, "File"));
}

#[test]
fn clicking_the_open_title_closes_it() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "File").click();
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "File").click();
    harness.run();
    assert!(!is_open(&harness, "File"));
    assert!(!has_item(&harness, "Open"));
}

#[test]
fn hovering_another_title_switches_the_open_menu() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "File").click();
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "Edit").hover();
    harness.run();
    harness.run();
    assert!(is_open(&harness, "Edit"));
    assert!(!is_open(&harness, "File"));
    assert!(has_item(&harness, "Undo"));
    assert!(!has_item(&harness, "Open"));
}

#[test]
fn hovering_without_an_open_menu_does_not_open_one() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "Edit").hover();
    harness.run();
    assert!(!is_open(&harness, "Edit"));
}

#[test]
fn alt_f_opens_file() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::F);
    harness.run();
    harness.run();
    assert!(is_open(&harness, "File"), "Alt+F opens File");
    assert!(has_item(&harness, "Open"));
    assert!(
        harness.get_by_role_and_label(Role::MenuItem, "Open").is_focused(),
        "focus moves to the first item"
    );
}

#[test]
fn alt_h_opens_help_and_escape_closes_then_leaves() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::H);
    harness.run();
    harness.run();
    assert!(is_open(&harness, "Help"));

    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    assert!(!is_open(&harness, "Help"));
    assert!(
        harness.get_by_role_and_label(Role::MenuItem, "Help").is_focused(),
        "Esc leaves focus on the title"
    );

    harness.key_press(Key::Escape);
    harness.run();
    harness.run();
    assert!(!harness.get_by_role_and_label(Role::MenuItem, "Help").is_focused());
}

#[test]
fn alt_alone_focuses_the_bar_and_arrows_move_and_open() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.event(egui::Event::ModifiersChanged(Modifiers::ALT));
    harness.run();
    harness.event(egui::Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
    harness.run();
    assert!(harness.get_by_role_and_label(Role::MenuItem, "File").is_focused());
    assert!(
        !is_open(&harness, "File"),
        "Alt focuses the bar without opening a menu"
    );

    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.run();
    assert!(harness.get_by_role_and_label(Role::MenuItem, "Edit").is_focused());

    harness.key_press(Key::ArrowLeft);
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    harness.run();
    assert!(
        harness.get_by_role_and_label(Role::MenuItem, "Help").is_focused(),
        "Left wraps from the first menu to the last"
    );

    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.run();
    assert!(is_open(&harness, "Help"));
    assert!(
        harness
            .get_by_role_and_label(Role::MenuItem, "About")
            .is_focused()
    );
}

#[test]
fn f10_focuses_the_bar_and_a_bare_mnemonic_opens() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.key_press(Key::F10);
    harness.run();
    harness.run();
    assert!(harness.get_by_role_and_label(Role::MenuItem, "File").is_focused());

    harness.key_press(Key::V);
    harness.run();
    harness.run();
    assert!(is_open(&harness, "View"));
}

#[test]
fn arrows_switch_the_open_menu() {
    let mut harness = harness(Rc::default());
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::F);
    harness.run();
    harness.run();
    harness.key_press(Key::ArrowRight);
    harness.run();
    harness.run();
    assert!(is_open(&harness, "Edit"));
    assert!(harness.get_by_role_and_label(Role::MenuItem, "Undo").is_focused());

    harness.key_press(Key::ArrowDown);
    harness.run();
    harness.key_press(Key::ArrowLeft);
    harness.run();
    harness.run();
    assert!(is_open(&harness, "File"));
}

#[test]
fn enter_on_a_focused_item_activates_it() {
    let picked = Rc::new(Cell::new(None));
    let mut harness = harness(Rc::clone(&picked));
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::E);
    harness.run();
    harness.run();
    assert!(harness.get_by_role_and_label(Role::MenuItem, "Undo").is_focused());
    harness.key_press(Key::Enter);
    harness.run();
    harness.run();
    assert_eq!(picked.get(), Some("undo"));
    assert!(!is_open(&harness, "Edit"));
}

#[test]
fn a_disabled_bar_ignores_mnemonics() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(|ui| {
            theme::install(ui.ctx(), &Theme::default());
            ui.add_enabled_ui(false, |ui| {
                MenuBar::new("menu").show(ui, |bar| {
                    bar.menu("&File", |ui| {
                        let _ = ui.add(MenuItem::new("Open"));
                    });
                });
            });
        });
    harness.run();
    harness.key_press_modifiers(Modifiers::ALT, Key::F);
    harness.run();
    harness.run();
    assert!(!is_open(&harness, "File"));
    assert!(
        harness
            .get_by_role_and_label(Role::MenuItem, "File")
            .accesskit_node()
            .is_disabled()
    );
}
