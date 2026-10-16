//! Fluent elevation shadows.

use egui::{Color32, CornerRadius, Painter, Shadow, epaint::Rect};
use fluent2_tokens::ShadowLayer;

use crate::theme::Theme;

/// Fluent elevation steps. Each maps to `theme.shadow.shadow_N`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Elevation {
    /// `shadow2`.
    E2,
    /// `shadow4`.
    E4,
    /// `shadow8`.
    E8,
    /// `shadow16`.
    E16,
    /// `shadow28`.
    E28,
    /// `shadow64`.
    E64,
}

/// Paint every layer of `theme.shadow` for this elevation, in token order.
///
/// `egui::Shadow` stores offset, blur and spread as small integers, so a Fluent layer is
/// rounded and clamped to that range. A non-finite layer is skipped.
pub fn paint_shadow(
    painter: &Painter,
    rect: Rect,
    radius: CornerRadius,
    elevation: Elevation,
    theme: &Theme,
) {
    for layer in layers(theme, elevation) {
        let Some(shadow) = shadow_from_layer(layer) else {
            continue;
        };
        painter.add(shadow.as_shape(rect, radius));
    }
}

fn layers(theme: &Theme, elevation: Elevation) -> &'static [ShadowLayer] {
    match elevation {
        Elevation::E2 => theme.shadow.shadow_2,
        Elevation::E4 => theme.shadow.shadow_4,
        Elevation::E8 => theme.shadow.shadow_8,
        Elevation::E16 => theme.shadow.shadow_16,
        Elevation::E28 => theme.shadow.shadow_28,
        Elevation::E64 => theme.shadow.shadow_64,
    }
}

fn shadow_from_layer(layer: &ShadowLayer) -> Option<Shadow> {
    if !layer.x.is_finite() || !layer.y.is_finite() || !layer.blur.is_finite() || !layer.spread.is_finite() {
        return None;
    }
    Some(Shadow {
        offset: [quantize_i8(layer.x), quantize_i8(layer.y)],
        blur: quantize_u8(layer.blur),
        spread: quantize_u8(layer.spread),
        color: Color32::from_rgba_unmultiplied(layer.color.r, layer.color.g, layer.color.b, layer.color.a),
    })
}

fn quantize_i8(value: f32) -> i8 {
    value.round().clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8
}

fn quantize_u8(value: f32) -> u8 {
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}
