//! StatusBar: the zoom slider changes its value, and the percentage label resets it to 100.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::office::status_bar::StatusBar;
use fluent2_egui::theme::{self, Theme};

#[test]
fn dragging_the_zoom_slider_changes_the_value() {
    let zoom = Cell::new(100.0_f32);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 40.0))
        .build_ui(|ui| {
            theme::install(ui.ctx(), &Theme::default());
            let mut current = zoom.get();
            StatusBar::new("status").zoom(&mut current).show(ui);
            zoom.set(current);
        });
    harness.run();
    let rect = harness.get_by_role_and_label(Role::Slider, "Zoom").rect();
    let center = rect.center();
    harness.event(egui::Event::PointerMoved(center));
    harness.event(egui::Event::PointerButton {
        pos: center,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    harness.event(egui::Event::PointerButton {
        pos: center,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    assert!(
        zoom.get() > 150.0 && zoom.get() < 260.0,
        "pressing mid-rail moves the zoom to about 205%, got {}",
        zoom.get()
    );
}

#[test]
fn the_percentage_label_resets_zoom_to_100() {
    let zoom = Cell::new(250.0_f32);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 40.0))
        .build_ui(|ui| {
            theme::install(ui.ctx(), &Theme::default());
            let mut current = zoom.get();
            StatusBar::new("status").zoom(&mut current).show(ui);
            zoom.set(current);
        });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "250%").click();
    harness.run();
    assert_eq!(zoom.get(), 100.0);
}
