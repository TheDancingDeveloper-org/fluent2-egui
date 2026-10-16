//! Tooltip and popover behaviour, without the wgpu snapshot feature.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::surfaces::{Popover, TooltipExt};
use fluent2_egui::{Theme, install};

fn install_light(ui: &egui::Ui) {
    install(ui.ctx(), &Theme::default());
}

#[test]
fn tooltip_appears_after_hover_time() {
    let mut harness = Harness::builder().with_step_dt(0.2).build_ui(|ui| {
        install_light(ui);
        ui.button("Save").fluent_tooltip("Saves the document");
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Save").hover();
    // egui's default tooltip delay is 0.5s. Two 0.2s steps are not enough.
    harness.step();
    harness.step();
    let too_soon = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness.get_by_label("Saves the document");
    }));
    assert!(too_soon.is_err(), "tooltip should wait for the hover delay");
    harness.step();
    harness.step();
    let _ = harness.get_by_label("Saves the document");
}

#[test]
fn tooltip_appears_on_keyboard_focus() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        ui.button("Save").fluent_tooltip("Saves the document");
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Save").focus();
    // A key event marks keyboard focus. Tab also moves focus onto the button.
    harness.key_press(egui::Key::Tab);
    harness.run();
    let _ = harness.get_by_label("Saves the document");
}

#[test]
fn popover_closes_on_escape_and_outside_click_and_returns_focus() {
    let open = std::cell::Cell::new(false);
    let anchor_focused = std::cell::Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = ui.button("Details");
        if ui.button("Toggle").clicked() {
            open.set(!open.get());
        }
        let mut is_open = open.get();
        Popover::new("details")
            .label("Details popover")
            .show(&response, &mut is_open, |ui| {
                ui.label("Body of the popover");
            });
        open.set(is_open);
        anchor_focused.set(response.has_focus());
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Toggle").click();
    harness.run();
    harness.run();
    let dialog = harness.get_by_role_and_label(Role::Dialog, "Details popover");
    assert!(
        dialog.is_focused(),
        "focus should stay on the popover after it opens"
    );
    let _ = harness.get_by_label("Body of the popover");

    harness.key_press(egui::Key::Escape);
    harness.run();
    let after_escape = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness.get_by_label("Body of the popover");
    }));
    assert!(after_escape.is_err(), "Escape should close the popover");
    harness.run();
    assert!(anchor_focused.get(), "focus should return to the anchor");
}

#[test]
fn popover_closes_on_click_outside() {
    let open = std::cell::Cell::new(true);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(400.0, 300.0))
        .build_ui(|ui| {
            install_light(ui);
            let response = ui.button("Details");
            let mut is_open = open.get();
            Popover::new("details")
                .label("Details popover")
                .show(&response, &mut is_open, |ui| {
                    ui.label("Body of the popover");
                });
            open.set(is_open);
        });
    harness.run();
    let _ = harness.get_by_label("Body of the popover");
    harness.event(egui::Event::PointerMoved(egui::pos2(390.0, 290.0)));
    harness.event(egui::Event::PointerButton {
        pos: egui::pos2(390.0, 290.0),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.event(egui::Event::PointerButton {
        pos: egui::pos2(390.0, 290.0),
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    harness.run();
    let still = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness.get_by_label("Body of the popover");
    }));
    assert!(still.is_err(), "a click outside should close the popover");
}
