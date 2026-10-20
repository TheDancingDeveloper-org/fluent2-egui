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

/// Fluent styling for a tooltip, attached to an egui [`Response`].
pub trait TooltipExt {
    /// Show `text` as a Fluent tooltip after egui's hover delay, and while keyboard focus is
    /// visible.
    ///
    /// Returns `self` so it chains. For the inverted palette, use
    /// [`FluentTooltip::inverted`] on the returned builder.
    fn fluent_tooltip(self, text: impl Into<String>) -> Self;
}
use crate::theme::{Theme, ThemeKind, c};

/// Maximum width of the tooltip text, in points.
const MAX_WIDTH: f32 = 240.0;
/// Same key `foundation::focus` uses for the sticky keyboard-focus flag.
const KEYBOARD_FOCUS_KEY: &str = "fluent2_keyboard_focus";

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

/// A Fluent tooltip attached to an egui [`Response`].
///
/// Created by [`TooltipExt::fluent_tooltip`]. Hover waits for egui's tooltip delay. Call
/// [`Self::inverted`] before dropping (or before [`Self::show`]) to use the static inverted
/// palette.
pub struct FluentTooltip {
    response: Response,
    text: String,
    tone: TooltipTone,
    shown: bool,
}

impl FluentTooltip {
    /// Use `neutral_background_static` with `neutral_foreground_static_inverted`.
    #[must_use]
    pub fn inverted(mut self) -> Self {
        self.tone = TooltipTone::Inverted;
        self
    }

    /// Attach the tooltip. This also runs on drop, so a plain
    /// `response.fluent_tooltip(...).inverted()` chain is enough.
    pub fn show(mut self) {
        self.paint();
    }

    fn paint(&mut self) {
        if self.shown {
            return;
        }
        self.shown = true;
        let text = self.text.clone();
        let tone = self.tone;
        let response = self.response.clone();
        // Screen readers read the tooltip as the widget's description, on focus and without
        // hover, as Fluent web does with aria-describedby (WI-1931). An icon-only button whose
        // label is the tooltip text gets no duplicate.
        response.ctx.accesskit_node_builder(response.id, |node| {
            if let Some(description) = tooltip_description(node.label(), &text) {
                node.set_description(description);
            }
        });
        let show_for_focus = keyboard_focus(&response);
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
            .show(move |ui| {
                paint_tooltip(ui, &text, tone);
            });
        }
    }
}

impl Drop for FluentTooltip {
    fn drop(&mut self) {
        self.paint();
    }
}

impl TooltipExt for Response {
    /// Show `text` as a Fluent tooltip after egui's hover delay, and while keyboard focus is
    /// visible.
    ///
    /// ```
    /// # use fluent2_egui::surfaces::TooltipExt;
    /// # fn demo(ui: &mut egui::Ui) {
    /// let _response = ui.button("Save").fluent_tooltip("Saves the document");
    /// # }
    /// ```
    fn fluent_tooltip(self, text: impl Into<String>) -> Self {
        let tooltip = FluentTooltip {
            response: self.clone(),
            text: text.into(),
            tone: TooltipTone::Normal,
            shown: false,
        };
        tooltip.show();
        self
    }
}

/// The part of a tooltip that adds to the widget's name: `None` when it only repeats the label,
/// and without a leading line that repeats it (ribbon tooltips are "Label\nShortcut").
fn tooltip_description(label: Option<&str>, text: &str) -> Option<String> {
    let text = text.trim();
    let rest = match label.map(str::trim) {
        Some(label) if !label.is_empty() => {
            if text == label {
                return None;
            }
            text.strip_prefix(label)
                .and_then(|rest| rest.strip_prefix('\n'))
                .unwrap_or(text)
        }
        _ => text,
    };
    let rest = rest.trim();
    (!rest.is_empty()).then(|| rest.to_owned())
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
        // `transparent_stroke` is clear except in high contrast, where the inverted fill is the
        // window colour and needs an edge.
        TooltipTone::Inverted => (
            c(theme.color.neutral_background_static),
            c(theme.color.transparent_stroke),
        ),
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

#[cfg(test)]
mod tests {
    use super::tooltip_description;

    #[test]
    fn description_skips_what_the_label_already_says() {
        assert_eq!(tooltip_description(Some("Save"), "Save"), None);
        assert_eq!(
            tooltip_description(Some("Paste"), "Paste\nCtrl+V"),
            Some("Ctrl+V".to_owned())
        );
        assert_eq!(
            tooltip_description(Some("Save"), "Saves the document"),
            Some("Saves the document".to_owned())
        );
        assert_eq!(tooltip_description(None, "Help"), Some("Help".to_owned()));
        assert_eq!(tooltip_description(Some("Help"), "  "), None);
    }
}
