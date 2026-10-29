//! TabList and Card behaviour, without the wgpu snapshot feature.

use std::cell::Cell;

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::surfaces::{Card, CardAppearance, CardHeader, TabList, TabSize};
use fluent2_egui::{Theme, install};
use fluent2_icons::Icon;

fn install_light(ui: &egui::Ui) {
    install(ui.ctx(), &Theme::default());
}

fn tab_harness(selected: &'static Cell<usize>, vertical: bool) -> Harness<'static> {
    // Build a harness that keeps the selection in a cell so the test can read it.
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let mut current = selected.get();
        let mut list = TabList::new(("tabs", vertical), &mut current)
            .size(TabSize::Medium)
            .tab("One")
            .tab("Two")
            .tab_disabled("Three")
            .tab("Four");
        if vertical {
            list = list.vertical();
        }
        let response = list.show(ui);
        response.request_focus();
        selected.set(current);
    });
    harness.run();
    harness.run();
    harness
}

fn selection_cell() -> &'static Cell<usize> {
    // Each test owns its own cell; a leaked box gives the closure a 'static handle.
    Box::leak(Box::new(Cell::new(0)))
}

#[test]
fn arrow_keys_move_the_selection() {
    let selected = selection_cell();
    let mut harness = tab_harness(selected, false);
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert_eq!(selected.get(), 1);
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    // "Three" is disabled, so the next enabled tab is "Four".
    assert_eq!(selected.get(), 3);
    harness.key_press(egui::Key::ArrowLeft);
    harness.run();
    assert_eq!(selected.get(), 1);
}

#[test]
fn vertical_uses_up_and_down() {
    let selected = selection_cell();
    let mut harness = tab_harness(selected, true);
    harness.key_press(egui::Key::ArrowDown);
    harness.run();
    assert_eq!(selected.get(), 1);
    harness.key_press(egui::Key::ArrowUp);
    harness.run();
    assert_eq!(selected.get(), 0);
}

#[test]
fn home_and_end_jump() {
    let selected = selection_cell();
    let mut harness = tab_harness(selected, false);
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(selected.get(), 3);
    harness.key_press(egui::Key::Home);
    harness.run();
    assert_eq!(selected.get(), 0);
}

#[test]
fn disabled_tabs_are_skipped() {
    let selected = selection_cell();
    let mut harness = tab_harness(selected, false);
    // End lands on "Four" (last enabled), never "Three".
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(selected.get(), 3);
    assert_ne!(selected.get(), 2);
}

#[test]
fn one_tab_stop_for_the_whole_list() {
    let focused = Box::leak(Box::new(Cell::new(false))) as &Cell<bool>;
    let after_focused = Box::leak(Box::new(Cell::new(false))) as &Cell<bool>;
    let requested = Box::leak(Box::new(Cell::new(false))) as &Cell<bool>;
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let mut selected = 0;
        let response = TabList::new("tabs", &mut selected).tab("One").tab("Two").show(ui);
        // Focus the list once. A focusable control after it lets Tab show where focus goes:
        // from the list straight to that control, not to a second tab stop.
        if !requested.get() {
            response.request_focus();
            requested.set(true);
        }
        focused.set(response.has_focus());
        let after = ui.button("After");
        after_focused.set(after.has_focus());
    });
    harness.run();
    harness.run();
    assert!(focused.get(), "the list takes focus");
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(!focused.get(), "Tab leaves the list");
    assert!(
        after_focused.get(),
        "Tab moves to the next control, not a second tab stop"
    );
    // The list announces itself as a TabList with two Tabs inside it.
    let _ = harness.get_by_role(Role::TabList);
    assert_eq!(
        harness.get_all_by_role(Role::Tab).count(),
        2,
        "each tab is a Tab node"
    );
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
            let mut selected = 0;
            TabList::new("tabs", &mut selected)
                .size(TabSize::Small)
                .appearance(fluent2_egui::surfaces::TabAppearance::Subtle)
                .tab_icon(Icon::Calendar, "Home")
                .tab("Files")
                .show(ui);
            let mut checked = true;
            Card::new()
                .appearance(CardAppearance::Outline)
                .label("Pick")
                .selectable(&mut checked)
                .show(ui, |ui| {
                    ui.label("Body");
                });
        });
        harness.run();
        let _ = harness.get_by_role(Role::TabList);
        let _ = harness.get_by_role(Role::CheckBox);
    }
}

#[test]
fn selectable_card_toggles() {
    let state = Box::leak(Box::new(Cell::new(false))) as &Cell<bool>;
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let mut value = state.get();
        let response = Card::new()
            .id_salt("card")
            .label("Pick")
            .selectable(&mut value)
            .show(ui, |ui| {
                ui.label("Body");
            });
        response.request_focus();
        state.set(value);
    });
    harness.run();
    assert!(!state.get());
    harness.get_by_role(Role::CheckBox).click();
    harness.run();
    assert!(state.get(), "clicking the card toggles it on");
    harness.get_by_role(Role::CheckBox).click();
    harness.run();
    assert!(!state.get(), "clicking again toggles it off");
}

#[test]
fn selectable_card_activates_on_space() {
    let state = Box::leak(Box::new(Cell::new(false))) as &Cell<bool>;
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let mut value = state.get();
        let response = Card::new()
            .id_salt("card")
            .label("Pick")
            .selectable(&mut value)
            .show(ui, |ui| {
                ui.label("Body");
            });
        response.request_focus();
        state.set(value);
    });
    harness.run();
    harness.run();
    harness.key_down(egui::Key::Space);
    harness.run();
    assert!(state.get());
}

#[test]
fn header_action_inside_a_selectable_card_fires_without_toggling() {
    let state = Box::leak(Box::new(Cell::new(false))) as &Cell<bool>;
    let fired = Box::leak(Box::new(Cell::new(false))) as &Cell<bool>;
    let mut harness = Harness::new_ui(move |ui| {
        install_light(ui);
        let mut value = state.get();
        let _ = Card::new()
            .id_salt("card")
            .label("Pick")
            .selectable(&mut value)
            .show(ui, |ui| {
                CardHeader::new("Title")
                    .action(|ui| {
                        if ui.button("Go").clicked() {
                            fired.set(true);
                        }
                    })
                    .show(ui);
            });
        state.set(value);
    });
    harness.run();
    harness.run();
    harness.get_by_role_and_label(Role::Button, "Go").click();
    harness.run();
    assert!(fired.get(), "the header action fires");
    assert!(!state.get(), "a click on the action does not toggle the card");
}

#[test]
fn selectable_card_is_named_by_its_label() {
    let mut harness = Harness::new_ui(|ui| {
        install_light(ui);
        let mut value = false;
        let _ = Card::new()
            .label("Pick me")
            .selectable(&mut value)
            .show(ui, |ui| {
                ui.label("Body");
            });
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::CheckBox, "Pick me");
}
