//! TagPicker behaviour (WI-1941): separators and paste, suggestions, removal, validation and
//! the AccessKit tree.

use egui::accesskit::Role;
use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{TagPicker, TagSuggestion};
use fluent2_egui::{Theme, install};

#[derive(Default)]
struct State {
    tags: Vec<String>,
    query: String,
    suggestions: Vec<TagSuggestion>,
    query_changes: usize,
    removed: Vec<String>,
}

fn harness(state: State) -> Harness<'static, State> {
    Harness::builder()
        .with_size(egui::vec2(500.0, 300.0))
        .build_ui_state(
            |ui, state: &mut State| {
                install(ui.ctx(), &Theme::default());
                let valid = |tag: &str| tag.contains('@');
                // The app answers each query change with suggestions, as a fetch would.
                let out = TagPicker::new("to", &mut state.tags, &mut state.query)
                    .label("To")
                    .suggestions(&state.suggestions)
                    .validate(&valid)
                    .show(ui);
                if out.query_changed {
                    state.query_changes += 1;
                    state.suggestions = if state.query.starts_with("ad") {
                        vec![
                            TagSuggestion::new("ada@example.com").label("Ada Lovelace"),
                            TagSuggestion::new("adam@example.com").label("Adam Smith"),
                        ]
                    } else {
                        Vec::new()
                    };
                }
                state.removed.extend(out.removed);
            },
            state,
        )
}

fn type_into(harness: &mut Harness<'static, State>, text: &str) {
    harness.get_by_role_and_label(Role::ComboBox, "To").focus();
    harness.run_steps(2);
    harness.event(Event::Text(text.to_owned()));
    harness.run_steps(2);
}

#[test]
fn separators_and_paste_add_tags() {
    let mut harness = harness(State::default());
    harness.run_steps(2);
    type_into(&mut harness, "a@x.com, b@y.com;c");
    assert_eq!(harness.state().tags, ["a@x.com", "b@y.com"]);
    assert_eq!(harness.state().query, "c");
    harness.event(Event::Paste("d@z.com\ne@z.com; ".to_owned()));
    harness.run_steps(2);
    assert_eq!(
        harness.state().tags,
        ["a@x.com", "b@y.com", "cd@z.com", "e@z.com"]
    );
    assert!(harness.state().query.is_empty());
}

#[test]
fn enter_picks_the_highlighted_suggestion() {
    let mut harness = harness(State::default());
    harness.run_steps(2);
    type_into(&mut harness, "ad");
    assert!(
        harness.state().query_changes > 0,
        "a query change asks the app for suggestions"
    );
    let _ = harness.get_by_label("Ada Lovelace <ada@example.com>");
    harness.key_press(Key::ArrowDown);
    harness.run_steps(2);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["adam@example.com"]);
    assert!(harness.state().query.is_empty());
}

#[test]
fn enter_adds_free_text_and_backspace_removes_the_last_tag() {
    let mut harness = harness(State::default());
    harness.run_steps(2);
    type_into(&mut harness, "bob@x.com");
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["bob@x.com"]);
    harness.key_press(Key::Backspace);
    harness.run_steps(2);
    assert!(harness.state().tags.is_empty());
    assert_eq!(harness.state().removed, ["bob@x.com"]);
}

#[test]
fn remove_buttons_and_invalid_tags_in_the_accesskit_tree() {
    let mut harness = harness(State {
        tags: vec!["ok@x.com".to_owned(), "not-an-address".to_owned()],
        ..State::default()
    });
    harness.run_steps(2);
    let field = harness.get_by_role_and_label(Role::ComboBox, "To");
    assert_eq!(
        field.accesskit_node().value().as_deref(),
        Some("ok@x.com, not-an-address")
    );
    let bad = harness.get_by_role_and_label(Role::ListItem, "not-an-address");
    assert!(
        bad.accesskit_node().data().invalid().is_some(),
        "an invalid tag is marked invalid"
    );
    let good = harness.get_by_role_and_label(Role::ListItem, "ok@x.com");
    assert!(good.accesskit_node().data().invalid().is_none());

    harness
        .get_by_role_and_label(Role::Button, "Remove ok@x.com")
        .click();
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["not-an-address"]);
    let _ = Modifiers::NONE;
}

#[test]
fn renders_in_every_theme_with_a_spinner_while_loading() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::new_ui(move |ui| {
            install(ui.ctx(), &theme);
            let mut tags = vec!["a@x.com".to_owned(), "nope".to_owned()];
            let mut query = String::from("ad");
            let valid = |tag: &str| tag.contains('@');
            TagPicker::new("cc", &mut tags, &mut query)
                .placeholder("Add people")
                .loading(true)
                .validate(&valid)
                .show(ui);
        });
        harness.run_steps(3);
    }
}
