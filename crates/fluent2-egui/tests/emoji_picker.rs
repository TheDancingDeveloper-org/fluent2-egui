//! Emoji picker: themes, search, keyboard, AccessKit, and a missing glyph that does not panic.

use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::{EmojiCategory, EmojiPicker, filter_emoji};
use fluent2_egui::{Theme, install};

fn install_kind(ui: &egui::Ui, theme: Theme) {
    install(ui.ctx(), &theme);
}

#[test]
fn picker_renders_in_light_dark_and_high_contrast() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            install_kind(ui, theme.clone());
            let _ = EmojiPicker::new("themes").show(ui);
        });
        harness.run();
        let _ = harness.get_by_role_and_label(Role::TextInput, "Search emoji");
        let _ = harness.get_by_role_and_label(Role::Button, "Smileys");
        let _ = harness.get_by_role_and_label(Role::Button, "grinning face");
    }
}

#[test]
fn search_narrows_the_grid() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        let _ = EmojiPicker::new("search").show(ui);
    });
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, "Search emoji")
        .focus();
    harness.run();
    harness
        .get_by_role_and_label(Role::TextInput, "Search emoji")
        .type_text("pizza");
    harness.run();
    let _ = harness.get_by_role_and_label(Role::Button, "pizza");
    assert!(harness.query_by_label("grinning face").is_none());
}

#[test]
fn keyboard_moves_and_inserts() {
    let mut harness = Harness::builder().build_ui_state(
        |ui, picked: &mut Option<String>| {
            install(ui.ctx(), &Theme::default());
            let response = EmojiPicker::new("keys").show(ui);
            if let Some(emoji) = response.picked {
                *picked = Some(emoji);
            }
        },
        None,
    );
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    harness.key_press(egui::Key::Enter);
    harness.run();
    assert_eq!(harness.state().as_deref(), Some("😃"));
}

#[test]
fn missing_glyph_does_not_panic() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        ui.label("\u{1FAE0}");
        let _ = EmojiPicker::new("tofu").show(ui);
    });
    harness.run();
}

#[test]
fn filter_matches_category() {
    assert!(
        filter_emoji(EmojiCategory::Food, "pizza")
            .iter()
            .any(|e| e.emoji == "🍕")
    );
    assert!(filter_emoji(EmojiCategory::Smileys, "pizza").is_empty());
}
