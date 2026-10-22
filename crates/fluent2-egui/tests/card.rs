//! Card paints its elevation shadow behind its fill, in light and dark (issue #124).
//!
//! These tests read one frame's shapes. The shadow is a blurred rect shape; the fill is a
//! plain rect shape with the face colour. The shadow must come first in paint order.

use egui::{Shape, Vec2};
use fluent2_egui::surfaces::{Card, CardAppearance, CardHeader};
use fluent2_egui::theme::{Theme, c, install};
use fluent2_tokens::FLUENT;

/// Run `build` for two frames and return the second frame's shapes, with every
/// `Shape::Vec` expanded so the list is in paint order.
fn shapes(theme: &Theme, build: impl Fn(&mut egui::Ui)) -> Vec<Shape> {
    let ctx = egui::Context::default();
    install(&ctx, theme);
    let mut last = Vec::new();
    for _ in 0..2 {
        let raw = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(600.0, 400.0),
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

fn resting_card(ui: &mut egui::Ui) {
    Card::new()
        .appearance(CardAppearance::Filled)
        .id_salt("card")
        .show(ui, |ui| {
            CardHeader::new("Card title")
                .description("Card description")
                .show(ui);
        });
}

#[test]
fn resting_filled_card_paints_its_shadow_before_its_fill_in_both_themes() {
    for theme in [Theme::light(FLUENT), Theme::dark(FLUENT)] {
        let painted = shapes(&theme, resting_card);
        let fill = c(theme.color.neutral_background_1);

        let fill_at = painted
            .iter()
            .position(|shape| matches!(shape, Shape::Rect(r) if r.fill == fill && r.blur_width == 0.0))
            .unwrap_or_else(|| panic!("no card fill of {fill:?} in {:?}", theme.kind));
        let shadow_at: Vec<usize> = painted
            .iter()
            .enumerate()
            .filter_map(|(i, shape)| match shape {
                Shape::Rect(r) if r.blur_width > 0.0 => Some(i),
                _ => None,
            })
            .collect();

        assert!(
            !shadow_at.is_empty(),
            "a filled card at rest has a shadow in {:?}",
            theme.kind
        );
        assert!(
            shadow_at.iter().all(|&i| i < fill_at),
            "every shadow layer paints before the fill (fill at {fill_at}, shadows at {shadow_at:?}) in {:?}",
            theme.kind,
        );
    }
}
