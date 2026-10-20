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

/// A fetch that answers only when the test delivers it.
#[derive(Default)]
struct Slow {
    tags: Vec<String>,
    query: String,
    /// The query the current results are for, and the results.
    results_for: String,
    results: Vec<TagSuggestion>,
    loading: bool,
}

fn slow_harness() -> Harness<'static, Slow> {
    Harness::builder()
        .with_size(egui::vec2(500.0, 300.0))
        .build_ui_state(
            |ui, state: &mut Slow| {
                install(ui.ctx(), &Theme::default());
                let out = TagPicker::new("to", &mut state.tags, &mut state.query)
                    .label("To")
                    .suggestions_for(state.results_for.clone(), &state.results)
                    .loading(state.loading)
                    .show(ui);
                if out.query_changed {
                    state.loading = true;
                }
            },
            Slow::default(),
        )
}

fn deliver(harness: &mut Harness<'static, Slow>) {
    let state = harness.state_mut();
    state.results_for = state.query.clone();
    state.results = vec![TagSuggestion::new("ada@example.com").label("Ada Lovelace")];
    state.loading = false;
    harness.run_steps(2);
}

#[test]
fn enter_never_adds_a_suggestion_for_an_older_query() {
    let mut harness = slow_harness();
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::ComboBox, "To").focus();
    harness.run_steps(2);
    harness.event(Event::Text("ad".to_owned()));
    harness.run_steps(2);
    deliver(&mut harness);
    let _ = harness.get_by_label("Ada Lovelace <ada@example.com>");

    // Typing more and pressing Enter before the new results arrive adds the typed text.
    harness.event(Event::Text("am".to_owned()));
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["adam"]);

    // Results for an old query are not offered even when the app forgets to set loading.
    harness.state_mut().loading = false;
    harness.event(Event::Text("x".to_owned()));
    harness.run_steps(2);
    harness.state_mut().loading = false;
    harness.run_steps(2);
    assert!(
        harness.query_by_label("Ada Lovelace <ada@example.com>").is_none(),
        "results for \"ad\" are hidden while the query is \"x\""
    );
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["adam", "x"]);

    // Once results for the current query arrive, Enter picks from them again.
    harness.event(Event::Text("ad".to_owned()));
    harness.run_steps(2);
    deliver(&mut harness);
    harness.key_press(Key::Enter);
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["adam", "x", "ada@example.com"]);
}
