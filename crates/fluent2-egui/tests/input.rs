//! Input, SearchBox and Textarea behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by label.

use std::cell::RefCell;
use std::rc::Rc;

use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{
    ButtonSize, Input, InputAppearance, SearchBox, Textarea, TextareaResize, accent_open_for_test,
};
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
            let mut name = String::new();
            let mut notes = String::new();
            let mut query = String::new();
            ui.add(Input::new(&mut name).label("Name"));
            ui.add(SearchBox::new(&mut query).label("Find"));
            ui.add(Textarea::new(&mut notes).label("Notes").rows(2));
        });
        harness.run();
        let _ = harness.get_by_label("Name");
        let _ = harness.get_by_label("Find");
        let _ = harness.get_by_label("Notes");
    }
}

#[test]
fn typing_changes_the_string() {
    let value = Rc::new(RefCell::new(String::new()));
    let shown = Rc::clone(&value);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut text = shown.borrow().clone();
        let response = ui.add(Input::new(&mut text).label("Name"));
        response.request_focus();
        *shown.borrow_mut() = text;
    });
    harness.run();
    harness.get_by_label("Name").focus();
    harness.run();
    harness.get_by_label("Name").type_text("a");
    harness.run();
    assert_eq!(
        value.borrow().as_str(),
        "a",
        "a focused input should take the typed character"
    );
}

#[test]
fn disabled_refuses_input() {
    let value = Rc::new(RefCell::new(String::new()));
    let shown = Rc::clone(&value);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut text = shown.borrow().clone();
        ui.add_enabled(false, Input::new(&mut text).label("Name"));
        *shown.borrow_mut() = text;
    });
    harness.run();
    harness.get_by_label("Name").focus();
    harness.run();
    harness.get_by_label("Name").type_text("a");
    harness.run();
    assert!(value.borrow().is_empty(), "a disabled input should ignore typing");
}

#[test]
fn search_box_escape_clears() {
    let value = Rc::new(RefCell::new(String::from("rust")));
    let shown = Rc::clone(&value);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut text = shown.borrow().clone();
        let response = ui.add(SearchBox::new(&mut text).label("Find"));
        response.request_focus();
        *shown.borrow_mut() = text;
    });
    harness.run();
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(
        value.borrow().is_empty(),
        "Escape in a focused SearchBox should clear it"
    );
}

#[test]
fn focused_input_opens_the_accent_bar() {
    let rect = Rc::new(RefCell::new(egui::Rect::NOTHING));
    let shown = Rc::clone(&rect);
    let mut harness = Harness::new_ui(move |ui| {
        install_egui_theme(ui);
        let mut text = String::new();
        let response = ui.add(Input::new(&mut text).label("Name").desired_width(120.0));
        response.request_focus();
        *shown.borrow_mut() = response.rect;
    });
    harness.run();
    harness.run();
    let open = accent_open_for_test(&harness.ctx, *rect.borrow());
    assert_eq!(
        open,
        Some(true),
        "a focused input should aim its accent bar at fully open"
    );
}

#[test]
fn appearances_and_sizes_are_distinct() {
    assert_ne!(InputAppearance::Outline, InputAppearance::Underline);
    assert_ne!(InputAppearance::FilledDarker, InputAppearance::FilledLighter);
    assert_ne!(ButtonSize::Small, ButtonSize::Large);
    assert_ne!(TextareaResize::Vertical, TextareaResize::None);
}
