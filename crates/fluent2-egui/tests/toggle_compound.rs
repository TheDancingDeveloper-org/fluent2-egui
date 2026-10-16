//! ToggleButton and CompoundButton behaviour.

use std::cell::Cell;

use egui::accesskit::{Role, Toggled};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{Appearance, Button, CompoundButton, ToggleButton};
use fluent2_egui::{Theme, install};

fn install_light(ui: &egui::Ui) {
    install(ui.ctx(), &Theme::default());
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            fluent2_egui::theme::install(ui.ctx(), &theme);
            let mut on = true;
            let _ = ui.add(ToggleButton::new(&mut on, "Bold"));
            let _ = ui.add(CompoundButton::new("Meeting").secondary_content("Soon"));
        });
        harness.run();
        let _ = harness.get_by_role_and_label(Role::Button, "Bold");
        let _ = harness.get_by_role_and_label(Role::Button, "Meeting");
    }
}

#[test]
fn click_and_space_flip_checked() {
    for key in [None, Some(egui::Key::Space)] {
        let checked = Cell::new(false);
        let mut harness = Harness::new_ui(|ui| {
            install_light(ui);
            let mut value = checked.get();
            let response = ui.add(ToggleButton::new(&mut value, "Bold"));
            if key.is_some() {
                response.request_focus();
            }
            checked.set(value);
        });
        harness.run();
        if let Some(key) = key {
            harness.run();
            harness.key_down(key);
        } else {
            harness.get_by_label("Bold").click();
        }
        harness.run();
        assert!(checked.get(), "flip via {key:?}");
    }
}

#[test]
fn accesskit_toggled_follows_checked() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let mut on = true;
        let _ = ui.add(ToggleButton::new(&mut on, "Bold"));
        let mut off = false;
        let _ = ui.add(ToggleButton::new(&mut off, "Italic"));
    });
    harness.run();
    let bold = harness.get_by_role_and_label(Role::Button, "Bold");
    let italic = harness.get_by_role_and_label(Role::Button, "Italic");
    assert_eq!(bold.accesskit_node().toggled(), Some(Toggled::True));
    assert_eq!(italic.accesskit_node().toggled(), Some(Toggled::False));
}

#[test]
fn compound_is_taller_than_a_single_line_button() {
    let heights = Cell::new((0.0, 0.0));
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let button = ui.add(Button::new("Meeting"));
        let compound = ui.add(CompoundButton::new("Meeting").secondary_content("Starts in 5 minutes"));
        heights.set((button.rect.height(), compound.rect.height()));
    });
    harness.run();
    let (single, compound) = heights.get();
    assert!(
        compound > single,
        "compound {compound} should exceed single {single}"
    );
}

#[test]
fn disabled_toggle_does_not_flip() {
    let checked = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let mut value = checked.get();
        let _ = ui.add_enabled(false, ToggleButton::new(&mut value, "Bold"));
        checked.set(value);
    });
    harness.run();
    harness.get_by_label("Bold").click();
    harness.run();
    assert!(!checked.get());
}

#[test]
fn every_appearance_paints() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        for appearance in [
            Appearance::Primary,
            Appearance::Secondary,
            Appearance::Outline,
            Appearance::Subtle,
            Appearance::Transparent,
        ] {
            let mut on = true;
            let _ = ui.add(ToggleButton::new(&mut on, format!("T{appearance:?}")).appearance(appearance));
            let _ = ui.add(
                CompoundButton::new(format!("C{appearance:?}"))
                    .secondary_content("line")
                    .appearance(appearance),
            );
        }
    });
    harness.run();
    let _ = harness.get_by_label("TPrimary");
    let _ = harness.get_by_label("CTransparent");
}
