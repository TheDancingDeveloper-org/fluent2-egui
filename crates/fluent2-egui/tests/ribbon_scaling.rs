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

/// Text measurement differs between platforms, so the exact step at a given width is not
/// pinned. These are the §5.2 invariants every platform must hold.
const WIDTHS: [f32; 4] = [1400.0, 900.0, 600.0, 300.0];

/// How far a group is reduced: Large 0 … Collapsed 3, overflowed 4.
fn depth(step: Option<ScalingStep>) -> u8 {
    match step {
        Some(L) => 0,
        Some(ScalingStep::Medium) => 1,
        Some(S) => 2,
        Some(C) => 3,
        None => 4,
    }
}

#[test]
fn groups_reduce_from_the_right() {
    // The rightmost group steps down first, so no group is reduced further than one to
    // its right (every group here allows every step).
    for width in WIDTHS {
        let found = steps(&harness(width));
        let depths: Vec<u8> = found.iter().copied().map(depth).collect();
        assert!(
            depths.windows(2).all(|pair| pair[0] <= pair[1]),
            "at {width}: {found:?}"
        );
    }
}

#[test]
fn narrower_windows_never_widen_a_group() {
    let plans: Vec<Vec<u8>> = WIDTHS
        .iter()
        .map(|width| steps(&harness(*width)).into_iter().map(depth).collect())
        .collect();
    for (pair, widths) in plans.windows(2).zip(WIDTHS.windows(2)) {
        let wider = &pair[0];
        let narrower = &pair[1];
        assert!(
            wider.iter().zip(narrower).all(|(a, b)| a <= b),
            "{} pt {wider:?} vs {} pt {narrower:?}",
            widths[0],
            widths[1]
        );
    }
}

#[test]
fn medium_widths_reduce_without_collapsing_everything() {
    // 900 and 600 pt are below the all-Large width of these four groups and far above the
    // all-collapsed width, on any platform's fonts.
    for width in [900.0, 600.0] {
        let found = steps(&harness(width));
        assert_ne!(found, vec![Some(L); 4], "at {width}");
        assert_eq!(found[0], Some(L), "at {width}: {found:?}");
    }
}

#[test]
fn width_300_collapses_the_rightmost_groups() {
    let found = steps(&harness(300.0));
    assert_eq!(found[3], Some(C), "{found:?}");
    assert!(!found.contains(&None), "{found:?}");
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
