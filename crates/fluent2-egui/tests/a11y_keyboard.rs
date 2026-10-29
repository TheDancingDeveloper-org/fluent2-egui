//! Keyboard and focus audit (WI-1586): every basic control on one page is reached with Tab,
//! each focus stop paints the Fluent focus ring, and reduced motion stops the motion that
//! does not go through `motion::animate`.

use egui::accesskit::Role;
use egui::epaint::{RectShape, Shape};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable, by};
use fluent2_egui::controls::Button;
use fluent2_egui::foundation::motion;
use fluent2_egui::surfaces::{Intent, Toast, ToastPosition, Toaster};
use fluent2_egui::theme::c;
use fluent2_egui::{
    CheckState, Checkbox, Dropdown, Field, Input, Link, RadioGroup, Slider, SpinButton, Spinner, Switch,
    TabList, Theme, ToggleButton, install,
};

#[derive(Default)]
struct Page {
    check: CheckState,
    radio: usize,
    switch: bool,
    toggle: bool,
    text: String,
    slider: f64,
    spin: f64,
    pick: Option<usize>,
    tab: usize,
}

fn page(theme: Theme) -> Harness<'static, Page> {
    Harness::builder()
        .with_size(egui::vec2(480.0, 900.0))
        .build_ui_state(
            move |ui, page: &mut Page| {
                install(ui.ctx(), &theme);
                ui.add(Button::new("Save"));
                ui.add(ToggleButton::new(&mut page.toggle, "Bold"));
                ui.add(Checkbox::new(&mut page.check, "Agree"));
                RadioGroup::new(&mut page.radio)
                    .label("Size")
                    .option("Small")
                    .option("Large")
                    .show(ui);
                ui.add(Switch::new(&mut page.switch, "Wi-Fi"));
                ui.add(
                    Input::new(&mut page.text)
                        .label("Name")
                        .placeholder("Type a name"),
                );
                ui.add(Slider::new(&mut page.slider, 0.0..=10.0).label("Volume"));
                // No label of its own: the Field's label names it (WI-1843).
                Field::new("Copies").show(ui, |ui| ui.add(SpinButton::new(&mut page.spin)));
                ui.add(Dropdown::new("pick", &mut page.pick, &["Red", "Green"]).placeholder("Colour"));
                TabList::new("tabs", &mut page.tab)
                    .tab("Home")
                    .tab("Insert")
                    .show(ui);
                ui.add(Link::new("Help"));
            },
            Page::default(),
        )
}

/// Labels of every focused node; normally one.
fn focused(harness: &Harness<'_, Page>) -> Vec<(Role, String)> {
    harness
        .query_all(by().predicate(|node| node.is_focused()))
        .map(|node| {
            let node = node.accesskit_node();
            (node.role(), node.label().unwrap_or_default())
        })
        .collect()
}

fn has_focus_ring(shapes: &[egui::epaint::ClippedShape], outer: egui::Color32) -> bool {
    fn walk(shape: &Shape, outer: egui::Color32) -> bool {
        match shape {
            Shape::Rect(RectShape { stroke, .. }) => stroke.color == outer && stroke.width >= 2.0,
            Shape::Vec(shapes) => shapes.iter().any(|shape| walk(shape, outer)),
            _ => false,
        }
    }
    shapes.iter().any(|clipped| walk(&clipped.shape, outer))
}

fn walk_tab_order(theme: Theme) -> Vec<(Role, String)> {
    let ring = c(theme.color.stroke_focus_2);
    let mut harness = page(theme);
    harness.run();
    let mut stops = Vec::new();
    for _ in 0..24 {
        harness.key_press(egui::Key::Tab);
        harness.run();
        let now = focused(&harness);
        assert!(now.len() <= 1, "two nodes focused at once: {now:?}");
        let Some(stop) = now.into_iter().next() else {
            panic!("Tab left focus on no AccessKit node (after {stops:?})");
        };
        assert!(
            has_focus_ring(&harness.output().shapes, ring),
            "{stop:?} has keyboard focus but no focus ring (after {stops:?})"
        );
        if stops.contains(&stop) {
            break;
        }
        stops.push(stop);
    }
    stops
}

