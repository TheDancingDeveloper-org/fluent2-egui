//! High contrast: content on a highlight fill takes the fill's foreground (WI-1845).
//!
//! In high contrast the hover, pressed and selected fills are the highlight colour, and the
//! resting foreground on it is about 1.46:1. These tests read one frame's shapes, or the colour
//! a caller's draw closure sees.

use std::cell::Cell;

use egui::{Color32, Shape, Vec2};
use fluent2_egui::office::ribbon::Gallery;
use fluent2_egui::surfaces::{Card, CardHeader, TabAppearance, TabList};
use fluent2_egui::theme::{Theme, c, install};

/// Run `build` for two frames and return the second frame's shapes.
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
        last = output.shapes.into_iter().map(|clipped| clipped.shape).collect();
        // egui panics if texture deltas are dropped unhandled; this test does not upload them.
        output.textures_delta.clear();
    }
    last
}

/// The colour the text shape reading `text` was painted with.
fn text_color(shapes: &[Shape], text: &str) -> Option<Color32> {
    shapes.iter().find_map(|shape| match shape {
        Shape::Text(t) if t.galley.text() == text => Some(t.fallback_color),
        _ => None,
    })
}

fn card(selected: bool) -> impl Fn(&mut egui::Ui) {
    move |ui| {
        let mut on = selected;
        Card::new().id_salt("card").selectable(&mut on).show(ui, |ui| {
            CardHeader::new("Card title")
                .description("Card description")
                .show(ui);
        });
    }
}

#[test]
fn selected_card_header_takes_the_selected_foreground_in_high_contrast() {
    let hc = Theme::high_contrast();
    let painted = shapes(&hc, card(true));
    assert_eq!(
        text_color(&painted, "Card title"),
        Some(c(hc.color.neutral_foreground_1_selected)),
        "title"
    );
    assert_eq!(
        text_color(&painted, "Card description"),
        Some(c(hc.color.neutral_foreground_1_selected)),
        "description"
    );
}

#[test]
fn resting_card_header_keeps_its_own_colours() {
    for theme in [Theme::default(), Theme::high_contrast()] {
        let painted = shapes(&theme, card(false));
        assert_eq!(
            text_color(&painted, "Card title"),
            Some(c(theme.color.neutral_foreground_1))
        );
        assert_eq!(
            text_color(&painted, "Card description"),
            Some(c(theme.color.neutral_foreground_2))
        );
    }
}

#[test]
fn gallery_draw_closure_sees_the_selected_foreground_on_the_selected_tile() {
    let hc = Theme::high_contrast();
    let selected_ink = Cell::new(None);
    let resting_ink = Cell::new(None);
    shapes(&hc, |ui| {
        let mut selected = Some(1);
        Gallery::new("styles", "Styles", 4, &mut selected)
            .columns(4)
            .show(ui, |ui, index, _rect, _state| {
                let ink = ui.visuals().text_color();
                if index == 1 {
                    selected_ink.set(Some(ink));
                } else if index == 2 {
                    resting_ink.set(Some(ink));
                }
            });
    });
    assert_eq!(
        selected_ink.get(),
        Some(c(hc.color.neutral_foreground_1_selected))
    );
    assert_ne!(
        resting_ink.get(),
        Some(c(hc.color.neutral_foreground_1_selected)),
        "a resting tile keeps the resting text colour"
    );
}

fn has_fill(shapes: &[Shape], color: Color32) -> bool {
    shapes
        .iter()
        .any(|shape| matches!(shape, Shape::Rect(rect) if rect.fill == color))
}

#[test]
fn subtle_tab_indicator_stays_visible_on_the_highlight_fill() {
    let hc = Theme::high_contrast();
    let painted = shapes(&hc, |ui| {
        let mut selected = 0;
        TabList::new("tabs", &mut selected)
            .appearance(TabAppearance::Subtle)
            .tab("One")
            .tab("Two")
            .show(ui);
    });
    assert!(
        has_fill(&painted, c(hc.color.neutral_foreground_1_selected)),
        "the indicator takes the selected foreground on the highlight fill"
    );
}

#[test]
fn transparent_tab_indicator_keeps_the_brand_stroke() {
    let hc = Theme::high_contrast();
    let painted = shapes(&hc, |ui| {
        let mut selected = 0;
        TabList::new("tabs", &mut selected).tab("One").tab("Two").show(ui);
    });
    assert!(has_fill(&painted, c(hc.color.compound_brand_stroke)));
}
