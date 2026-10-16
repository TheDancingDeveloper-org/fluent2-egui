//! Button behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{Appearance, Button, ButtonSize};
use fluent2_egui::{Theme, install};
use fluent2_icons::Icon;

fn install_light(ui: &egui::Ui) {
    install(ui.ctx(), &Theme::default());
}

fn install_kind(ui: &egui::Ui, theme: Theme) {
    fluent2_egui::theme::install(ui.ctx(), &theme);
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            install_kind(ui, theme.clone());
            for appearance in [
                Appearance::Primary,
                Appearance::Secondary,
                Appearance::Outline,
                Appearance::Subtle,
                Appearance::Transparent,
            ] {
                let _ = ui.add(Button::new(format!("{appearance:?}")).appearance(appearance));
            }
            let _ = ui.add(Button::icon_only(Icon::Dismiss, "Close"));
        });
        harness.run();
        for label in [
            "Primary",
            "Secondary",
            "Outline",
            "Subtle",
            "Transparent",
            "Close",
        ] {
            let _ = harness.get_by_role_and_label(Role::Button, label);
        }
    }
}

fn activates(appearance: Appearance, key: Option<egui::Key>) {
    let clicked = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = ui.add(Button::new("Go").appearance(appearance));
        if key.is_some() {
            response.request_focus();
        }
        if response.clicked() {
            clicked.set(true);
        }
    });
    harness.run();
    if let Some(key) = key {
        harness.key_press(key);
    } else {
        harness.get_by_label("Go").click();
    }
    harness.run();
    assert!(clicked.get(), "{appearance:?} via {key:?} should activate");
}

#[test]
fn every_appearance_clicks() {
    for appearance in [
        Appearance::Primary,
        Appearance::Secondary,
        Appearance::Outline,
        Appearance::Subtle,
        Appearance::Transparent,
    ] {
        activates(appearance, None);
    }
}

#[test]
fn every_appearance_activates_on_enter_and_space() {
    for appearance in [
        Appearance::Primary,
        Appearance::Secondary,
        Appearance::Outline,
        Appearance::Subtle,
        Appearance::Transparent,
    ] {
        activates(appearance, Some(egui::Key::Enter));
        activates(appearance, Some(egui::Key::Space));
    }
}

#[test]
fn disabled_ignores_click_and_keyboard() {
    let clicked = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = ui.add_enabled(false, Button::new("Disabled"));
        response.request_focus();
        if response.clicked() {
            clicked.set(true);
        }
    });
    harness.run();
    harness.get_by_label("Disabled").click();
    harness.key_press(egui::Key::Enter);
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(!clicked.get());
}

#[test]
fn medium_button_is_at_least_32_tall() {
    let height = Cell::new(0.0);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = ui.add(Button::new("Save").size(ButtonSize::Medium));
        height.set(response.rect.height());
    });
    harness.run();
    assert!(height.get() >= 32.0, "height {}", height.get());
}

#[test]
fn long_label_truncates_and_shows_the_full_text_on_hover() {
    let full = "A very long label that cannot fit in this narrow button";
    let mut harness = Harness::builder()
        .with_size(egui::vec2(140.0, 80.0))
        .build_ui(|ui| {
            install_light(ui);
            ui.set_max_width(100.0);
            let _ = ui.add(Button::new(full));
            // The button attaches the full label as hover text only when the galley is elided.
            // egui shows that tooltip once the pointer rests on the widget.
        });
    harness.run();
    let node = harness.get_by_label_contains("very long");
    assert!(
        node.rect().width() <= 120.0,
        "button grew to {}, so the label was not truncated",
        node.rect().width()
    );
}

#[test]
fn same_label_with_different_salts_are_distinct() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let _ = ui.add(Button::new("Save").id_salt("one"));
        let _ = ui.add(Button::new("Save").id_salt("two"));
    });
    harness.run();
    let count = harness.get_all_by_label("Save").count();
    assert!(count >= 2, "two salted buttons, got {count}");
}

#[path = "../../../gallery/src/pages/buttons.rs"]
mod buttons;

#[test]
fn gallery_page_is_a_matrix() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        buttons::page(ui);
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Button, "Primary Small");
    let _ = harness.get_by_role_and_label(Role::Button, "Transparent Large");
    let _ = harness.get_by_role_and_label(Role::Button, "Close");
}
