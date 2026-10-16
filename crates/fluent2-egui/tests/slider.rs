//! Slider: Home/End, drag, and step snapping.

use std::cell::Cell;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::Slider;
use fluent2_egui::theme::{self, Theme};

fn install(ui: &egui::Ui) {
    theme::install(ui.ctx(), &Theme::default());
}

#[test]
fn home_and_end_jump_to_the_range() {
    let value = Cell::new(40.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(Slider::new(&mut current, 0.0..=100.0));
        response.request_focus();
        value.set(current);
    });
    harness.run();
    harness.key_press(egui::Key::Home);
    harness.run();
    assert_eq!(value.get(), 0.0);
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(value.get(), 100.0);
}

#[test]
fn drag_changes_the_value() {
    let value = Cell::new(0.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        ui.add(Slider::new(&mut current, 0.0..=100.0));
        value.set(current);
    });
    harness.run();
    let node = harness.get_by_label("Slider");
    let rect = node.rect();
    harness.event(egui::Event::PointerMoved(rect.center()));
    harness.event(egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    assert!(
        value.get() > 10.0 && value.get() < 90.0,
        "drag to the middle moves the value, got {}",
        value.get()
    );
}

#[test]
fn step_snaps() {
    let value = Cell::new(0.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(Slider::new(&mut current, 0.0..=100.0).step(Some(10.0)));
        response.request_focus();
        value.set(current);
    });
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert_eq!(value.get(), 10.0);
}

#[test]
fn inverted_range_does_not_panic() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut value = f64::NAN;
        ui.add(Slider::new(&mut value, 8.0..=f64::NAN).vertical());
    });
    harness.run();
}

#[test]
fn renders_in_dark_and_high_contrast() {
    for theme in [Theme::dark(fluent2_tokens::FLUENT), Theme::high_contrast()] {
        let mut harness = Harness::new_ui(|ui| {
            theme::install(ui.ctx(), &theme);
            let mut value = 20.0;
            ui.add(Slider::new(&mut value, 0.0..=100.0));
        });
        harness.run();
    }
}
