//! EditableList through kittest (no wgpu): rendering in each theme, add, edit, remove and the
//! keyboard. The app side applies each event to its rows, as a real app would.

use egui::accesskit::{Invalid, Role};
use egui::{Key, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::data::editable_list::{EditableKind, EditableList, EditableListEvent, EditableListState};
use fluent2_egui::{Theme, install};

#[derive(Default)]
struct Model {
    rows: Vec<String>,
    state: EditableListState,
    kind: EditableKind,
    /// Every event, in order, so tests can check what was emitted.
    seen: Vec<EditableListEvent>,
}

fn model(rows: &[&str], kind: EditableKind) -> Model {
    Model {
        rows: rows.iter().map(|row| (*row).to_owned()).collect(),
        kind,
        ..Model::default()
    }
}

fn build(model: Model, theme: fn() -> Theme) -> Harness<'static, Model> {
    let mut harness = Harness::builder()
        .with_size(Vec2::new(360.0, 320.0))
        .build_ui_state(
            move |ui, model: &mut Model| {
                install(ui.ctx(), &theme());
                let response = EditableList::new("filters", &model.rows)
                    .label("IP filters")
                    .kind(model.kind)
                    .show(ui, &mut model.state);
                for event in response.events {
                    match &event {
                        EditableListEvent::Added(value) => model.rows.push(value.clone()),
                        EditableListEvent::Edited { index, value } => {
                            if let Some(row) = model.rows.get_mut(*index) {
                                *row = value.clone();
                            }
                        }
                        EditableListEvent::Removed { index } => {
                            if *index < model.rows.len() {
                                model.rows.remove(*index);
                            }
                        }
                    }
                    model.seen.push(event);
                }
            },
            model,
        );
    harness.run();
    harness
}

/// kittest's `type_text` sends text to the focused widget, so focus the field first.
fn focus_and_type(harness: &mut Harness<'static, Model>, label: &str, text: &str) {
    harness.get_by_label(label).focus();
    harness.run();
    harness.get_by_label(label).type_text(text);
}

/// Empties the open editor: End, then Backspace for the whole value.
fn clear_editor(harness: &mut Harness<'static, Model>) {
    harness.key_press(Key::End);
    for _ in 0..32 {
        harness.key_press(Key::Backspace);
    }
    harness.run_steps(2);
}

fn light() -> Theme {
    Theme::light(fluent2_tokens::FLUENT)
}

#[test]
fn renders_in_light_dark_and_high_contrast() {
    let themes: [fn() -> Theme; 3] = [
        light,
        || Theme::dark(fluent2_tokens::FLUENT),
        Theme::high_contrast,
    ];
    for theme in themes {
        let harness = build(model(&["10.0.0.0/8"], EditableKind::Text), theme);
        assert_eq!(
            harness.get_by_label("IP filters").accesskit_node().role(),
            Role::List
        );
        assert_eq!(
            harness.get_by_label("10.0.0.0/8").accesskit_node().role(),
            Role::Button
        );
        assert_eq!(
            harness.get_by_label("New item").accesskit_node().role(),
            Role::TextInput
        );
    }
}

#[test]
fn enter_in_the_add_field_appends_a_trimmed_row() {
    let mut harness = build(model(&["a"], EditableKind::Text), light);
    focus_and_type(&mut harness, "New item", "  b  ");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert_eq!(harness.state().rows, ["a", "b"]);
    assert_eq!(harness.state().seen, [EditableListEvent::Added("b".to_owned())]);
}

#[test]
fn the_add_button_appends_a_row() {
    let mut harness = build(model(&[], EditableKind::Text), light);
    focus_and_type(&mut harness, "New item", "c");
    harness.run();
    harness.get_by_label("Add").click();
    harness.run();
    assert_eq!(harness.state().rows, ["c"]);
}

#[test]
fn blank_text_is_refused_with_a_message() {
    let mut harness = build(model(&[], EditableKind::Text), light);
    focus_and_type(&mut harness, "New item", "   ");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().seen.is_empty());
    assert!(harness.query_by_label("Enter some text").is_some());
}

#[test]
fn number_rows_refuse_text_and_keep_the_add_field_open() {
    let mut harness = build(model(&["1.5"], EditableKind::Number), light);
    focus_and_type(&mut harness, "New item", "fast");
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().seen.is_empty());
    assert!(harness.query_by_label("Enter a number").is_some());
    assert_eq!(
        harness.get_by_label("New item").accesskit_node().data().invalid(),
        Some(Invalid::True)
    );
}

#[test]
fn enter_commits_an_edit_and_escape_cancels_it() {
    let mut harness = build(model(&["10.0.0.0/8"], EditableKind::Text), light);
    harness.get_by_label("10.0.0.0/8").click();
    harness.run_steps(3);
    clear_editor(&mut harness);
    harness.get_by_label("Edit 10.0.0.0/8").type_text("10.1.0.0/16");
    harness.run_steps(3);
    harness.key_press(Key::Escape);
    harness.run_steps(3);
    assert!(harness.state().seen.is_empty(), "Escape must not commit");
    assert_eq!(harness.state().rows, ["10.0.0.0/8"]);

    harness.get_by_label("10.0.0.0/8").click();
    harness.run_steps(3);
    clear_editor(&mut harness);
    harness.get_by_label("Edit 10.0.0.0/8").type_text("10.1.0.0/16");
    harness.run_steps(3);
    harness.key_press(Key::Enter);
    harness.run_steps(3);
    assert_eq!(harness.state().rows, ["10.1.0.0/16"]);
    assert_eq!(
        harness.state().seen,
        [EditableListEvent::Edited {
            index: 0,
            value: "10.1.0.0/16".to_owned(),
        }]
    );
}

#[test]
fn an_unchanged_edit_emits_nothing() {
    let mut harness = build(model(&["x"], EditableKind::Text), light);
    harness.get_by_label("x").click();
    harness.run();
    harness.key_press(Key::Enter);
    harness.run();
    assert!(harness.state().seen.is_empty());
}

#[test]
fn f2_edits_the_focused_row_and_delete_removes_it() {
    let mut harness = build(model(&["one", "two"], EditableKind::Text), light);
    harness.get_by_label("two").focus();
    harness.run();
    harness.key_press(Key::F2);
    harness.run();
    assert!(harness.query_by_label("Edit two").is_some());
    harness.key_press(Key::Escape);
    harness.run();

    harness.get_by_label("one").focus();
    harness.run();
    harness.key_press(Key::Delete);
    harness.run();
    assert_eq!(harness.state().rows, ["two"]);
    assert_eq!(harness.state().seen, [EditableListEvent::Removed { index: 0 }]);
}

#[test]
fn the_remove_button_removes_its_row() {
    let mut harness = build(model(&["one", "two"], EditableKind::Text), light);
    harness.get_by_label("Remove two").click();
    harness.run();
    assert_eq!(harness.state().rows, ["one"]);
    assert_eq!(harness.state().seen, [EditableListEvent::Removed { index: 1 }]);
}
