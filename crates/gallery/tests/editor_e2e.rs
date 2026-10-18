//! End-to-end tests of the sample editor (WI-1570): the whole chrome in one app, driven
//! through kittest without the wgpu snapshot feature.

#[allow(dead_code)]
#[path = "../examples/editor.rs"]
mod editor;

use std::cell::RefCell;
use std::rc::Rc;

use editor::Editor;
use egui::accesskit::Role;
use egui::{Event, Key, Modifiers, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::{Theme, install};

/// The editor in a harness of `width` × 700, with `theme` installed.
fn harness(width: f32, theme: Theme) -> (Harness<'static>, Rc<RefCell<Editor>>) {
    let editor = Rc::new(RefCell::new(Editor::default()));
    let shared = Rc::clone(&editor);
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width, 700.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &theme);
            shared.borrow_mut().ui(ui);
        });
    harness.run();
    harness.run();
    (harness, editor)
}

fn alt(harness: &mut Harness<'static>) {
    harness.event(Event::ModifiersChanged(Modifiers::ALT));
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
}

fn press(harness: &mut Harness<'static>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn has(harness: &Harness<'_>, role: Role, label: &str) -> bool {
    harness.query_by_role_and_label(role, label).is_some()
}

fn contextual_tab_shown(harness: &Harness<'_>) -> bool {
    harness.query_by_label("Picture format").is_some()
}

#[test]
fn alt_h_bold_keytip_toggles_bold() {
    let (mut harness, editor) = harness(1400.0, Theme::default());
    assert!(!editor.borrow().bold);
    alt(&mut harness);
    press(&mut harness, Key::H);
    press(&mut harness, Key::Num1);
    assert!(editor.borrow().bold);
    assert!(has(&harness, Role::Label, "Bold"), "status bar shows bold");

    alt(&mut harness);
    press(&mut harness, Key::H);
    press(&mut harness, Key::Num1);
    assert!(!editor.borrow().bold);
}

#[test]
fn file_opens_backstage_and_escape_returns_to_the_document() {
    let (mut harness, editor) = harness(1400.0, Theme::default());
    harness.get_by_label("File").click();
    harness.run();
    harness.run();
    assert!(editor.borrow().backstage.open);
    assert!(
        has(&harness, Role::Button, "Options"),
        "backstage navigation is shown"
    );
    assert!(harness.query_by_role(Role::MultilineTextInput).is_none());

    press(&mut harness, Key::Escape);
    harness.run();
    assert!(!editor.borrow().backstage.open);
    assert!(harness.query_by_role(Role::MultilineTextInput).is_some());
}

#[test]
fn selecting_the_picture_shows_the_contextual_tab() {
    let (mut harness, editor) = harness(1400.0, Theme::default());
    assert!(!contextual_tab_shown(&harness));

    harness.get_by_label("Placeholder picture").click();
    harness.run();
    harness.run();
    assert!(editor.borrow().picture_selected);
    assert!(contextual_tab_shown(&harness));

    harness.get_by_label("Placeholder picture").click();
    harness.run();
    harness.run();
    assert!(!editor.borrow().picture_selected);
    assert!(!contextual_tab_shown(&harness));
}

/// Ribbon buttons drawn tall, at the Large step.
fn large_buttons(harness: &Harness<'_>) -> usize {
    harness
        .query_all_by_role(Role::Button)
        .filter(|node| node.rect().height() > 40.0)
        .count()
}

#[test]
fn the_ribbon_reduces_as_the_window_narrows() {
    let counts: Vec<usize> = [1400.0, 800.0, 500.0]
        .into_iter()
        .map(|width| {
            let (harness, _) = harness(width, Theme::default());
            assert!(has(&harness, Role::TabList, "Ribbon tabs"), "at {width}");
            large_buttons(&harness)
        })
        .collect();
    assert!(counts[0] >= counts[1], "{counts:?}");
    assert!(counts[1] >= counts[2], "{counts:?}");
    assert!(counts[2] < counts[0], "{counts:?}");
}

#[test]
fn every_theme_renders() {
    let fluent = fluent2_egui::tokens::FLUENT;
    for theme in [Theme::light(fluent), Theme::dark(fluent), Theme::high_contrast()] {
        for width in [1400.0, 500.0] {
            let (mut harness, _) = harness(width, theme.clone());
            harness.get_by_label("Placeholder picture").click();
            harness.run();
            harness.get_by_label("File").click();
            harness.run();
            harness.run();
        }
    }
}

#[test]
fn the_accessibility_tree_names_the_chrome() {
    let (harness, _) = harness(1400.0, Theme::default());
    assert!(has(&harness, Role::TabList, "Ribbon tabs"));
    assert!(harness.query_all_by_role(Role::Toolbar).next().is_some());
    for label in ["Save", "Undo", "Redo", "Paste", "Bold"] {
        assert!(harness.query_by_label(label).is_some(), "{label}");
    }
    assert!(has(&harness, Role::Button, "Paste"));
    assert!(has(&harness, Role::Group, "Status bar"));
    let words = format!("{} words", Editor::default().word_count());
    assert!(has(&harness, Role::Label, &words), "{words}");
    assert!(has(&harness, Role::Label, "Saved"));
}
