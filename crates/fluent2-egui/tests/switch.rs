//! Switch behaviour.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::Switch;

#[test]
fn click_toggles() {
    let on = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = on.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        ui.add(Switch::new(&mut value, "Wi-Fi"));
        on.set(value);
    });
    harness.run();
    harness.get_by_role_and_label(Role::Switch, "Wi-Fi").click();
    harness.run();
    assert!(on.get());
}

#[test]
fn space_toggles_the_focused_switch() {
    let on = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = on.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        let response = ui.add(Switch::new(&mut value, "Wi-Fi"));
        response.request_focus();
        on.set(value);
    });
    harness.run();
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(on.get());
}

#[test]
fn disabled_does_not_change() {
    let on = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        let mut value = on.get();
        fluent2_egui::theme::install(ui.ctx(), &fluent2_egui::Theme::default());
        ui.add_enabled(false, Switch::new(&mut value, "Wi-Fi"));
        on.set(value);
    });
    harness.run();
    harness.get_by_label("Wi-Fi").click();
    harness.run();
    assert!(!on.get());
}
