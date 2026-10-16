//! Dropdown and Combobox behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by label.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{ButtonSize, Combobox, Dropdown};
use fluent2_egui::theme::{self, Theme};
use fluent2_tokens::FLUENT;

fn install_for(ui: &egui::Ui, theme: Theme) {
    theme::install(ui.ctx(), &theme);
}

fn install_egui_theme(ui: &egui::Ui) {
    let theme = match ui.ctx().theme() {
        egui::Theme::Light => Theme::light(FLUENT),
        egui::Theme::Dark => Theme::dark(FLUENT),
    };
    install_for(ui, theme);
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    for theme in [Theme::light(FLUENT), Theme::dark(FLUENT), Theme::high_contrast()] {
        let mut harness = Harness::new_ui(move |ui| {
            install_for(ui, theme.clone());
            let mut selected = Some(0usize);
            let mut text = String::new();
            ui.add(Dropdown::new("city", &mut selected, &["Perth", "Paris"]).placeholder("City"));
            ui.add(Combobox::new("town", &mut text, &["Lyon", "Nice"]).placeholder("Town"));
        });
        harness.run();
        let _ = harness.get_by_label("City");
    }
}

#[test]
fn pointer_selects_an_option() {
    let selected = Rc::new(RefCell::new(None));
    let shown = Rc::clone(&selected);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut value = *shown.borrow();
        ui.add(
            Dropdown::new("city", &mut value, &["Perth", "Paris"])
                .placeholder("City")
                .width(160.0),
        );
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.get_by_label("City").click();
    harness.run();
    harness
        .get_by_role_and_label(egui::accesskit::Role::Button, "Paris")
        .click();
    harness.run();
    assert_eq!(*selected.borrow(), Some(1), "clicking an option should select it");
}

#[test]
fn keyboard_opens_moves_and_selects() {
    let selected = Rc::new(RefCell::new(None));
    let shown = Rc::clone(&selected);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut value = *shown.borrow();
        let _response = ui.add(
            Dropdown::new("city", &mut value, &["Perth", "Paris", "Pisa"])
                .placeholder("City")
                .width(160.0),
        );
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.get_by_label("City").click();
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        *selected.borrow(),
        Some(0),
        "Enter, Down, Enter should select the first option"
    );
}

#[test]
fn type_to_jump() {
    let selected = Rc::new(RefCell::new(None));
    let shown = Rc::clone(&selected);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut value = *shown.borrow();
        let response = ui.add(
            Dropdown::new("city", &mut value, &["Perth", "Paris", "Rome"])
                .placeholder("City")
                .width(160.0),
        );
        response.request_focus();
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.run();
    harness.key_press(egui::Key::R);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(*selected.borrow(), Some(2), "typing R should jump to Rome");
}

#[test]
fn combobox_filters() {
    let text = Rc::new(RefCell::new(String::new()));
    let shown = Rc::clone(&text);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut value = shown.borrow().clone();
        let response = ui.add(
            Combobox::new("city", &mut value, &["Perth", "Paris", "Rome"])
                .placeholder("City")
                .width(160.0),
        );
        response.request_focus();
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.get_by_label("City").type_text("pa");
    harness.run();
    let _ = harness.get_by_label("Paris");
    assert!(
        harness.query_by_label("Rome").is_none(),
        "a prefix filter should hide options that do not match"
    );
}

#[test]
fn many_options_virtualise() {
    let options: Vec<String> = (0..5_000).map(|index| format!("Option {index}")).collect();
    let options2 = options.clone();
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut selected = None;
        ui.add(
            Dropdown::new("many", &mut selected, &options2)
                .placeholder("Pick")
                .width(200.0),
        );
    });
    harness.run();
    harness.get_by_label("Pick").click();
    harness.run();
    // A virtualised popup lays out one viewport. Options far past it are not in the tree.
    assert!(harness.query_by_label("Option 0").is_some() || harness.query_by_label("Option 1").is_some());
    assert!(
        harness.query_by_label("Option 4999").is_none(),
        "the last of 5000 options must not be laid out in one frame"
    );
    let _ = options;
    let _ = ButtonSize::Medium;
}

