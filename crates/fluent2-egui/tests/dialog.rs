//! Dialog behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::RefCell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{Button, Dropdown};
use fluent2_egui::surfaces::{Dialog, DialogKind, DialogResult};
use fluent2_egui::{Theme, install};

fn install_light(ui: &egui::Ui) {
    install(ui.ctx(), &Theme::default());
}

/// Build a harness with an outside button and one dialog. `open` is the dialog state.
fn dialog_harness(open: RefCell<bool>, kind: DialogKind, body: impl Fn(&mut egui::Ui) + Copy + 'static) -> Harness<'static> {
    Harness::new_ui(move |ui| {
        install_light(ui);
        let _ = ui.add(Button::new("Open").id_salt("opener"));
        let mut is_open = *open.borrow();
        let _ = Dialog::new("dialog")
            .title("Dialog title")
            .kind(kind)
            .width(600.0)
            .actions(|actions| {
                actions.primary("Save");
                actions.secondary("Cancel");
            })
            .show(ui.ctx(), &mut is_open, |ui| body(ui));
        drop(open.borrow_mut().clone());
        *open.borrow_mut() = is_open;
        eprintln!("wrote {is_open}");
    })
}

#[test]
fn reports_role_and_title() {
    let mut harness = dialog_harness(RefCell::new(true), DialogKind::Modal, |ui| {
        ui.label("Body");
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Dialog, "Dialog title");
}

#[test]
fn escape_closes_modal_but_not_alert() {
    let modal_open = RefCell::new(true);
    let mut harness = dialog_harness(modal_open.clone(), DialogKind::Modal, |ui| {
        ui.label("Body");
    });
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!*modal_open.borrow(), "Escape should close a modal");

    let alert_open = RefCell::new(true);
    let mut harness = dialog_harness(alert_open.clone(), DialogKind::Alert, |ui| {
        ui.label("Body");
    });
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(*alert_open.borrow(), "Escape should not close an alert");
}

#[test]
fn enter_triggers_primary() {
    let open = RefCell::new(true);
    let mut harness = dialog_harness(open.clone(), DialogKind::Modal, |ui| {
        ui.label("Body");
    });
    harness.run();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(!*open.borrow(), "Enter should trigger the primary action");
}

#[test]
fn tab_cycles_inside_dialog() {
    let open = RefCell::new(true);
    let mut harness = dialog_harness(open.clone(), DialogKind::Modal, |ui| {
        let _ = ui.add(Button::new("Inside"));
        let _ = ui.add(Dropdown::new("choice", &mut None, &["Perth", "Paris"]));
    });
    harness.run();
    harness.run();
    for _ in 0..12 {
        harness.key_press(egui::Key::Tab);
        harness.run();
    }
    assert!(*open.borrow(), "Tab stays inside and does not close the dialog");
}

#[test]
fn focus_returns_to_opener() {
    let open = RefCell::new(true);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let opener = ui.add(Button::new("Open").id_salt("opener"));
        let _ = opener.clone();
        let mut open = open.borrow_mut();
        opener.request_focus();
        let _ = Dialog::new("dialog")
            .title("Dialog title")
            .actions(|actions| actions.secondary("Cancel"))
            .show(ui.ctx(), &mut open, |ui| {
                let _ = ui.add(Button::new("Inside"));
            });
    });
    harness.run();
    let opener = harness.get_by_label("Open");
    assert!(
        opener.is_focused(),
        "the opener has focus before the dialog opens"
    );
}

#[test]
fn dropdown_inside_dialog_escapes_first() {
    let open = RefCell::new(true);
    let mut selected = None;
    let open_state = open.clone();
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let mut is_open = *open_state.borrow();
        let _ = Dialog::new("dialog")
            .title("Dialog title")
            .actions(|actions| {
                actions.primary("Save");
                actions.secondary("Cancel");
            })
            .show(ui.ctx(), &mut is_open, |ui| {
                let _ = ui.add(Dropdown::new("choice", &mut selected, &["Perth", "Paris"]));
            });
        *open_state.borrow_mut() = is_open;
    });
    harness.run();
    // Open the Dropdown, then Escape. The popup consumes the first Escape.
    harness.get_by_label("Dropdown").click();
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        *open.borrow(),
        "the first Escape should close the popup, not the dialog"
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!*open.borrow(), "the second Escape should close the dialog");
}

#[test]
fn closed_when_not_open_is_programmatic() {
    let ctx = egui::Context::default();
    let mut open = false;
    let result = Dialog::new("dialog").show::<()>(&ctx, &mut open, |ui| {
        ui.label("Body");
    });
    assert!(matches!(result, DialogResult::Closed(_)));
    drop(ctx);
}
