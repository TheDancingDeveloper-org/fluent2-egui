//! Tag, InteractionTag and TagGroup behaviour (WI-1934): dismissing by click and keyboard, the
//! primary action, roving focus, focus after a dismiss, and the AccessKit tree.

use egui::Key;
use egui::accesskit::Role;
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{Avatar, InteractionTag, Tag, TagAppearance, TagGroup, TagSize};
use fluent2_egui::foundation::icon::IconName;
use fluent2_egui::{Theme, install};

#[derive(Default)]
struct State {
    tags: Vec<String>,
    interactive: bool,
    clicked: Vec<usize>,
    dismissed: Vec<String>,
}

fn group(state: State) -> Harness<'static, State> {
    Harness::builder()
        .with_size(egui::vec2(600.0, 200.0))
        .build_ui_state(
            |ui, state: &mut State| {
                install(ui.ctx(), &Theme::default());
                let tags = state.tags.iter().map(|t| Tag::new(t.clone())).collect();
                let out = TagGroup::new("filters", tags)
                    .label("Filters")
                    .interactive(state.interactive)
                    .dismissible(true)
                    .show(ui);
                if let Some(index) = out.clicked {
                    state.clicked.push(index);
                }
                if let Some(index) = out.dismissed
                    && index < state.tags.len()
                {
                    state.dismissed.push(state.tags.remove(index));
                }
            },
            state,
        )
}

fn three(interactive: bool) -> State {
    State {
        tags: vec!["Bug".to_owned(), "Docs".to_owned(), "UI".to_owned()],
        interactive,
        ..State::default()
    }
}

#[test]
fn group_is_a_list_of_items_with_named_dismiss_buttons() {
    let mut harness = group(three(false));
    harness.run_steps(2);
    let list = harness.get_by_role_and_label(Role::List, "Filters");
    let items = list.query_all_by_role(Role::ListItem).count();
    assert_eq!(items, 3, "each tag is a ListItem in the List");
    let _ = harness.get_by_role_and_label(Role::ListItem, "Docs");
    harness.get_by_role_and_label(Role::Button, "Remove Docs").click();
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["Bug", "UI"]);
}

#[test]
fn arrows_rove_and_delete_moves_focus_to_the_next_tag() {
    let mut harness = group(three(false));
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, "Remove Bug").focus();
    harness.run_steps(2);
    harness.key_press(Key::ArrowRight);
    harness.run_steps(2);
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Remove Docs")
            .is_focused(),
        "Right moves to the next tag"
    );
    harness.key_press(Key::Delete);
    harness.run_steps(3);
    assert_eq!(harness.state().dismissed, ["Docs"]);
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Remove UI")
            .is_focused(),
        "focus moves to the tag after the dismissed one"
    );
    harness.key_press(Key::Backspace);
    harness.run_steps(3);
    assert_eq!(harness.state().tags, ["Bug"]);
    assert!(
        harness
            .get_by_role_and_label(Role::Button, "Remove Bug")
            .is_focused()
    );
}

#[test]
fn interaction_tags_have_a_primary_action_and_a_dismiss_target() {
    let mut harness = group(three(true));
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, "Docs").click();
    harness.run_steps(2);
    assert_eq!(harness.state().clicked, [1]);
    harness.get_by_role_and_label(Role::Button, "Remove UI").click();
    harness.run_steps(2);
    assert_eq!(harness.state().tags, ["Bug", "Docs"]);
}

#[test]
fn standalone_tag_dismisses_from_the_keyboard() {
    let mut harness = Harness::builder().build_ui_state(
        |ui, dismissed: &mut bool| {
            install(ui.ctx(), &Theme::default());
            if Tag::new("Ada").dismissible(true).show(ui).dismissed {
                *dismissed = true;
            }
        },
        false,
    );
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, "Remove Ada").focus();
    harness.run_steps(2);
    harness.key_press(Key::Delete);
    harness.run_steps(2);
    assert!(*harness.state());
}

#[test]
fn disabled_tags_do_not_dismiss() {
    let mut harness = Harness::builder().build_ui_state(
        |ui, dismissed: &mut bool| {
            install(ui.ctx(), &Theme::default());
            ui.add_enabled_ui(false, |ui| {
                if Tag::new("Ada").dismissible(true).show(ui).dismissed {
                    *dismissed = true;
                }
            });
        },
        false,
    );
    harness.run_steps(2);
    harness.get_by_role_and_label(Role::Button, "Remove Ada").click();
    harness.run_steps(2);
    assert!(!*harness.state());
}

#[test]
fn renders_every_variant_in_every_theme() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let mut harness = Harness::builder()
            .with_size(egui::vec2(1200.0, 900.0))
            .build_ui(move |ui| {
                install(ui.ctx(), &theme);
                for appearance in [
                    TagAppearance::Filled,
                    TagAppearance::Outline,
                    TagAppearance::Brand,
                ] {
                    for size in [TagSize::ExtraSmall, TagSize::Small, TagSize::Medium] {
                        ui.add(
                            Tag::new("Label")
                                .appearance(appearance)
                                .size(size)
                                .icon(IconName::Person),
                        );
                        ui.add(
                            Tag::new("Ada Lovelace")
                                .appearance(appearance)
                                .size(size)
                                .avatar(Avatar::new("Ada Lovelace"))
                                .secondary("Owner")
                                .dismissible(true),
                        );
                        ui.add(
                            InteractionTag::new(
                                Tag::new("Open")
                                    .appearance(appearance)
                                    .size(size)
                                    .dismissible(true),
                            )
                            .selected(true),
                        );
                    }
                }
            });
        harness.run_steps(2);
        assert_eq!(
            harness.query_all_by_role_and_label(Role::Button, "Open").count(),
            9
        );
        let labels = harness
            .query_all_by_role(Role::Label)
            .filter(|n| n.accesskit_node().label().as_deref() == Some("Ada Lovelace, Owner"))
            .count();
        assert_eq!(
            labels, 9,
            "a standalone tag is a Label named by its text and secondary"
        );
    }
}
