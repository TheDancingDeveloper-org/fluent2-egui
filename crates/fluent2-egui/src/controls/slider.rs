//! Fluent Slider: a 4 px rail, a brand fill and a 20 px thumb with an inner brand dot.
//!
//! Arrow keys step by the configured step, or by 1% of the range when none is set.
//! Home and End jump to the ends. Dragging the thumb uses [`egui::Sense::drag`].

use egui::{Key, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo};

use crate::foundation::focus::paint_focus_ring;
use crate::theme::{self, Theme};

const RAIL: f32 = 4.0;
const THUMB: f32 = 20.0;
const DOT: f32 = 12.0;

/// A continuous or stepped slider.
///
/// ```
/// # use fluent2_egui::controls::Slider;
/// # fn demo(ui: &mut egui::Ui) {
/// let mut value = 40.0;
/// ui.add(Slider::new(&mut value, 0.0..=100.0).step(Some(5.0)));
/// # }
/// ```
pub struct Slider<'a> {
    value: &'a mut f64,
    min: f64,
    max: f64,
    step: Option<f64>,
    vertical: bool,
}

impl<'a> Slider<'a> {
    /// A horizontal slider over `range`. Non-finite bounds are dropped and an inverted
    /// range is swapped.
    pub fn new(value: &'a mut f64, range: std::ops::RangeInclusive<f64>) -> Self {
        let mut slider = Self {
            value,
            min: 0.0,
            max: 100.0,
            step: None,
            vertical: false,
        };
        let (start, end) = (*range.start(), *range.end());
        if start.is_finite() {
            slider.min = start;
        }
        if end.is_finite() {
            slider.max = end;
        }
        if slider.min > slider.max {
            std::mem::swap(&mut slider.min, &mut slider.max);
        }
        slider
    }

    /// Snap to multiples of `step` measured from the minimum. `None` and non-positive
    /// values leave the slider continuous.
    #[must_use]
    pub fn step(mut self, step: Option<f64>) -> Self {
        self.step = step.filter(|step| step.is_finite() && *step > 0.0);
        self
    }

    /// Draw the rail vertically.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }
}

impl Widget for Slider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let Slider {
            value,
            min,
            max,
            step,
            vertical,
        } = self;
        let current = snap(clamp(*value, min, max), min, max, step);
        *value = current;

        let length = if vertical {
            ui.available_height().clamp(80.0, 160.0)
        } else {
            ui.available_width().clamp(120.0, 240.0)
        };
        let size = if vertical {
            Vec2::new(THUMB, length)
        } else {
            Vec2::new(length, THUMB)
        };
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::drag());
        let enabled = ui.is_enabled();
        let span = (max - min).max(f64::EPSILON);

        if enabled && response.dragged() {
            let pointer = response.interact_pointer_pos().unwrap_or(rect.center());
            let t = fraction_at(pointer, rect, vertical);
            *value = snap(min + t * span, min, max, step);
            response.mark_changed();
        }

        if enabled && response.has_focus() {
            let increment = step.unwrap_or(span * 0.01);
            let (home, end, left, right, up, down) = ui.input(|input| {
                (
                    input.key_pressed(Key::Home),
                    input.key_pressed(Key::End),
                    input.key_pressed(Key::ArrowLeft),
                    input.key_pressed(Key::ArrowRight),
                    input.key_pressed(Key::ArrowUp),
                    input.key_pressed(Key::ArrowDown),
                )
            });
            let next = if home {
                Some(min)
            } else if end {
                Some(max)
            } else if (vertical && up) || (!vertical && right) {
                Some(current + increment)
            } else if (vertical && down) || (!vertical && left) {
                Some(current - increment)
            } else {
                None
            };
            if let Some(next) = next {
                *value = snap(clamp(next, min, max), min, max, step);
                response.mark_changed();
            }
        }

        paint(
            ui,
            &theme,
            rect,
            fraction(current, min, span),
            vertical,
            enabled,
            response.has_focus(),
        );
        response.widget_info(|| WidgetInfo::slider(enabled, current, "Slider"));
        response
    }
}

fn paint(ui: &Ui, theme: &Theme, rect: Rect, fraction: f32, vertical: bool, enabled: bool, focused: bool) {
    let color = &theme.color;
    let painter = ui.painter();
    let rail = if vertical {
        Rect::from_center_size(rect.center(), Vec2::new(RAIL, rect.height() - THUMB))
    } else {
        Rect::from_center_size(rect.center(), Vec2::new(rect.width() - THUMB, RAIL))
    };
    painter.rect_filled(rail, RAIL / 2.0, theme::c(color.neutral_stroke_accessible));
    let filled = if vertical {
        Rect::from_min_max(
            Pos2::new(rail.left(), rail.bottom() - rail.height() * fraction),
            rail.right_bottom(),
        )
    } else {
        Rect::from_min_max(
            rail.left_top(),
            Pos2::new(rail.left() + rail.width() * fraction, rail.bottom()),
        )
    };
    if fraction > 0.0 {
        painter.rect_filled(filled, RAIL / 2.0, theme::c(color.compound_brand_background));
    }
    let thumb_center = if vertical {
        Pos2::new(rect.center().x, rail.bottom() - rail.height() * fraction)
    } else {
        Pos2::new(rail.left() + rail.width() * fraction, rect.center().y)
    };
    let thumb = Rect::from_center_size(thumb_center, Vec2::splat(THUMB));
    painter.circle_filled(thumb.center(), THUMB / 2.0, theme::c(color.neutral_background_1));
    painter.circle_stroke(
        thumb.center(),
        THUMB / 2.0 - 0.5,
        Stroke::new(1.0, theme::c(color.neutral_stroke_1)),
    );
    painter.circle_filled(
        thumb.center(),
        DOT / 2.0,
        theme::c(color.compound_brand_background),
    );
    if focused {
        paint_focus_ring(ui, thumb, egui::CornerRadius::same((THUMB / 2.0) as u8));
    }
    let _ = enabled;
}

fn fraction_at(pointer: Pos2, rect: Rect, vertical: bool) -> f64 {
    // The rail is inset by half a thumb on both ends, matching `paint`.
    let (along, length) = if vertical {
        (rect.bottom() - THUMB / 2.0 - pointer.y, rect.height() - THUMB)
    } else {
        (pointer.x - rect.left() - THUMB / 2.0, rect.width() - THUMB)
    };
    if length <= f32::EPSILON {
        return 0.0;
    }
    (along as f64 / length as f64).clamp(0.0, 1.0)
}

fn fraction(value: f64, min: f64, span: f64) -> f32 {
    ((value - min) / span).clamp(0.0, 1.0) as f32
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if !value.is_finite() {
        return min;
    }
    value.clamp(min, max)
}

fn snap(value: f64, min: f64, max: f64, step: Option<f64>) -> f64 {
    let Some(step) = step else {
        return clamp(value, min, max);
    };
    let steps = ((value - min) / step).round();
    clamp(min + steps * step, min, max)
}