#[test]
fn tab_reaches_every_control_with_a_visible_ring() {
    for theme in [
        Theme::default(),
        Theme::dark(fluent2_egui::tokens::FLUENT),
        Theme::high_contrast(),
    ] {
        let stops = walk_tab_order(theme);
        let reached = |role: Role, label: &str| stops.iter().any(|(r, l)| *r == role && l.contains(label));
        for (role, label) in [
            (Role::Button, "Save"),
            (Role::Button, "Bold"),
            (Role::CheckBox, "Agree"),
            (Role::Switch, "Wi-Fi"),
            (Role::Link, "Help"),
        ] {
            assert!(
                reached(role, label),
                "Tab never reached {role:?} {label:?}; stops: {stops:#?}"
            );
        }
        // Every stop has a name a screen reader can read (WI-1843).
        for (role, label) in &stops {
            assert!(
                !label.trim().is_empty(),
                "{role:?} stop has no name; stops: {stops:#?}"
            );
        }
        assert!(
            reached(Role::Slider, "Volume"),
            "the slider is named; stops: {stops:#?}"
        );
        assert!(
            reached(Role::SpinButton, "Copies"),
            "the spin button takes its Field's label; stops: {stops:#?}"
        );
        for role in [
            Role::RadioButton,
            Role::TextInput,
            Role::Slider,
            Role::SpinButton,
            Role::ComboBox,
            Role::Tab,
        ] {
            assert!(
                stops.iter().any(|(r, _)| *r == role),
                "Tab never reached a {role:?}; stops: {stops:#?}"
            );
        }
    }
}

fn toast_harness(reduced: bool) -> Harness<'static, u32> {
    Harness::builder().with_step_dt(0.02).build_ui_state(
        move |ui, step: &mut u32| {
            let ctx = ui.ctx().clone();
            install(&ctx, &Theme::default());
            motion::set_reduced_motion(&ctx, reduced);
            if *step == 0 {
                let _ = Toaster::push(
                    &ctx,
                    Toast {
                        intent: Intent::Success,
                        title: "Saved".to_owned(),
                        body: None,
                        timeout: None,
                        action: None,
                    },
                );
            }
            Toaster::show(&ctx, ToastPosition::BottomEnd);
            *step += 1;
        },
        0u32,
    )
}

/// The top edge of the painted toast surface on each of the first frames. The AccessKit
/// bounds stay on the layout slot, so this reads the paint.
fn toast_tops(reduced: bool) -> Vec<f32> {
    let fill = c(Theme::default().color.status_success_background_1);
    let same_rgb = |color: egui::Color32| {
        let unmultiplied = color.to_srgba_unmultiplied();
        unmultiplied[..3] == fill.to_srgba_unmultiplied()[..3] && unmultiplied[3] > 0
    };
    fn find(shape: &Shape, same_rgb: &dyn Fn(egui::Color32) -> bool) -> Option<f32> {
        match shape {
            Shape::Rect(rect) if same_rgb(rect.fill) => Some(rect.rect.min.y),
            Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, same_rgb)),
            _ => None,
        }
    }
    let mut harness = toast_harness(reduced);
    let mut tops = Vec::new();
    for _ in 0..20 {
        harness.step();
        if let Some(top) = harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| find(&clipped.shape, &same_rgb))
        {
            tops.push(top);
        }
    }
    tops
}

#[test]
fn toasts_slide_only_without_reduced_motion() {
    let moving = toast_tops(false);
    let still = toast_tops(true);
    assert!(
        moving.windows(2).any(|pair| (pair[0] - pair[1]).abs() > 0.2),
        "the enter slide never moved: {moving:?}"
    );
    // The first frames lay the Area out; from the third on the toast must not move.
    let settled = still.get(2..).unwrap_or_default();
    assert!(!settled.is_empty(), "toast never shown");
    assert!(
        settled.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.01),
        "toast moved under reduced motion: {still:?}"
    );
}

#[test]
fn spinner_stops_repainting_under_reduced_motion() {
    for (reduced, expect_repaint) in [(false, true), (true, false)] {
        let mut harness = Harness::new_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            motion::set_reduced_motion(ui.ctx(), reduced);
            ui.add(Spinner::new());
        });
        harness.run_steps(3);
        let repaint = harness
            .output()
            .viewport_output
            .values()
            .any(|viewport| viewport.repaint_delay.is_zero());
        assert_eq!(repaint, expect_repaint, "reduced motion {reduced}");
    }
}

#[test]
fn indeterminate_progress_is_still_under_reduced_motion() {
    let fill = c(Theme::default().color.compound_brand_background);
    let mut harness = Harness::new_ui(move |ui| {
        install(ui.ctx(), &Theme::default());
        motion::set_reduced_motion(ui.ctx(), true);
        ui.add(fluent2_egui::ProgressBar::new(None));
    });
    let mut lefts = Vec::new();
    for _ in 0..6 {
        harness.step();
        let left = harness
            .output()
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                Shape::Rect(rect) if rect.fill == fill => Some(rect.rect.left()),
                _ => None,
            });
        lefts.extend(left);
    }
    assert_eq!(lefts.len(), 6, "the sweep was not painted every frame");
    assert!(
        lefts.windows(2).all(|pair| (pair[0] - pair[1]).abs() < 0.01),
        "the sweep moved under reduced motion: {lefts:?}"
    );
    let repaint = harness
        .output()
        .viewport_output
        .values()
        .any(|viewport| viewport.repaint_delay.is_zero());
    assert!(!repaint, "indeterminate progress still asks for every frame");
}

