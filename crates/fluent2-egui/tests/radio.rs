//! Radio group behaviour.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::RadioGroup;

#[test]
fn click_selects_an_option() {
    let selected = Cell::new(0usize);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = selected.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        ui.add(RadioGroup::new(&mut value).option("A").option("B"));
        selected.set(value);
    });
    harness.run();
    harness.get_by_role_and_label(Role::RadioButton, "B").click();
    harness.run();
    assert_eq!(selected.get(), 1);
}

#[test]
fn arrows_move_within_the_group() {
    let selected = Cell::new(0usize);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = selected.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        let response = ui.add(RadioGroup::new(&mut value).option("A").option("B").option("C"));
        response.request_focus();
        selected.set(value);
    });
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(selected.get(), 1);
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(selected.get(), 2);
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(selected.get(), 0);
}

#[test]
fn space_selects_the_focused_option() {
    let selected = Cell::new(0usize);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = selected.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        let response = ui.add(RadioGroup::new(&mut value).option("A").option("B"));
        response.request_focus();
        selected.set(value);
    });
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(selected.get(), 1);
    harness.key_press(egui::Key::Space);
    harness.run();
    assert_eq!(selected.get(), 1);
}

#[test]
fn disabled_does_not_change() {
    let selected = Cell::new(0usize);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = selected.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        ui.add_enabled(false, RadioGroup::new(&mut value).option("A").option("B"));
        selected.set(value);
    });
    harness.run();
    harness.get_by_label("B").click();
    harness.run();
    assert_eq!(selected.get(), 0);
}
