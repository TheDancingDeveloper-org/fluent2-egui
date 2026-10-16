//! Checkbox behaviour.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::{CheckState, Checkbox};

fn demo(ui: &mut egui::Ui, state: &mut CheckState) {
    fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
    ui.add(Checkbox::new(state, "Agree"));
}

#[test]
fn click_checks_and_unchecks() {
    let state = Cell::new(CheckState::Unchecked);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = state.get();
        demo(ui, &mut value);
        state.set(value);
    });
    harness.run();
    harness.get_by_role_and_label(Role::CheckBox, "Agree").click();
    harness.run();
    assert_eq!(state.get(), CheckState::Checked);
    harness.get_by_label("Agree").click();
    harness.run();
    assert_eq!(state.get(), CheckState::Unchecked);
}

#[test]
fn space_toggles_the_focused_checkbox() {
    let state = Cell::new(CheckState::Unchecked);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = state.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        let response = ui.add(Checkbox::new(&mut value, "Agree"));
        response.request_focus();
        state.set(value);
    });
    harness.run();
    harness.key_press(egui::Key::Space);
    harness.run();
    assert_eq!(state.get(), CheckState::Checked);
}

#[test]
fn mixed_becomes_checked() {
    let state = Cell::new(CheckState::Mixed);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = state.get();
        demo(ui, &mut value);
        state.set(value);
    });
    harness.run();
    harness.get_by_label("Agree").click();
    harness.run();
    assert_eq!(state.get(), CheckState::Checked);
}

#[test]
fn disabled_does_not_change() {
    let state = Cell::new(CheckState::Unchecked);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = state.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        ui.add_enabled(false, Checkbox::new(&mut value, "Agree"));
        state.set(value);
    });
    harness.run();
    harness.get_by_label("Agree").click();
    harness.run();
    assert_eq!(state.get(), CheckState::Unchecked);
}

#[test]
fn bool_constructor_round_trips() {
    let on = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = on.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        ui.add(Checkbox::bool(&mut value, "On"));
        on.set(value);
    });
    harness.run();
    harness.get_by_label("On").click();
    harness.run();
    assert!(on.get());
}