/// Focus returns to the opener when the dialog closes. Found by the Orca walk: the opener is
/// focused once by Tab, not re-requested every frame.
#[test]
fn escape_returns_focus_to_a_keyboard_opener() {
    use fluent2_egui::Dialog;
    let mut harness = Harness::new_ui_state(
        |ui, open: &mut bool| {
            install(ui.ctx(), &Theme::default());
            ui.add(Button::new("Before"));
            if ui.add(Button::new("Open dialog")).clicked() {
                *open = true;
            }
            let _ = Dialog::new("walk")
                .title("Save changes?")
                .actions(|actions| actions.secondary("cancel", "Cancel"))
                .show(ui.ctx(), open, |ui| {
                    ui.label("Body");
                });
        },
        false,
    );
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(
        harness.get_by_label("Open dialog").is_focused(),
        "Tab reached the opener"
    );
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(*harness.state(), "Space opened the dialog");
    harness.key_press(egui::Key::Escape);
    harness.run();
    assert!(!*harness.state(), "Escape closed the dialog");
    assert!(
        harness.get_by_label("Open dialog").is_focused(),
        "focus did not return to the opener"
    );
}

#[test]
fn opening_backstage_from_the_keyboard_moves_focus_into_it() {
    use fluent2_egui::office::backstage::{Backstage, BackstageState};
    // WI-1850: Tab to File, Space. Focus lands on the selected Backstage entry, and Tab
    // moves on inside the backstage instead of back to the page.
    let mut harness = Harness::new_ui_state(
        |ui, state: &mut BackstageState| {
            install(ui.ctx(), &Theme::default());
            if !state.open {
                ui.add(Button::new("Before"));
                if ui.add(Button::new("File")).clicked() {
                    state.open = true;
                }
            }
            let _ = Backstage::new(state)
                .page("info", "Info", |ui| {
                    ui.label("Info body");
                })
                .page("new", "New", |ui| {
                    ui.label("New body");
                })
                .show(ui);
        },
        BackstageState::default(),
    );
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(harness.get_by_label("File").is_focused(), "Tab reached File");
    harness.key_press(egui::Key::Space);
    harness.run();
    assert!(harness.state().open, "Space opened the backstage");
    assert!(
        harness
            .get_by_role_and_label(Role::ListBoxOption, "Info")
            .is_focused(),
        "focus moved to the selected backstage entry"
    );
    harness.key_press(egui::Key::Tab);
    harness.run();
    assert!(
        harness
            .get_by_role_and_label(Role::ListBoxOption, "New")
            .is_focused(),
        "Tab stays inside the backstage"
    );
}

#[test]
fn dialog_is_described_by_its_body_on_open() {
    use fluent2_egui::Dialog;
    // WI-1850: focus opens on the primary button, so the dialog node carries its body as the
    // description relation, and an explicit description when one is given.
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        let mut open = true;
        let _ = Dialog::new("described")
            .title("Save changes?")
            .description("Your document has unsaved changes.")
            .actions(|actions| actions.primary("save", "Save"))
            .show(ui.ctx(), &mut open, |ui| {
                ui.label("Your document has unsaved changes.");
            });
    });
    harness.run();
    harness.run();
    let dialog = harness.get_by_role_and_label(Role::Dialog, "Save changes?");
    let node = dialog.accesskit_node();
    assert_eq!(
        node.description().as_deref(),
        Some("Your document has unsaved changes.")
    );
    let described_by = node.data().described_by().to_vec();
    assert_eq!(described_by.len(), 1, "one describing node");
    let Some(body) = harness
        .root()
        .children_recursive()
        .find(|n| n.accesskit_node().locate().0 == described_by[0])
    else {
        panic!("the describing node is not in the tree");
    };
    assert!(
        body.children_recursive()
            .any(
                |n| n.accesskit_node().label().as_deref() == Some("Your document has unsaved changes.")
                    || n.accesskit_node().value().as_deref() == Some("Your document has unsaved changes.")
            ),
        "the describing node holds the body text"
    );
}

#[test]
fn radio_group_is_named_by_its_label_or_its_field() {
    let mut harness = Harness::new_ui(|ui| {
        install(ui.ctx(), &Theme::default());
        let mut a = 0;
        let mut b = 0;
        let mut volume = 3.0;
        RadioGroup::new(&mut a)
            .label("Size")
            .option("Small")
            .option("Large")
            .show(ui);
        Field::new("Alignment").show(ui, |ui| {
            RadioGroup::new(&mut b).option("Left").option("Right").show(ui);
        });
        // Outside any Field again: the default name is back to the generic one.
        ui.add(Slider::new(&mut volume, 0.0..=10.0));
    });
    harness.run();
    let _ = harness.get_by_role_and_label(Role::RadioGroup, "Size");
    let _ = harness.get_by_role_and_label(Role::RadioGroup, "Alignment");
    let _ = harness.get_by_role_and_label(Role::Slider, "Slider");
}
