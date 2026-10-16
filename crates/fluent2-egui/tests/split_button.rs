//! SplitButton and MenuButton behaviour, without the wgpu snapshot feature.

use std::cell::{Cell, RefCell};

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{MenuButton, SplitButton};
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
            fluent2_egui::theme::install(ui.ctx(), &theme.clone());
            let _ = SplitButton::new("Save").show(ui, |_| {});
            let _ = MenuButton::new("Open").show(ui, |_| {});
        });
        harness.run();
        let _ = harness.get_by_role_and_label(Role::Button, "Save");
        let _ = harness.get_by_role_and_label(Role::Button, "Save options");
        let _ = harness.get_by_role_and_label(Role::Button, "Open");
    }
}

#[test]
fn clicking_main_does_not_open_the_menu() {
    let main_clicked = Cell::new(false);
    let open = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = SplitButton::new("Save").show(ui, |ui| {
            ui.label("Save as");
        });
        if response.main.clicked() {
            main_clicked.set(true);
        }
        open.set(response.menu_open);
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Save").click();
    harness.run();
    assert!(main_clicked.get(), "the main region should click");
    assert!(!open.get(), "clicking main must not open the menu");
}

#[test]
fn clicking_the_chevron_opens_the_menu_and_does_not_click_main() {
    let main_clicked = Cell::new(false);
    let open = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = SplitButton::new("Save").show(ui, |ui| {
            ui.label("Save as");
        });
        if response.main.clicked() {
            main_clicked.set(true);
        }
        open.set(response.menu_open);
    });
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Save options")
        .click();
    harness.run();
    assert!(
        open.get(),
        "the chevron should open the menu (open={})",
        open.get()
    );
    assert!(!main_clicked.get(), "the chevron must not click the main region");
    let _ = harness.get_by_label("Save as");
}

#[test]
fn tab_moves_from_main_to_chevron() {
    let focused = RefCell::new(("".to_owned(), "".to_owned()));
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = SplitButton::new("Save").show(ui, |_| {});
        let name = |response: &egui::Response, label: &str| -> String {
            if response.has_focus() {
                label.to_owned()
            } else {
                String::new()
            }
        };
        *focused.borrow_mut() = (name(&response.main, "main"), name(&response.menu, "chevron"));
    });
    harness.run();
    harness.key_down(egui::Key::Tab);
    harness.run();
    assert_eq!(focused.borrow().0, "main", "first Tab focuses the main region");
    harness.key_up(egui::Key::Tab);
    harness.run();
    harness.key_down(egui::Key::Tab);
    harness.run();
    assert_eq!(focused.borrow().1, "chevron", "second Tab focuses the chevron");
}

#[test]
fn escape_closes_the_menu_and_focuses_the_chevron() {
    let open = Cell::new(false);
    let chevron_focus = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = SplitButton::new("Save").show(ui, |ui| {
            ui.label("Save as");
        });
        open.set(response.menu_open);
        chevron_focus.set(response.menu.has_focus());
    });
    harness.run();
    harness
        .get_by_role_and_label(Role::Button, "Save options")
        .click();
    harness.run();
    assert!(open.get());
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!open.get(), "Escape closes the menu");
    assert!(chevron_focus.get(), "focus returns to the chevron");
}

#[test]
fn menu_button_opens_on_click() {
    let open = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let (response, value) = MenuButton::new("Open").show(ui, |ui| {
            ui.label("Recent");
            7
        });
        let _ = response;
        open.set(value == Some(7));
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Open").click();
    harness.run();
    assert!(open.get(), "clicking a menu button opens its popup");
}

#[test]
fn disabled_split_button_ignores_both_regions() {
    let main_clicked = Cell::new(false);
    let open = Cell::new(false);
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let response = ui
            .add_enabled_ui(false, |ui| SplitButton::new("Save").show(ui, |_| {}))
            .inner;
        if response.main.clicked() {
            main_clicked.set(true);
        }
        open.set(response.menu_open);
    });
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Save").click();
    harness
        .get_by_role_and_label(Role::Button, "Save options")
        .click();
    harness.run();
    assert!(!main_clicked.get());
    assert!(!open.get());
}

#[path = "../../../gallery/src/pages/split_buttons.rs"]
mod split_buttons;

#[test]
fn gallery_page_renders() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        split_buttons::page(ui);
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Button, "Split Primary");
    let _ = harness.get_by_role_and_label(Role::Button, "Split Primary options");
    let _ = harness.get_by_role_and_label(Role::Button, "Menu Transparent");
    let _ = harness.get_by_role_and_label(Role::Button, "Disabled");
}
