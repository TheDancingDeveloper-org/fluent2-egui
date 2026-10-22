//! A filled danger Badge is readable in dark mode (issue #124).
//!
//! The test reads the badge's painted fill and text colour from one frame and checks the WCAG
//! contrast between them against the 4.5:1 text minimum.

use egui::{Color32, Shape, Vec2};
use fluent2_egui::controls::{Badge, BadgeAppearance, BadgeColor};
use fluent2_egui::theme::{Theme, c, install};
use fluent2_tokens::FLUENT;

/// Run `build` for two frames and return the second frame's shapes, with every
/// `Shape::Vec` expanded.
fn shapes(theme: &Theme, build: impl Fn(&mut egui::Ui)) -> Vec<Shape> {
    let ctx = egui::Context::default();
    install(&ctx, theme);
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
        // egui panics if texture deltas are dropped unhandled; this test does not upload them.
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

/// WCAG relative luminance of an sRGB colour.
fn luminance(color: Color32) -> f64 {
    let linear = |channel: u8| {
        let s = f64::from(channel) / 255.0;
        if s <= 0.03928 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.r()) + 0.7152 * linear(color.g()) + 0.0722 * linear(color.b())
}

/// WCAG contrast ratio, from 1 to 21.
fn contrast(a: Color32, b: Color32) -> f64 {
    let (la, lb) = (luminance(a), luminance(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[test]
fn filled_danger_badge_text_meets_aa_on_its_fill_in_dark_mode() {
    let theme = Theme::dark(FLUENT);
    let fill = c(theme.color.status_danger_background_3);
    let painted = shapes(&theme, |ui| {
        ui.add(
            Badge::new("Failed")
                .appearance(BadgeAppearance::Filled)
                .color(BadgeColor::Danger),
        );
    });

    assert!(
        painted
            .iter()
            .any(|shape| matches!(shape, Shape::Rect(r) if r.fill == fill)),
        "the badge paints its danger fill {fill:?}",
    );
    let text = painted
        .iter()
        .find_map(|shape| match shape {
            Shape::Text(t) if t.galley.text() == "Failed" => Some(t.fallback_color),
            _ => None,
        })
        .unwrap_or_else(|| panic!("the badge paints its label \"Failed\""));

    let ratio = contrast(text, fill);
    assert!(
        ratio >= 4.5,
        "filled danger badge text {text:?} on {fill:?} is {ratio:.2}:1, below 4.5:1 (dark)",
    );
}
