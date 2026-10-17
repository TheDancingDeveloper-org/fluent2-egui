//! MessageBar behaviour.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::surfaces::{Intent, MessageBar};
use fluent2_egui::{Theme, install};

#[path = "../../../gallery/src/pages/message_bars.rs"]
mod gallery_message_bars;

fn install_light(ui: &egui::Ui) {
    install(ui.ctx(), &Theme::default());
}

#[test]
fn dismiss_and_action_are_reported() {
    let action = Cell::new(None);
    let dismissed = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = MessageBar::new(Intent::Warning, "Disk is almost full")
            .title("Storage")
            .action("Free space")
            .dismissible()
            .show(ui);
        // The response is per frame and `run` may take several; keep what happened.
        if response.action_clicked.is_some() {
            action.set(response.action_clicked);
        }
        if response.dismissed {
            dismissed.set(true);
        }
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Alert, "Storage. Disk is almost full");
    harness.get_by_label("Free space").click();
    harness.run();
    assert_eq!(action.get(), Some(0));
    harness.get_by_label("Dismiss").click();
    harness.run();
    assert!(dismissed.get());
}

#[test]
fn info_is_status_not_alert() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let _ = MessageBar::new(Intent::Info, "Saved").show(ui);
        let _ = MessageBar::new(Intent::Error, "Failed").show(ui);
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Status, "Saved");
    let _ = harness.get_by_role_and_label(Role::Alert, "Failed");
}

#[test]
fn gallery_page_renders_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            fluent2_egui::theme::install(ui.ctx(), &theme);
            gallery_message_bars::page(ui);
        });
        harness.run();
        let _ = harness.get_by_label("Save");
    }
}

#[test]
fn renders_every_intent() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            fluent2_egui::theme::install(ui.ctx(), &theme);
            for intent in [Intent::Info, Intent::Success, Intent::Warning, Intent::Error] {
                let _ = MessageBar::new(intent, format!("{intent:?}")).show(ui);
            }
        });
        harness.run();
    }
}