#[test]
fn dropdown_value_is_the_selected_text() {
    let mut harness = Harness::new_ui(|ui| {
        install_egui_theme(ui);
        let mut selected = Some(1usize);
        ui.add(Dropdown::new("city", &mut selected, &["Perth", "Paris"]).placeholder("City"));
    });
    harness.run();
    let node = harness.get_by_role_and_label(egui::accesskit::Role::ComboBox, "City");
    assert_eq!(node.value().as_deref(), Some("Paris"));
}

#[test]
fn combobox_value_is_the_typed_text() {
    let mut harness = Harness::new_ui(|ui| {
        install_egui_theme(ui);
        let mut text = String::from("Perth");
        ui.add(Combobox::new("city", &mut text, &["Perth", "Paris"]).placeholder("City"));
    });
    harness.run();
    let node = harness.get_by_role_and_label(egui::accesskit::Role::ComboBox, "City");
    assert_eq!(node.value().as_deref(), Some("Perth"));
}

#[test]
fn freeform_enter_keeps_typed_text() {
    let text = Rc::new(RefCell::new(String::new()));
    let shown = Rc::clone(&text);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut value = shown.borrow().clone();
        let response = ui.add(
            Combobox::new("city", &mut value, &["Perth", "Paris"])
                .placeholder("City")
                .freeform(true)
                .width(160.0),
        );
        response.request_focus();
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.run();
    harness.get_by_label("City").type_text("Oslo");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        text.borrow().as_str(),
        "Oslo",
        "freeform Enter should keep the typed text"
    );
}

#[test]
fn non_freeform_enter_picks_the_active_option() {
    let text = Rc::new(RefCell::new(String::new()));
    let shown = Rc::clone(&text);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut value = shown.borrow().clone();
        let response = ui.add(
            Combobox::new("city", &mut value, &["Perth", "Paris"])
                .placeholder("City")
                .width(160.0),
        );
        response.request_focus();
        *shown.borrow_mut() = value;
    });
    harness.run();
    harness.run();
    harness.get_by_label("City").type_text("pa");
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(
        text.borrow().as_str(),
        "Paris",
        "Enter without freeform should replace the text with the active option"
    );
}

#[test]
fn combobox_marks_the_matching_option() {
    let mut harness = Harness::new_ui(|ui| {
        install_egui_theme(ui);
        let mut text = String::from("Paris");
        let response = ui.add(
            Combobox::new("city", &mut text, &["Perth", "Paris"])
                .placeholder("City")
                .width(160.0),
        );
        response.request_focus();
    });
    harness.run();
    harness.key_press_modifiers(egui::Modifiers::ALT, egui::Key::ArrowDown);
    harness.run();
    let node = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Paris");
    let toggled = egui_kittest::kittest::NodeT::accesskit_node(&node).toggled();
    assert_eq!(toggled, Some(egui::accesskit::Toggled::True));
}

#[test]
fn five_thousand_options_lay_out_under_one_hundred() {
    let options: Vec<String> = (0..5_000).map(|index| format!("Option {index}")).collect();
    let options2 = options.clone();
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut selected = None;
        ui.add(
            Dropdown::new("many", &mut selected, &options2)
                .placeholder("Pick")
                .width(200.0),
        );
    });
    harness.run();
    harness.get_by_label("Pick").click();
    harness.run();
    let laid_out = harness.query_all_by_label_contains("Option ").count();
    assert!(
        laid_out < 100,
        "5000 options should lay out fewer than 100 rows, laid out {laid_out}"
    );
}

#[test]
fn press_outside_closes_the_listbox() {
    let mut harness = Harness::new_ui(|ui| {
        install_egui_theme(ui);
        let mut selected = None;
        ui.add(
            Dropdown::new("city", &mut selected, &["Perth", "Paris"])
                .placeholder("City")
                .width(160.0),
        );
        // A second widget so a press can land outside both the field and the popup.
        ui.add_space(80.0);
        let _ = ui.button("Elsewhere");
    });
    harness.run();
    harness.get_by_label("City").click();
    harness.run();
    let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, "Paris");
    harness.get_by_label("Elsewhere").click();
    harness.run();
    assert!(
        harness
            .query_by_role_and_label(egui::accesskit::Role::Button, "Paris")
            .is_none(),
        "a press outside the field and the popup should close the listbox"
    );
}
