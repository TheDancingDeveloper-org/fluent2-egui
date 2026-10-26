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

fn finite_slider(
    start: f64,
    range: std::ops::RangeInclusive<f64>,
    step: Option<f64>,
    press: Option<egui::Key>,
) -> f64 {
    let value = Cell::new(start);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(Slider::new(&mut current, range.clone()).step(step));
        if press.is_some() {
            response.request_focus();
        }
        value.set(current);
    });
    harness.run();
    if let Some(key) = press {
        harness.key_press(key);
        harness.run();
    }
    value.get()
}

#[test]
fn inverted_range_does_not_panic() {
    let value = finite_slider(f64::NAN, 80.0..=10.0, None, None);
    assert!(value.is_finite());
    assert!((10.0..=80.0).contains(&value));
}

#[test]
fn nan_bound_does_not_panic() {
    let value = finite_slider(f64::NAN, 8.0..=f64::NAN, None, None);
    assert!(value.is_finite());
}

#[test]
fn extreme_finite_range_stays_finite() {
    let value = finite_slider(-f64::MAX, -f64::MAX..=f64::MAX, None, None);
    assert!(value.is_finite(), "value {value}");
    assert_eq!(value, -f64::MAX);
}

#[test]
fn degenerate_range_stays_finite() {
    let value = finite_slider(4.0, 7.0..=7.0, None, None);
    assert_eq!(value, 7.0);
}

#[test]
fn tiny_step_stays_finite() {
    let value = finite_slider(
        0.0,
        0.0..=1.0,
        Some(f64::MIN_POSITIVE),
        Some(egui::Key::ArrowRight),
    );
    assert!(value.is_finite(), "value {value}");
    assert!((0.0..=1.0).contains(&value));
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
