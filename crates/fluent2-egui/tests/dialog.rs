//! Dialog behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
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
                actions.primary("save", "Save");
                actions.secondary("cancel", "Cancel");
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
    assert_eq!(
        probe.borrow().closed_with,
        Some(CloseReason::Button("save".to_owned()))
    );
}

#[test]
fn same_label_actions_are_distinct_and_survive_a_language_switch() {
    let label = Rc::new(RefCell::new("Save".to_owned()));
    let probe = open_probe();
    let label_for_ui = label.clone();
    let probe_for_ui = probe.clone();
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let text = label_for_ui.borrow().clone();
        let mut open = probe_for_ui.borrow().open;
        let was_open = open;
        let result = Dialog::new("duplicate_labels")
            .title("Save?")
            .actions(|actions| {
                actions.primary("save", text.clone());
                actions.secondary("discard", text);
            })
            .show(ui.ctx(), &mut open, |_ui| {});
        record(&probe_for_ui, was_open, open, result);
    });
    harness.run();
    let english: Vec<_> = harness
        .get_all_by_label("Save")
        .map(|node| node.accesskit_node().id())
        .collect();
    assert_eq!(english.len(), 2, "both actions show the same label");
    assert_ne!(
        english[0], english[1],
        "widget ids come from the action id, not the label"
    );

    *label.borrow_mut() = "Speichern".to_owned();
    harness.run();
    let translated: Vec<_> = harness
        .get_all_by_label("Speichern")
        .map(|node| node.accesskit_node().id())
        .collect();
    assert_eq!(translated.len(), 2, "both buttons show the translated label");
    assert_eq!(translated, english, "a language switch keeps the widget ids");
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        probe.borrow().closed_with,
        Some(CloseReason::Button("save".to_owned())),
        "the close result is the action id, not the translated label"
    );
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
    assert_eq!(
        probe.borrow().closed_with,
        Some(CloseReason::Button("cancel".to_owned()))
    );
}

