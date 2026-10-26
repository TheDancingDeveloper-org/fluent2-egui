//! Slider: Home/End, drag, and step snapping.

use std::cell::Cell;

use proptest::strategy::Strategy;

use egui::{Shape, Vec2};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable;
use fluent2_egui::controls::Slider;
use fluent2_egui::theme::{self, SliderMetrics, Theme, c};

fn install(ui: &egui::Ui) {
    theme::install(ui.ctx(), &Theme::default());
}

#[test]
fn home_and_end_jump_to_the_range() {
    let value = Cell::new(40.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(Slider::new(&mut current, 0.0..=100.0));
        response.request_focus();
        value.set(current);
    });
    harness.run();
    harness.key_press(egui::Key::Home);
    harness.run();
    assert_eq!(value.get(), 0.0);
    harness.key_press(egui::Key::End);
    harness.run();
    assert_eq!(value.get(), 100.0);
}

#[test]
fn drag_changes_the_value() {
    let value = Cell::new(0.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        ui.add(Slider::new(&mut current, 0.0..=100.0));
        value.set(current);
    });
    harness.run();
    let node = harness.get_by_label("Slider");
    let rect = node.rect();
    harness.event(egui::Event::PointerMoved(rect.center()));
    harness.event(egui::Event::PointerButton {
        pos: rect.center(),
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: egui::Modifiers::default(),
    });
    harness.run();
    assert!(
        value.get() > 10.0 && value.get() < 90.0,
        "drag to the middle moves the value, got {}",
        value.get()
    );
}

#[test]
fn step_snaps() {
    let value = Cell::new(0.0);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(Slider::new(&mut current, 0.0..=100.0).step(Some(10.0)));
        response.request_focus();
        value.set(current);
    });
    harness.run();
    harness.key_press(egui::Key::ArrowRight);
    harness.run();
    assert_eq!(value.get(), 10.0);
}

fn finite_slider(
    start: f64,
    range: std::ops::RangeInclusive<f64>,
    step: Option<f64>,
    press: Option<egui::Key>,
) -> f64 {
    let value = Cell::new(start);
    let mut harness = Harness::new_ui(|ui| {
        install(ui);
        let mut current = value.get();
        let response = ui.add(Slider::new(&mut current, range.clone()).step(step));
        if press.is_some() {
            response.request_focus();
        }
        value.set(current);
    });
    harness.run();
    if let Some(key) = press {
        harness.key_press(key);
        harness.run();
    }
    value.get()
}

#[test]
fn inverted_range_does_not_panic() {
    let value = finite_slider(f64::NAN, 80.0..=10.0, None, None);
    assert!(value.is_finite());
    assert!((10.0..=80.0).contains(&value));
}

#[test]
fn nan_bound_does_not_panic() {
    let value = finite_slider(f64::NAN, 8.0..=f64::NAN, None, None);
    assert!(value.is_finite());
}

#[test]
fn extreme_finite_range_stays_finite() {
    let value = finite_slider(-f64::MAX, -f64::MAX..=f64::MAX, None, None);
    assert!(value.is_finite(), "value {value}");
    assert_eq!(value, -f64::MAX);
}

#[test]
fn degenerate_range_stays_finite() {
    let value = finite_slider(4.0, 7.0..=7.0, None, None);
    assert_eq!(value, 7.0);
}

#[test]
fn tiny_step_stays_finite() {
    let value = finite_slider(
        0.0,
        0.0..=1.0,
        Some(f64::MIN_POSITIVE),
        Some(egui::Key::ArrowRight),
    );
    assert!(value.is_finite(), "value {value}");
    assert!((0.0..=1.0).contains(&value));
}

#[test]
fn renders_in_dark_and_high_contrast() {
    for theme in [Theme::dark(fluent2_tokens::FLUENT), Theme::high_contrast()] {
        let mut harness = Harness::new_ui(|ui| {
            theme::install(ui.ctx(), &theme);
            let mut value = 20.0;
            ui.add(Slider::new(&mut value, 0.0..=100.0));
        });
        harness.run();
    }
}

/// Run `build` for two frames and return the second frame's shapes, with every
/// `Shape::Vec` expanded.
fn shapes(theme: &Theme, build: impl Fn(&mut egui::Ui)) -> Vec<Shape> {
    let ctx = egui::Context::default();
    theme::install(&ctx, theme);
    let mut last = Vec::new();
    for _ in 0..2 {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(400.0, 200.0),
            )),
            ..Default::default()
        };
        let mut output = ctx.run_ui(raw, |ui| build(ui));
        last = flatten(output.shapes.into_iter().map(|clipped| clipped.shape));
        output.textures_delta.clear();
    }
    last
}

fn flatten(shapes: impl Iterator<Item = Shape>) -> Vec<Shape> {
    shapes
        .flat_map(|shape| match shape {
            Shape::Vec(inner) => flatten(inner.into_iter()),
            other => vec![other],
        })
        .collect()
}

