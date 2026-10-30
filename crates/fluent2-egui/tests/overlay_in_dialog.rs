//! Overlays opened from inside a modal dialog take input above the dialog.
//!
//! egui blocks every layer ordered below the top modal layer. A popup drawn on its own layer
//! must be registered above the dialog's layer, or its rows can be seen but not picked.

use std::cell::RefCell;
use std::rc::Rc;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{Combobox, Dropdown, MenuButton};
use fluent2_egui::surfaces::{Dialog, DialogKind, MenuItem};
use fluent2_egui::{Theme, install};

/// A harness with an open modal dialog whose body is `body`.
fn modal_harness(mut body: impl FnMut(&mut egui::Ui) + 'static) -> Harness<'static> {
    Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        let mut open = true;
        let _ = Dialog::new("dialog")
            .title("Edit connection")
            .kind(DialogKind::Modal)
            .width(480.0)
            .show(ui.ctx(), &mut open, |ui| body(ui));
    })
}

#[test]
fn dropdown_in_a_modal_dialog_picks_an_option() {
    let selected = Rc::new(RefCell::new(None));
    let shown = Rc::clone(&selected);
    let mut harness = modal_harness(move |ui| {
        let mut value = *shown.borrow();
        let _ = ui.add(
            Dropdown::new("protocol", &mut value, &["RDP", "SSH", "VNC"])
                .placeholder("Protocol")
                .width(160.0),
        );
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.get_by_label("Protocol").click();
    harness.run();
    harness.get_by_role_and_label(Role::ListBoxOption, "SSH").click();
    harness.run();
    assert_eq!(
        *selected.borrow(),
        Some(1),
        "a click on an option inside a modal should select it"
    );
}

#[test]
fn combobox_in_a_modal_dialog_picks_an_option() {
    let text = Rc::new(RefCell::new(String::new()));
    let shown = Rc::clone(&text);
    let mut harness = modal_harness(move |ui| {
        let mut value = shown.borrow().clone();
        let _ = ui.add(
            Combobox::new("host", &mut value, &["alpha", "beta", "gamma"])
                .placeholder("Host")
                .width(160.0),
        );
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.get_by_label("Host").click();
    harness.run();
    harness.get_by_label("Host").type_text("b");
    harness.run();
    harness.get_by_role_and_label(Role::ListBoxOption, "beta").click();
    harness.run();
    assert_eq!(
        text.borrow().as_str(),
        "beta",
        "a click on an option inside a modal should pick it"
    );
}

#[test]
fn menu_button_in_a_modal_dialog_activates_an_item() {
    let clicked = Rc::new(RefCell::new(false));
    let seen = Rc::clone(&clicked);
    let mut harness = modal_harness(move |ui| {
        let _ = MenuButton::new("Actions").id_salt("actions").show(ui, |ui| {
            if ui.add(MenuItem::new("Duplicate")).clicked() {
                *seen.borrow_mut() = true;
            }
        });
    });
    harness.run();
    harness.get_by_label("Actions").click();
    harness.run();
    harness.get_by_role_and_label(Role::MenuItem, "Duplicate").click();
    harness.run();
    assert!(
        *clicked.borrow(),
        "a menu item inside a modal should take the click"
    );
}
