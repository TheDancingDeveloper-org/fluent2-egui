//! Menu behaviour, without the wgpu snapshot feature.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::surfaces::MenuItem;
use fluent2_egui::{Theme, install};

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
            ui.menu_button("File", |ui| {
                let _ = ui.add(MenuItem::new("Save"));
                let _ = ui.add(MenuItem::new("Close"));
            });
        });
        harness.run();
        harness.get_by_role_and_label(Role::Button, "File").click();
        harness.run();
        let _ = harness.get_by_role_and_label(Role::Button, "Save");
        let _ = harness.get_by_role_and_label(Role::Button, "Close");
    }
}

#[test]
fn clicking_an_item_returns_clicked_and_closes() {
    let clicked = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        ui.menu_button("File", |ui| {
            if ui.add(MenuItem::new("Save")).clicked() {
                clicked.set(true);
            }
        });
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "File").click();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Save").click();
    harness.run();
    assert!(clicked.get(), "Save should report clicked");
    harness.run();
    let open = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness.get_by_role_and_label(Role::Button, "Save");
    }));
    assert!(open.is_err(), "menu should close after the click");
}

#[test]
fn escape_closes_the_menu() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        ui.menu_button("File", |ui| {
            let _ = ui.add(MenuItem::new("Save"));
        });
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "File").click();
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Button, "Save");
    harness.key_press(egui::Key::Escape);
    harness.run();
    let open = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        harness.get_by_role_and_label(Role::Button, "Save");
    }));
    assert!(open.is_err(), "Escape should close the menu");
}

#[test]
fn tab_moves_focus_between_items() {
    let focused = std::cell::RefCell::new(String::new());
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let first = ui.add(MenuItem::new("Alpha"));
        let second = ui.add(MenuItem::new("Beta"));
        if first.has_focus() {
            *focused.borrow_mut() = "Alpha".to_owned();
        } else if second.has_focus() {
            *focused.borrow_mut() = "Beta".to_owned();
        }
    });
    harness.run();
    harness.key_down(egui::Key::Tab);
    harness.run();
    assert_eq!(focused.borrow().as_str(), "Alpha", "first Tab focuses Alpha");
    harness.key_up(egui::Key::Tab);
    harness.run();
    harness.key_down(egui::Key::Tab);
    harness.run();
    assert_eq!(focused.borrow().as_str(), "Beta", "second Tab focuses Beta");
}

#[path = "../../../gallery/src/pages/menus.rs"]
mod menus;

#[test]
fn gallery_page_renders() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        menus::page(ui);
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Button, "File");
}
