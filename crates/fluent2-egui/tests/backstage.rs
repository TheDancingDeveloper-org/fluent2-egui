//! Backstage behaviour, without the wgpu snapshot feature (spec §7, §9).
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::Button;
use fluent2_egui::office::backstage::{Backstage, BackstageResponse, BackstageState};
use fluent2_egui::{Theme, install};

/// Shared state between the harness closure and the test body.
#[derive(Default)]
struct Probe {
    state: BackstageState,
    response: BackstageResponse,
}

/// A harness with a "File" button (the app's File tab) and a backstage with three pages.
/// The File tab opens the backstage, as the app does in the real ribbon.
fn backstage_harness(probe: Rc<RefCell<Probe>>, theme: fn() -> Theme) -> Harness<'static> {
    Harness::new_ui(move |ui| {
        install(ui.ctx(), &theme());
        let file = ui.add(Button::new("File").id_salt("file"));
        if file.clicked() {
            probe.borrow_mut().state.open = true;
        }
        // While closed, the File tab holds focus, as it would before the user opens the backstage.
        if !probe.borrow().state.open {
            file.request_focus();
        }

        let mut state = probe.borrow().state.clone();
        let response = Backstage::new(&mut state)
            .page("home", "Home", |ui| {
                ui.label("Home body");
            })
            .page("new", "New", |ui| {
                ui.label("New body");
            })
            .bottom_page("options", "Options", |ui| {
                ui.label("Options body");
            })
            .show(ui);
        // `closed` is true only on the frame that closed, so keep it across the harness's frames.
        let mut probe = probe.borrow_mut();
        probe.state = state;
        probe.response.closed |= response.closed;
        if response.page_changed.is_some() {
            probe.response.page_changed = response.page_changed;
        }
    })
}

fn light() -> Theme {
    Theme::default()
}

fn dark() -> Theme {
    Theme::dark(fluent2_tokens::FLUENT)
}

fn high_contrast() -> Theme {
    Theme::high_contrast()
}

fn open_harness() -> (Rc<RefCell<Probe>>, Harness<'static>) {
    let probe = Rc::new(RefCell::new(Probe::default()));
    probe.borrow_mut().state.open = true;
    let harness = backstage_harness(probe.clone(), light);
    (probe, harness)
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    for theme in [light as fn() -> Theme, dark, high_contrast] {
        let probe = Rc::new(RefCell::new(Probe::default()));
        probe.borrow_mut().state.open = true;
        let mut harness = backstage_harness(probe, theme);
        harness.run();
        let _ = harness.get_by_role_and_label(Role::Dialog, "Backstage");
        let _ = harness.get_by_role_and_label(Role::Button, "Home");
    }
}

#[test]
fn closed_backstage_draws_nothing() {
    let probe = Rc::new(RefCell::new(Probe::default()));
    let mut harness = backstage_harness(probe.clone(), light);
    harness.run();
    assert!(
        harness.query_by_label("Back").is_none(),
        "no backstage while closed"
    );
    assert_eq!(probe.borrow().response, BackstageResponse::default());
}

#[test]
fn file_tab_event_opens_backstage() {
    let probe = Rc::new(RefCell::new(Probe::default()));
    let mut harness = backstage_harness(probe.clone(), light);
    harness.run();
    harness.get_by_label("File").click();
    harness.run();
    assert!(probe.borrow().state.open, "the File tab click set open");
    let _ = harness.get_by_role_and_label(Role::Dialog, "Backstage");
}

#[test]
fn clicking_a_page_selects_it_and_reports_the_change() {
    let (probe, mut harness) = open_harness();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "New").click();
    harness.run();
    assert_eq!(probe.borrow().response.page_changed, Some("new"));
    assert_eq!(probe.borrow().state.page, Some("new"));
}

#[test]
fn bottom_group_page_is_selectable() {
    let (probe, mut harness) = open_harness();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Options").click();
    harness.run();
    assert_eq!(probe.borrow().state.page, Some("options"));
}

#[test]
fn arrow_keys_move_the_selection_through_the_entries() {
    let (probe, mut harness) = open_harness();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Home").focus();
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(
        probe.borrow().state.page,
        Some("new"),
        "ArrowDown moves to the next page"
    );
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(
        probe.borrow().state.page,
        Some("options"),
        "ArrowDown reaches the bottom group"
    );
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(probe.borrow().state.page, Some("new"), "ArrowUp moves back");
}

#[test]
fn escape_closes_and_returns_focus_to_the_file_tab() {
    let (probe, mut harness) = open_harness();
    harness.run();
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Home").focus();
    harness.run();
    assert!(
        !harness.get_by_label("File").is_focused(),
        "focus moved into the backstage"
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(probe.borrow().response.closed, "Escape reports the close");
    assert!(!probe.borrow().state.open, "Escape cleared open");
    assert!(
        harness.get_by_label("File").is_focused(),
        "focus returned to the File tab"
    );
}

#[test]
fn back_arrow_closes_and_returns_focus_to_the_file_tab() {
    let (probe, mut harness) = open_harness();
    harness.run();
    harness.get_by_label("File").click();
    harness.run();
    harness.get_by_label("Back").click();
    harness.run();
    assert!(probe.borrow().response.closed, "the back arrow reports the close");
    assert!(!probe.borrow().state.open);
    assert!(harness.get_by_label("File").is_focused());
}

#[test]
fn nav_column_has_the_spec_roles_and_labels() {
    let (_probe, mut harness) = open_harness();
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Dialog, "Backstage");
    let _ = harness.get_by_role_and_label(Role::Button, "Back");
    let _ = harness.get_by_role_and_label(Role::Button, "Home");
    let _ = harness.get_by_role_and_label(Role::Button, "New");
    let _ = harness.get_by_role_and_label(Role::Button, "Options");
}