#[test]
fn custom_slider_tokens_change_painted_geometry() {
    let theme = Theme {
        slider: SliderMetrics {
            rail: 10.0,
            thumb: 32.0,
            dot: 8.0,
        },
        ..Theme::default()
    };
    let painted = shapes(&theme, |ui| {
        let mut value = 50.0;
        ui.add(Slider::new(&mut value, 0.0..=100.0));
    });
    let rails: Vec<_> = painted
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if (rect.rect.height() - 10.0).abs() < 0.01 => Some(rect.rect),
            _ => None,
        })
        .collect();
    assert!(
        !rails.is_empty(),
        "custom rail thickness 10 must appear in painted rects"
    );
    let thumbs: Vec<_> = painted
        .iter()
        .filter_map(|shape| match shape {
            Shape::Circle(circle) if (circle.radius - 16.0).abs() < 0.01 => Some(circle.radius),
            _ => None,
        })
        .collect();
    assert!(
        !thumbs.is_empty(),
        "custom thumb diameter 32 must paint a 16 radius circle"
    );
    assert!(
        painted.iter().any(|shape| matches!(
            shape,
            Shape::Circle(circle) if (circle.radius - 4.0).abs() < 0.01
        )),
        "custom inner dot diameter 8 must paint a 4 radius circle"
    );
}

#[test]
fn disabled_slider_uses_disabled_colours() {
    let theme = Theme::default();
    let rgb = |color: egui::Color32| (color.r(), color.g(), color.b());
    let paint = |enabled: bool| {
        shapes(&theme, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                let mut value = 50.0;
                ui.add(Slider::new(&mut value, 0.0..=100.0));
            });
        })
    };
    let rects = |shapes: &[Shape]| {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Rect(rect) => Some(rgb(rect.fill)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let circles = |shapes: &[Shape]| {
        shapes
            .iter()
            .filter_map(|shape| match shape {
                Shape::Circle(circle) => Some(rgb(circle.fill)),
                _ => None,
            })
            .collect::<Vec<_>>()
    };
    let on = paint(true);
    let off = paint(false);
    // egui fades a disabled `Ui` after the widget chooses colours, so the pixels are not the
    // raw alias. The outcome is that rail, fill and thumb no longer match the enabled frame.
    assert_ne!(
        rects(&on),
        rects(&off),
        "disabled rail and fill must not use the enabled colours"
    );
    assert_ne!(
        circles(&on),
        circles(&off),
        "disabled thumb and dot must not use the enabled colours"
    );
    let enabled_rail = rgb(c(theme.color.neutral_stroke_accessible));
    let enabled_brand = rgb(c(theme.color.compound_brand_background));
    let enabled_thumb = rgb(c(theme.color.neutral_background_1));
    assert!(
        !rects(&off).contains(&enabled_rail) && !rects(&off).contains(&enabled_brand),
        "disabled rail and fill are not the resting stroke or brand, got {:?}",
        rects(&off)
    );
    assert!(
        !circles(&off).contains(&enabled_thumb) && !circles(&off).contains(&enabled_brand),
        "disabled thumb and dot are not the resting background or brand, got {:?}",
        circles(&off)
    );
}

/// The theme-render test must check painted outcomes, not only that the harness runs.
#[test]
fn theme_render_paints_token_colours_and_metrics() {
    let theme = Theme::default();
    let rgb = |color: egui::Color32| (color.r(), color.g(), color.b());
    let painted = shapes(&theme, |ui| {
        let mut value = 40.0;
        ui.add(Slider::new(&mut value, 0.0..=100.0));
    });
    let rail_h = theme.slider.rail;
    let thumb_r = theme.slider.thumb / 2.0;
    let dot_r = theme.slider.dot / 2.0;
    let rails: Vec<_> = painted
        .iter()
        .filter_map(|shape| match shape {
            Shape::Rect(rect) if (rect.rect.height() - rail_h).abs() < 0.01 => Some(rect.clone()),
            _ => None,
        })
        .collect();
    assert!(
        !rails.is_empty(),
        "rail thickness must come from SliderMetrics::rail ({rail_h})"
    );
    let rail_colour = rgb(c(theme.color.neutral_stroke_accessible));
    let brand = rgb(c(theme.color.compound_brand_background));
    assert!(
        rails.iter().any(|rect| rgb(rect.fill) == rail_colour),
        "resting rail fill must be neutral_stroke_accessible, got {rails:?}"
    );
    assert!(
        rails.iter().any(|rect| rgb(rect.fill) == brand),
        "brand fill must be compound_brand_background"
    );
    assert!(
        painted.iter().any(|shape| matches!(
            shape,
            Shape::Circle(circle) if (circle.radius - thumb_r).abs() < 0.01
                && rgb(circle.fill) == rgb(c(theme.color.neutral_background_1))
        )),
        "thumb radius and fill must come from the theme thumb token and neutral_background_1"
    );
    assert!(
        painted.iter().any(|shape| matches!(
            shape,
            Shape::Circle(circle) if (circle.radius - dot_r).abs() < 0.01
                && rgb(circle.fill) == brand
        )),
        "inner dot radius and fill must come from the theme dot token and brand"
    );
}

proptest::proptest! {
    #![proptest_config(proptest::test_runner::Config {
        cases: 64,
        failure_persistence: None,
        ..proptest::test_runner::Config::default()
    })]

    /// Slider fraction and the value it stores stay finite for arbitrary finite ranges.
    #[test]
    fn slider_fraction_and_value_stay_finite(
        value in proptest::num::f64::ANY.prop_filter("finite", |v| v.is_finite()),
        min in proptest::num::f64::ANY.prop_filter("finite", |v| v.is_finite()),
        max in proptest::num::f64::ANY.prop_filter("finite", |v| v.is_finite()),
    ) {
        let (lo, hi) = if min <= max { (min, max) } else { (max, min) };
        let stored = finite_slider(value, lo..=hi, None, None);
        proptest::prop_assert!(stored.is_finite(), "stored {stored}");
        if lo == hi {
            proptest::prop_assert_eq!(stored, lo);
        } else {
            proptest::prop_assert!((lo..=hi).contains(&stored), "{stored} not in {lo}..={hi}");
        }
    }
}
