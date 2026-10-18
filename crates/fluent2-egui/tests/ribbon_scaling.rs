//! Ribbon adaptive scaling (spec §5): the steps the ribbon picks at real widths, read back
//! from the AccessKit tree, and the hysteresis band when the window is resized.

use egui::accesskit::Role;
use egui::{Event, Modifiers, PointerButton, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::office::ribbon::{
    GroupBuilder, Ribbon, RibbonLarge, RibbonSmall, RibbonState, RibbonTab, ScalingStep, TabBuilder,
};
use fluent2_egui::{IconName, Theme, install};

/// Each group: a large probe button, two more large buttons and two small ones.
const GROUPS: [(&str, &str, [&str; 3]); 4] = [
    ("clipboard", "Clipboard", ["Paste", "Cut", "Copy"]),
    ("font", "Font", ["Font colour", "Bold", "Italic"]),
    ("paragraph", "Paragraph", ["Align text", "Bullets", "Numbering"]),
    ("styles", "Styles", ["Style gallery", "Heading", "Title"]),
];

fn harness(width: f32) -> Harness<'static> {
    let mut state = RibbonState::default();
    let mut harness = Harness::builder()
        .with_size(Vec2::new(width, 300.0))
        .build_ui(move |ui| {
            install(ui.ctx(), &Theme::default());
            Ribbon::new("main")
                .tab(RibbonTab::new("home", "Home", |tab: &mut TabBuilder<'_>| {
                    for (key, label, [large, first, second]) in GROUPS {
                        tab.group(key, label, |group: &mut GroupBuilder<'_>| {
                            group.add(RibbonLarge::new(key, IconName::ClipboardPaste, large));
                            group.add(RibbonLarge::new(extra_key(key), IconName::Settings, "Options"));
                            group.add(RibbonLarge::new(more_key(key), IconName::Settings, "Layout"));
                            group.add(RibbonSmall::new(first_key(key), IconName::Cut, first));
                            group.add(RibbonSmall::new(second_key(key), IconName::Copy, second));
                        });
                    }
                }))
                .show(ui, &mut state);
        });
    harness.run();
    harness
}

fn extra_key(key: &str) -> &'static str {
    match key {
        "clipboard" => "clipboard_0",
        "font" => "font_0",
        "paragraph" => "paragraph_0",
        _ => "styles_0",
    }
}

fn more_key(key: &str) -> &'static str {
    match key {
        "clipboard" => "clipboard_3",
        "font" => "font_3",
        "paragraph" => "paragraph_3",
        _ => "styles_3",
    }
}

fn first_key(key: &str) -> &'static str {
    match key {
        "clipboard" => "clipboard_1",
        "font" => "font_1",
        "paragraph" => "paragraph_1",
        _ => "styles_1",
    }
}

fn second_key(key: &str) -> &'static str {
    match key {
        "clipboard" => "clipboard_2",
        "font" => "font_2",
        "paragraph" => "paragraph_2",
        _ => "styles_2",
    }
}

/// The step a group is drawn at, from its probe button: absent when collapsed or overflowed,
/// tall at `Large`, a bare icon (32 pt wide) at `Small`, icon and label at `Medium`.
fn step(harness: &Harness<'_>, probe: &str) -> Option<ScalingStep> {
    let node = harness.query_by_role_and_label(Role::Button, probe)?;
    let rect = node.rect();
    Some(if rect.height() > 40.0 {
        ScalingStep::Large
    } else if rect.width() <= 32.5 {
        ScalingStep::Small
    } else {
        ScalingStep::Medium
    })
}

/// The step of every group; `None` for a group in the `…` menu.
fn steps(harness: &Harness<'_>) -> Vec<Option<ScalingStep>> {
    GROUPS
        .iter()
        .map(|(_, label, [probe, ..])| {
            step(harness, probe).or_else(|| {
                harness
                    .query_by_role_and_label(Role::Button, label)
                    .map(|_| ScalingStep::Collapsed)
            })
        })
        .collect()
}

/// Every button the body draws lies inside the window: the plan's widths match what is painted.
fn assert_fits(harness: &Harness<'_>, width: f32) {
    for node in harness.query_all_by_role(Role::Button) {
        let right = node.rect().right();
        assert!(
            right <= width,
            "{:?} ends at {right}, past {width}",
            node.accesskit_node().label()
        );
    }
}

fn resize(harness: &mut Harness<'_>, width: f32) {
    harness.set_size(Vec2::new(width, 300.0));
    harness.run();
}

use ScalingStep::{Collapsed as C, Large as L, Small as S};

#[test]
fn every_planned_row_fits_its_window() {
    for width in [1400.0, 900.0, 600.0, 300.0, 120.0] {
        assert_fits(&harness(width), width);
    }
}

#[test]
fn wide_window_shows_every_group_large() {
    assert_eq!(steps(&harness(1400.0)), vec![Some(L); 4]);
}

#[test]
fn width_900_reduces_the_rightmost_group() {
    assert_eq!(steps(&harness(900.0)), vec![Some(L), Some(L), Some(L), Some(S)]);
}

#[test]
fn width_600_collapses_the_two_rightmost_groups() {
    assert_eq!(steps(&harness(600.0)), vec![Some(L), Some(L), Some(C), Some(C)]);
}

#[test]
fn width_300_collapses_all_but_the_first_group() {
    let harness = harness(300.0);
    assert_eq!(steps(&harness), vec![Some(S), Some(C), Some(C), Some(C)]);
}

#[test]
fn width_120_moves_groups_into_the_overflow_menu() {
    let harness = harness(120.0);
    let found = steps(&harness);
    assert!(found.contains(&None), "{found:?}");
    assert!(
        harness
            .query_by_role_and_label(Role::Button, "More groups")
            .is_some()
    );
}

#[test]
fn resizing_by_two_points_across_a_threshold_keeps_the_plan() {
    // Narrow 1 pt at a time from a wide window until the first group steps down.
    let mut harness = harness(1400.0);
    let wide = steps(&harness);
    let mut width = 1400.0;
    while steps(&harness) == wide && width > 300.0 {
        width -= 1.0;
        resize(&mut harness, width);
    }
    let reduced = steps(&harness);
    assert_ne!(reduced, wide, "no threshold above 300 pt");
    // Back and forth by 2 pt across the threshold: the 8 pt band holds the plan.
    for w in [width + 2.0, width, width + 2.0, width, width + 2.0] {
        resize(&mut harness, w);
        assert_eq!(steps(&harness), reduced, "at {w}");
    }
}

#[test]
fn a_collapsed_group_opens_its_controls_in_a_popup() {
    let mut harness = harness(300.0);
    assert!(harness.query_by_role_and_label(Role::Button, "Title").is_none());
    let centre = harness
        .get_by_role_and_label(Role::Button, "Styles")
        .rect()
        .center();
    harness.event(Event::PointerMoved(centre));
    for pressed in [true, false] {
        harness.event(Event::PointerButton {
            pos: centre,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        });
    }
    harness.run();
    assert!(harness.query_by_role_and_label(Role::Button, "Title").is_some());
}
