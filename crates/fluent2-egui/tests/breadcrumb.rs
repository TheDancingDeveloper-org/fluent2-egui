//! Breadcrumb: rendering in each theme, item clicks, the current item, the landmark and the
//! overflow menu, through kittest (no wgpu).

use egui::Vec2;
use egui::accesskit::{AriaCurrent, Role};
use egui_kittest::Harness;
use egui_kittest::kittest::{NodeT, Queryable};
use fluent2_egui::controls::{Breadcrumb, BreadcrumbDivider, BreadcrumbItemStyle, BreadcrumbSize};
use fluent2_egui::{Theme, install};

const PATH: [&str; 5] = ["Home", "Documents", "Projects", "Reports", "2026"];

/// The last item the row reported, by index.
#[derive(Default)]
struct Picked(Option<usize>);

fn harness(width: f32, theme: fn() -> Theme) -> Harness<'static, Picked> {
    Harness::builder()
        .with_size(Vec2::new(width, 200.0))
        .build_ui_state(
            move |ui, picked: &mut Picked| {
                install(ui.ctx(), &theme());
                let trail = Breadcrumb::new("path", PATH).show(ui);
                if trail.clicked.is_some() {
                    picked.0 = trail.clicked;
                }
            },
            Picked::default(),
        )
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
        let mut harness = harness(600.0, theme);
        harness.run();
        harness.get_by_label("Home");
        harness.get_by_label("Breadcrumb");
    }
}

#[test]
fn landmark_is_a_navigation_region_named_breadcrumb() {
    let mut harness = harness(600.0, light);
    harness.run();
    let landmark = harness.get_by_label("Breadcrumb").accesskit_node();
    assert_eq!(landmark.role(), Role::Navigation);
}

#[test]
fn clicking_an_item_reports_its_index_and_the_current_item_is_not_a_button() {
    let mut harness = harness(600.0, light);
    harness.run();

    harness.get_by_label("Documents").click();
    harness.run();
    assert_eq!(harness.state().0, Some(1));

    // The last item is current: plain text, marked aria-current, with no button role.
    let current = harness.get_by_label("2026").accesskit_node();
    assert_eq!(current.aria_current(), Some(AriaCurrent::Page));
    assert_ne!(current.role(), Role::Button);
    assert_eq!(harness.state().0, Some(1), "nothing else was clicked");
}

#[test]
fn only_the_current_item_is_marked_current() {
    let mut harness = harness(600.0, light);
    harness.run();
    assert_eq!(harness.get_by_label("Home").accesskit_node().aria_current(), None);
}

#[test]
fn narrow_row_folds_the_middle_into_an_ellipsis_menu() {
    let mut harness = harness(230.0, light);
    harness.run();

    // The first and current items stay in the row; the middle is behind "…".
    harness.get_by_label("Home");
    harness.get_by_label("2026");
    assert!(
        harness.query_by_label("Documents").is_none(),
        "folded items are not in the row"
    );

    harness.get_by_label("…").click();
    harness.run();
    harness.get_by_label("Documents").click();
    harness.run();
    assert_eq!(harness.state().0, Some(1), "a folded item reports its index");
}

#[test]
fn every_option_renders_without_panic() {
    for size in [BreadcrumbSize::Small, BreadcrumbSize::Large] {
        for divider in [BreadcrumbDivider::Slash, BreadcrumbDivider::Chevron] {
            for style in [BreadcrumbItemStyle::Link, BreadcrumbItemStyle::Button] {
                let mut harness = Harness::builder()
                    .with_size(Vec2::new(400.0, 120.0))
                    .build_ui(move |ui| {
                        install(ui.ctx(), &light());
                        Breadcrumb::new("opts", PATH)
                            .size(size)
                            .divider(divider)
                            .item_style(style)
                            .current(2)
                            .show(ui);
                    });
                harness.run();
                harness.get_by_label("Projects");
            }
        }
    }
}
