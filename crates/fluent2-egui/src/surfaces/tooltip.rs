//! Fluent tooltip on top of egui's hover timing.
//!
//! Hover uses [`Response::on_hover_ui`], which goes through [`egui::Tooltip`], so the delay and
//! the "a tooltip is already showing" shortcut stay with egui. Keyboard focus is extra: egui
//! only shows tooltips on hover, so when [`crate::foundation::focus::focus_visible`] is true the
//! same content is shown through [`egui::Tooltip::always_open`], anchored on the widget (which
//! places it below, flipping when there is no room).
//!
//! Metrics follow the WI-1551 description (Fluent UI React tooltip, the revision pinned there).
//! Colours, radii and the shadow come from the installed [`crate::Theme`].

use egui::{Color32, CornerRadius, Frame, Id, Response, Stroke, Ui};

use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::foundation::text::{self, TypeRamp};
use crate::theme::{Theme, ThemeKind, c};

/// Maximum width of the tooltip text, in points.
const MAX_WIDTH: f32 = 240.0;
/// Same key `foundation::focus` uses for the sticky keyboard-focus flag.
const KEYBOARD_FOCUS_KEY: &str = "fluent2_keyboard_focus";

/// Fluent styling for a tooltip, attached to an egui [`Response`].
///
/// ```
/// # use fluent2_egui::surfaces::TooltipExt;
/// # fn demo(ui: &mut egui::Ui) {
/// let _response = ui.button("Save").fluent_tooltip("Saves the document");
/// # }
/// ```
pub trait TooltipExt {
    /// Show `text` as a Fluent tooltip after egui's hover delay, and while keyboard focus is
    /// visible.
    ///
    /// Returns `self` so it chains. For the inverted palette, use [`fluent_tooltip`].
    fn fluent_tooltip(self, text: impl Into<String>) -> Self;
}

/// How a tooltip is coloured.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TooltipTone {
    /// `neutral_background_1` with `neutral_foreground_1`. The stroke is transparent except in
    /// high contrast, where it is `neutral_stroke_1`.
    #[default]
    Normal,
    /// `neutral_background_static` with `neutral_foreground_static_inverted`.
    Inverted,
}

/// Show `text` as a Fluent tooltip on `response`.
///
/// Hover waits for egui's tooltip delay. Keyboard focus (the sticky flag written by
/// [`crate::foundation::focus::focus_visible`], or a key event this frame) shows it at once,
/// below the widget.
pub fn fluent_tooltip(response: &Response, text: impl Into<String>, tone: TooltipTone) {
    let text = text.into();
    let show_for_focus = keyboard_focus(response);
    let hover_text = text.clone();
    response.clone().on_hover_ui(move |ui| {
        paint_tooltip(ui, &hover_text, tone);
    });
    if show_for_focus {
        let _ = egui::Tooltip::always_open(
            response.ctx.clone(),
            response.layer_id,
            response.id.with("fluent_focus_tooltip"),
            response.rect,
        )
        .show(|ui| {
            paint_tooltip(ui, &text, tone);
        });
    }
}

impl TooltipExt for Response {
    fn fluent_tooltip(self, text: impl Into<String>) -> Self {
        fluent_tooltip(&self, text, TooltipTone::Normal);
        self
    }
}

fn keyboard_focus(response: &Response) -> bool {
    let key_pressed = response.ctx.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Key { .. }))
    });
    let pointer_pressed = response.ctx.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::PointerButton { pressed: true, .. }))
    });
    let id = keyboard_focus_id();
    if pointer_pressed {
        response.ctx.data_mut(|data| data.insert_temp(id, false));
    } else if key_pressed {
        response.ctx.data_mut(|data| data.insert_temp(id, true));
    }
    let keyboard = response
        .ctx
        .data(|data| data.get_temp::<bool>(id).unwrap_or(false));
    let focused = response.ctx.memory(|memory| memory.focused()) == Some(response.id);
    response.has_focus() && focused && keyboard
}

fn keyboard_focus_id() -> Id {
    Id::NULL.with(KEYBOARD_FOCUS_KEY)
}

fn paint_tooltip(ui: &mut Ui, text: &str, tone: TooltipTone) {
    tooltip_frame(ui, tone).show(ui, |ui| {
        ui.set_max_width(MAX_WIDTH);
        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
        let theme = Theme::get(ui.ctx());
        let color = match tone {
            TooltipTone::Normal => c(theme.color.neutral_foreground_1),
            TooltipTone::Inverted => c(theme.color.neutral_foreground_static_inverted),
        };
        ui.label(text::rich(text, TypeRamp::Body1).color(color));
    });
}

fn tooltip_frame(ui: &Ui, tone: TooltipTone) -> Frame {
    let theme = Theme::get(ui.ctx());
    let (fill, stroke_color) = match tone {
        TooltipTone::Normal => {
            let stroke = if theme.kind == ThemeKind::HighContrast {
                c(theme.color.neutral_stroke_1)
            } else {
                Color32::TRANSPARENT
            };
            (c(theme.color.neutral_background_1), stroke)
        }
        TooltipTone::Inverted => (c(theme.color.neutral_background_static), Color32::TRANSPARENT),
    };
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(stroke_width::THIN, stroke_color))
        .corner_radius(medium_radius())
        .shadow(first_shadow(&theme))
        .inner_margin(egui::Margin::symmetric(
            spacing_horizontal::M_NUDGE as i8,
            spacing_vertical::XS as i8,
        ))
}

fn medium_radius() -> CornerRadius {
    CornerRadius::same(border_radius::MEDIUM.round().clamp(0.0, 255.0) as u8)
}

fn first_shadow(theme: &Theme) -> egui::Shadow {
    theme
        .shadow
        .shadow_16
        .iter()
        .find_map(|layer| {
            if !layer.x.is_finite()
                || !layer.y.is_finite()
                || !layer.blur.is_finite()
                || !layer.spread.is_finite()
            {
                return None;
            }
            Some(egui::Shadow {
                offset: [
                    layer.x.round().clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8,
                    layer.y.round().clamp(f32::from(i8::MIN), f32::from(i8::MAX)) as i8,
                ],
                blur: layer.blur.round().clamp(0.0, f32::from(u8::MAX)) as u8,
                spread: layer.spread.round().clamp(0.0, f32::from(u8::MAX)) as u8,
                color: Color32::from_rgba_unmultiplied(
                    layer.color.r,
                    layer.color.g,
                    layer.color.b,
                    layer.color.a,
                ),
            })
        })
        .unwrap_or(egui::Shadow::NONE)
}
