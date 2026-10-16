//! SpinButton: step keys, clamp, revert, and a non-finite range.

use std::cell::Cell;

use egui_kittest::Harness;
use fluent2_egui::controls::SpinButton;
use fluent2_egui::theme::{self, Theme};

fn install(ui: &egui::Ui) {
    theme::install(ui.ctx(), &Theme::default());
}

#[test]
fn arrow_keys_step_and_clamp() {
    let value = Cell::new(1.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(
            SpinButton::new(&mut current)
                .range(0.0..=2.0)
                .step(1.0)
                .decimals(0),
        );
        response.request_focus();
        value.set(current);
    });
    harness.run();
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(value.get(), 2.0);
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(value.get(), 2.0, "stepping past the max clamps");
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(value.get(), 0.0, "stepping past the min clamps");
}

#[test]
fn typing_does_not_report_changed_until_commit() {
    let changed = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = 4.0;
        let response = ui.add(SpinButton::new(&mut current).decimals(0));
        response.request_focus();
        changed.set(response.changed());
    });
    harness.run();
    changed.set(false);
    harness.event(egui::Event::Text("7".to_owned()));
    harness.run();
    assert!(!changed.get(), "a keystroke edits the buffer but does not commit");
}

#[test]
fn invalid_text_reverts_on_enter() {
    let value = Cell::new(4.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(SpinButton::new(&mut current).range(0.0..=10.0).decimals(0));
        response.request_focus();
        value.set(current);
    });
    harness.run();
    // TextEdit only inserts Event::Text, so key presses would never reach the buffer.
    harness.event(egui::Event::Text("abc".to_owned()));
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(value.get(), 4.0, "non-numeric text reverts to the last value");
}

#[test]
fn non_finite_range_does_not_panic() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut value = f64::NAN;
        ui.add(SpinButton::new(&mut value).range(f64::NAN..=10.0));
        ui.add(SpinButton::new(&mut value).range(5.0..=1.0));
    });
    harness.run();
}

#[test]
fn renders_in_dark_and_high_contrast() {
    for theme in [Theme::dark(fluent2_tokens::FLUENT), Theme::high_contrast()] {
        let mut harness = Harness::new_ui(|ui| {
            theme::install(ui.ctx(), &theme);
            let mut value = 3.0;
            ui.add(SpinButton::new(&mut value));
        });
        harness.run();
    }
}
