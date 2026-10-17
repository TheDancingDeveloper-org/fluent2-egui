//! Dialog behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{Button, Dropdown};
use fluent2_egui::surfaces::{CloseReason, Dialog, DialogKind, DialogResult};
use fluent2_egui::{Theme, install};

/// Shared state between the harness closure and the test body.
#[derive(Default)]
struct Probe {
    open: bool,
    closed_with: Option<CloseReason>,
}

/// Store the dialog's open flag. The close reason is kept only on the frame that closed it:
/// while closed, `show` keeps returning `Closed(Programmatic)` and that must not overwrite it.
fn record<R>(probe: &RefCell<Probe>, was_open: bool, open: bool, result: DialogResult<R>) {
    let mut state = probe.borrow_mut();
    state.open = open;
    if let (true, DialogResult::Closed(reason)) = (was_open, result) {
        state.closed_with = Some(reason);
    }
}

fn install_light(ui: &egui::Ui) {
    install(ui.ctx(), &Theme::default());
}

/// A harness with an outside "Open" button and one dialog with Save (primary) and Cancel.
/// The dialog's `open` flag and its last close reason live in `probe`.
fn dialog_harness(probe: Rc<RefCell<Probe>>, kind: DialogKind) -> Harness<'static> {
    Harness::new_ui(move |ui| {
        install_light(ui);
        let _ = ui.add(Button::new("Open").id_salt("opener"));
        let mut open = probe.borrow().open;
        let was_open = open;
        let result = Dialog::new("dialog")
            .title("Dialog title")
            .kind(kind)
            .width(600.0)
            .actions(|actions| {
                actions.primary("Save");
                actions.secondary("Cancel");
            })
            .show(ui.ctx(), &mut open, |ui| {
                ui.label("Body");
                let _ = ui.add(Button::new("Inside").id_salt("inside"));
            });
        record(&probe, was_open, open, result);
    })
}

fn open_probe() -> Rc<RefCell<Probe>> {
    Rc::new(RefCell::new(Probe {
        open: true,
        closed_with: None,
    }))
}

#[test]
fn reports_role_and_title() {
    let probe = open_probe();
    let mut harness = dialog_harness(probe.clone(), DialogKind::Modal);
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Dialog, "Dialog title");
}

#[test]
fn alert_reports_alert_dialog_role() {
    let probe = open_probe();
    let mut harness = dialog_harness(probe.clone(), DialogKind::Alert);
    harness.run();
    let _ = harness.get_by_role_and_label(Role::AlertDialog, "Dialog title");
}

#[test]
fn escape_closes_modal_but_not_alert() {
    let modal = open_probe();
    let mut harness = dialog_harness(modal.clone(), DialogKind::Modal);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!modal.borrow().open, "Escape should close a modal");
    assert_eq!(modal.borrow().closed_with, Some(CloseReason::Escape));

    let alert = open_probe();
    let mut harness = dialog_harness(alert.clone(), DialogKind::Alert);
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(alert.borrow().open, "Escape should not close an alert");
}

#[test]
fn enter_triggers_primary() {
    let probe = open_probe();
    let mut harness = dialog_harness(probe.clone(), DialogKind::Modal);
    harness.run();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(!probe.borrow().open, "Enter should trigger the primary action");
    assert_eq!(probe.borrow().closed_with, Some(CloseReason::Button("Save")));
}

#[test]
fn enter_on_another_button_does_not_trigger_primary() {
    let probe = open_probe();
    let mut harness = dialog_harness(probe.clone(), DialogKind::Modal);
    harness.run();
    // Focus starts on Save. Step Tab until Cancel has focus; Cancel takes Enter itself.
    for _ in 0..6 {
        if harness.get_by_label("Cancel").is_focused() {
            break;
        }
        harness.key_press(egui::Key::Tab);
        harness.run();
    }
    assert!(
        harness.get_by_label("Cancel").is_focused(),
        "Cancel should be reachable by Tab"
    );
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(!probe.borrow().open, "Cancel should close the dialog");
    assert_eq!(probe.borrow().closed_with, Some(CloseReason::Button("Cancel")));
}

#[test]
fn tab_cycles_only_inside_dialog() {
    let probe = open_probe();
    let mut harness = dialog_harness(probe.clone(), DialogKind::Modal);
    harness.run();
    harness.run();
    for _ in 0..12 {
        harness.key_press(egui::Key::Tab);
        harness.run();
        assert!(
            !harness.get_by_label("Open").is_focused(),
            "focus must not leave the dialog for the page behind it"
        );
    }
    assert!(
        probe.borrow().open,
        "Tab stays inside and does not close the dialog"
    );
}

#[test]
fn focus_returns_to_opener() {
    let probe = open_probe();
    probe.borrow_mut().open = false;
    let mut harness = {
        let probe = probe.clone();
        Harness::new_ui(move |ui| {
            install_light(ui);
            let opener = ui.add(Button::new("Open").id_salt("opener"));
            let mut open = probe.borrow().open;
            let was_open = open;
            if !open {
                // While closed, the opener holds focus, as it would before the user opens it.
                opener.request_focus();
            }
            let result = Dialog::new("dialog")
                .title("Dialog title")
                .actions(|actions| actions.secondary("Cancel"))
                .show(ui.ctx(), &mut open, |ui| {
                    ui.label("Body");
                });
            record(&probe, was_open, open, result);
        })
    };
    harness.run();
    assert!(
        harness.get_by_label("Open").is_focused(),
        "the opener starts focused"
    );

    probe.borrow_mut().open = true;
    harness.run();
    assert!(
        !harness.get_by_label("Open").is_focused(),
        "focus moved into the dialog"
    );

    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!probe.borrow().open, "Escape closed the dialog");
    harness.run();
    assert!(
        harness.get_by_label("Open").is_focused(),
        "focus returns to the opener when the dialog closes"
    );
}

#[test]
fn dropdown_inside_dialog_escapes_first() {
    let probe = open_probe();
    let selected = Rc::new(RefCell::new(None::<usize>));
    let mut harness = {
        let probe = probe.clone();
        let selected = selected.clone();
        Harness::new_ui(move |ui| {
            install_light(ui);
            let mut open = probe.borrow().open;
            let was_open = open;
            let result = Dialog::new("dialog")
                .title("Dialog title")
                .actions(|actions| {
                    actions.primary("Save");
                    actions.secondary("Cancel");
                })
                .show(ui.ctx(), &mut open, |ui| {
                    let mut choice = *selected.borrow();
                    let _ = ui.add(Dropdown::new("choice", &mut choice, &["Perth", "Paris"]));
                    *selected.borrow_mut() = choice;
                });
            record(&probe, was_open, open, result);
        })
    };
    harness.run();
    // Open the Dropdown, then Escape. The popup consumes the first Escape.
    harness.get_by_label("Dropdown").click();
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        probe.borrow().open,
        "the first Escape should close the popup, not the dialog"
    );
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!probe.borrow().open, "the second Escape should close the dialog");
}

#[test]
fn closed_when_not_open_is_programmatic() {
    let ctx = egui::Context::default();
    let mut open = false;
    let result = Dialog::new("dialog").show::<()>(&ctx, &mut open, |ui| {
        ui.label("Body");
    });
    assert_eq!(result, DialogResult::Closed(CloseReason::Programmatic));
}
