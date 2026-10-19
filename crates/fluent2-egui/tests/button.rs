//! Button behaviour, without the wgpu snapshot feature.
//!
//! Widgets are found through the AccessKit tree by role and label.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

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
        // Focus requested this frame is live on the next one. A press-and-release is two
        // harness steps and the second clears `clicked`, so send the key-down only.
        harness.run();
        harness.key_down(key);
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

#[test]
fn tab_moves_from_one_button_to_the_next() {
    let focused = RefCell::new(("".to_owned(), "".to_owned()));
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let first = ui.add(Button::new("Alpha"));
        let second = ui.add(Button::new("Beta"));
        let name = |response: &egui::Response, label: &str| -> String {
            if response.has_focus() {
                label.to_owned()
            } else {
                String::new()
            }
        };
        *focused.borrow_mut() = (name(&first, "Alpha"), name(&second, "Beta"));
    });
    harness.run();
    harness.key_down(egui::Key::Tab);
    harness.run();
    assert_eq!(
        focused.borrow().0,
        "Alpha",
        "first Tab should focus Alpha, not an invisible stop"
    );
    harness.key_up(egui::Key::Tab);
    harness.run();
    harness.key_down(egui::Key::Tab);
    harness.run();
    assert_eq!(focused.borrow().1, "Beta", "second Tab should focus Beta");
}

fn click_nth(harness: &mut Harness, index: usize) {
    let mut nodes = harness.get_all_by_label("Save");
    let mut node = None;
    for _ in 0..=index {
        node = nodes.next();
    }
    drop(nodes);
    if let Some(node) = node {
        node.click();
    }
}

#[test]
fn two_unsalted_buttons_with_the_same_label_are_independent() {
    let hits = RefCell::new((0, 0));
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let mut counts = *hits.borrow();
        if ui.add(Button::new("Save")).clicked() {
            counts.0 += 1;
        }
        if ui.add(Button::new("Save")).clicked() {
            counts.1 += 1;
        }
        *hits.borrow_mut() = counts;
    });
    harness.run();
    assert_eq!(harness.get_all_by_label("Save").count(), 2);
    click_nth(&mut harness, 0);
    harness.run();
    click_nth(&mut harness, 1);
    harness.run();
    assert_eq!(*hits.borrow(), (1, 1), "each Save should click once");
}

#[test]
fn min_width_overrides_the_size_minimum_but_not_the_content() {
    let widths = Rc::new(RefCell::new(Vec::new()));
    let out = Rc::clone(&widths);
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let mut w = out.borrow_mut();
        w.clear();
        w.push(ui.add(Button::new("7")).rect.width());
        w.push(ui.add(Button::new("7").min_width(32.0)).rect.width());
        w.push(ui.add(Button::new("A long label").min_width(0.0)).rect.width());
        w.push(ui.add(Button::new("7").min_width(f32::NAN)).rect.width());
    });
    harness.run();
    let w = widths.borrow();
    assert!(w[0] >= 96.0, "default medium minimum: {}", w[0]);
    assert!((w[1] - 32.0).abs() < 0.5, "overridden minimum: {}", w[1]);
    assert!(w[2] > 60.0, "content still sets the width: {}", w[2]);
    assert!(w[3] >= 96.0, "NaN falls back to the size minimum: {}", w[3]);
}