/// Which focus stop of the dialog has focus: one of Save, Cancel, Inside, or the surface.
/// Panics if focus is on the page behind the dialog, which the trap must prevent.
fn focus_stop(harness: &Harness<'_>) -> &'static str {
    assert!(
        !harness.get_by_label("Open").is_focused(),
        "focus escaped the dialog to the page"
    );
    if harness.get_by_label("Save").is_focused() {
        "Save"
    } else if harness.get_by_label("Cancel").is_focused() {
        "Cancel"
    } else if harness.get_by_label("Inside").is_focused() {
        "Inside"
    } else {
        assert!(
            harness
                .get_by_role_and_label(Role::Dialog, "Dialog title")
                .is_focused(),
            "focus is on no stop inside the dialog"
        );
        "surface"
    }
}

#[test]
fn tab_and_shift_tab_wrap_inside_dialog() {
    let probe = open_probe();
    let mut harness = dialog_harness(probe.clone(), DialogKind::Modal);
    harness.run();
    harness.run();
    // Opening focuses the primary action.
    assert_eq!(focus_stop(&harness), "Save");

    // Forward: surface, Inside, Cancel, Save, then wrap back to the surface.
    let mut forward = Vec::new();
    for _ in 0..4 {
        harness.key_press(egui::Key::Tab);
        harness.run();
        forward.push(focus_stop(&harness));
    }
    assert_eq!(
        forward,
        ["surface", "Inside", "Cancel", "Save"],
        "Tab order, wrapping last to first"
    );

    // Backward from Save: Cancel, Inside, surface, then wrap to Save.
    let mut backward = Vec::new();
    for _ in 0..4 {
        harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
        harness.run();
        backward.push(focus_stop(&harness));
    }
    assert_eq!(
        backward,
        ["Cancel", "Inside", "surface", "Save"],
        "Shift+Tab order, wrapping first to last"
    );

    assert!(
        probe.borrow().open,
        "Tab and Shift+Tab stay inside and do not close the dialog"
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
                .actions(|actions| actions.secondary("cancel", "Cancel"))
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
                    actions.primary("save", "Save");
                    actions.secondary("cancel", "Cancel");
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

/// A 600 × 400 screen with a dialog whose body holds `rows` buttons, "Row 0" to "Row n-1".
fn tall_harness(probe: Rc<RefCell<Probe>>, rows: usize) -> Harness<'static> {
    Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(move |ui| {
            install_light(ui);
            let _ = ui.add(Button::new("Open").id_salt("opener"));
            let mut open = probe.borrow().open;
            let was_open = open;
            let result = Dialog::new("tall")
                .title("Tall dialog")
                .width(400.0)
                .actions(|actions| {
                    actions.primary("save", "Save");
                    actions.secondary("cancel", "Cancel");
                })
                .show(ui.ctx(), &mut open, |ui| {
                    for row in 0..rows {
                        let _ = ui.add(Button::new(format!("Row {row}")).id_salt(("row", row)));
                    }
                });
            record(&probe, was_open, open, result);
        })
}

fn assert_inside(outer: egui::Rect, inner: egui::Rect, what: &str) {
    assert!(
        outer.expand(0.5).contains_rect(inner),
        "{what} {inner:?} is outside {outer:?}"
    );
}

#[test]
fn tall_body_scrolls_inside_a_surface_that_fits_the_screen() {
    let probe = open_probe();
    let mut harness = tall_harness(probe.clone(), 40);
    for _ in 0..4 {
        harness.run();
    }
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(600.0, 400.0));
    let surface = harness.get_by_role_and_label(Role::Dialog, "Tall dialog").rect();
    assert_inside(screen, surface, "the surface");
    // The title and both actions stay visible inside the surface.
    assert_inside(surface, harness.get_by_label("Tall dialog").rect(), "the title");
    assert_inside(surface, harness.get_by_label("Save").rect(), "Save");
    assert_inside(surface, harness.get_by_label("Cancel").rect(), "Cancel");
    assert_inside(surface, harness.get_by_label("Row 0").rect(), "the first row");

    // The measured title and action heights settle: the surface does not creep frame to frame.
    for _ in 0..5 {
        harness.run();
    }
    let later = harness.get_by_role_and_label(Role::Dialog, "Tall dialog").rect();
    assert_eq!(surface, later, "the surface settles");
}

#[test]
fn focusing_a_lower_control_scrolls_it_into_view_and_clicks_reach_it() {
    let probe = open_probe();
    let clicked = Rc::new(RefCell::new(false));
    let seen = clicked.clone();
    let rows = 40;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(600.0, 400.0))
        .build_ui(move |ui| {
            install_light(ui);
            let mut open = probe.borrow().open;
            let was_open = open;
            let result = Dialog::new("tall")
                .title("Tall dialog")
                .width(400.0)
                .actions(|actions| actions.primary("save", "Save"))
                .show(ui.ctx(), &mut open, |ui| {
                    for row in 0..rows {
                        let button = ui.add(Button::new(format!("Row {row}")).id_salt(("row", row)));
                        if row == rows - 1 && button.clicked() {
                            *seen.borrow_mut() = true;
                        }
                    }
                });
            record(&probe, was_open, open, result);
        });
    for _ in 0..4 {
        harness.run();
    }
    // Opening focuses Save; Shift+Tab moves back to the last body row.
    harness.key_press_modifiers(egui::Modifiers::SHIFT, egui::Key::Tab);
    for _ in 0..6 {
        harness.run();
    }
    assert!(
        harness.get_by_label("Row 39").is_focused(),
        "Shift+Tab reached the last row"
    );
    let surface = harness.get_by_role_and_label(Role::Dialog, "Tall dialog").rect();
    let last = harness.get_by_label("Row 39").rect();
    assert_inside(surface, last, "the focused last row");

    harness.get_by_label("Row 39").click();
    harness.run();
    harness.run();
    assert!(*clicked.borrow(), "the click reached the scrolled-in row");
    assert!(
        probe_open(&harness),
        "the click did not land on the backdrop and close the dialog"
    );
}

fn probe_open(harness: &Harness<'_>) -> bool {
    harness
        .query_by_role_and_label(Role::Dialog, "Tall dialog")
        .is_some()
}

#[test]
fn short_body_does_not_grow_the_surface() {
    let probe = open_probe();
    let mut harness = tall_harness(probe, 2);
    for _ in 0..4 {
        harness.run();
    }
    let surface = harness.get_by_role_and_label(Role::Dialog, "Tall dialog").rect();
    assert!(
        surface.height() < 250.0,
        "a two-row dialog stays short, got {}",
        surface.height()
    );
}
