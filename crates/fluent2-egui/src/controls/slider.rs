//! Fluent Slider: a 4 px rail, a brand fill and a 20 px thumb with an inner brand dot.
//!
//! Arrow keys step by the configured step, or by 1% of the range when none is set.
//! Home and End jump to the ends. Dragging the thumb uses [`egui::Sense::drag`].

use egui::{Key, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo};

use crate::foundation::focus::paint_focus_ring;
use crate::theme::{self, Theme};

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
    label: Option<String>,
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
            label: None,
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

    /// Accessible name, read by screen readers. Not drawn. Inside a [`super::Field`] the
    /// Field's label is the default; otherwise it is "Slider".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
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
            label,
        } = self;
        let label = label
            .or_else(|| super::field::enclosing_field_label(ui.ctx()))
            .unwrap_or_else(|| "Slider".to_owned());
        let metrics = theme.slider;
        let length = if vertical {
            ui.available_height().clamp(80.0, 160.0)
        } else {
            ui.available_width().clamp(120.0, 240.0)
        };
        // Density grows the cross axis only (44 in Touch); the thumb diameter stays a
        // component token and is not scaled.
        let cross = crate::theme::density_of(ui).height(metrics.thumb);
        let size = if vertical {
            Vec2::new(cross, length)
        } else {
            Vec2::new(length, cross)
        };
        let (rect, mut response) = ui.allocate_exact_size(size, Sense::drag());
        let enabled = ui.is_enabled();
        let span = (max - min).max(f64::EPSILON);

        // Snap the incoming value, then apply this frame's drag or keys before paint
        // and AccessKit see it. Reading the value before input paints and reports the
        // previous frame's thumb (issue #143).
        let before = *value;
        let mut current = snap(clamp(*value, min, max), min, max, step);

        if enabled && response.dragged() {
            let pointer = response.interact_pointer_pos().unwrap_or(rect.center());
            let t = fraction_at(pointer, rect, vertical, metrics.thumb);
            current = snap(value_at(t, min, max), min, max, step);
        } else if enabled && response.has_focus() {
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
                current = snap(clamp(next, min, max), min, max, step);
            }
        }

        *value = current;
        // Changed only when the value the caller holds actually moved, including a snap
        // of an out-of-range or off-step value they wrote in. A drag or key that lands
        // on the same value is not a change.
        if current != before {
            response.mark_changed();
        }

        paint(
            ui,
            &theme,
            rect,
            fraction(current, min, max),
            vertical,
            PaintState {
                enabled,
                hovered: response.hovered(),
                pressed: response.is_pointer_button_down_on(),
                focused: response.has_focus(),
            },
        );
        response.widget_info(|| WidgetInfo::slider(enabled, current, label.clone()));
        // The range, so screen readers can say where the value sits: without it Orca read a
        // 100 on a 10..=400 zoom slider as "0 percent" (WI-1586).
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_min_numeric_value(min);
            node.set_max_numeric_value(max);
            if let Some(step) = step.filter(|step| step.is_finite() && *step > 0.0) {
                node.set_numeric_value_step(step);
            }
        });
        response
    }
}

struct PaintState {
    enabled: bool,
    hovered: bool,
    pressed: bool,
    focused: bool,
}

fn paint(ui: &Ui, theme: &Theme, rect: Rect, fraction: f32, vertical: bool, state: PaintState) {
    let PaintState {
        enabled,
        hovered,
        pressed,
        focused,
    } = state;
    let color = &theme.color;
    let painter = ui.painter();
    let metrics = theme.slider;
    let rail_size = metrics.rail;
    let thumb_size = metrics.thumb;
    let dot_size = metrics.dot;
    let rail = if vertical {
        Rect::from_center_size(rect.center(), Vec2::new(rail_size, rect.height() - thumb_size))
    } else {
        Rect::from_center_size(rect.center(), Vec2::new(rect.width() - thumb_size, rail_size))
    };
    // Resting, hover and pressed are Fluent alias colours, not slider-specific tokens.
    // Disabled wins over interaction, matching Checkbox.
    let rail_fill = if !enabled {
        color.neutral_stroke_disabled
    } else if pressed {
        color.neutral_stroke_accessible_pressed
    } else if hovered {
        color.neutral_stroke_accessible_hover
    } else {
        color.neutral_stroke_accessible
    };
    painter.rect_filled(rail, rail_size / 2.0, theme::c(rail_fill));
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
    let brand = if !enabled {
        color.neutral_foreground_disabled
    } else if pressed {
        color.compound_brand_background_pressed
    } else if hovered {
        color.compound_brand_background_hover
    } else {
        color.compound_brand_background
    };
    if fraction > 0.0 {
        painter.rect_filled(filled, rail_size / 2.0, theme::c(brand));
    }
    let thumb_center = if vertical {
        Pos2::new(rect.center().x, rail.bottom() - rail.height() * fraction)
    } else {
        Pos2::new(rail.left() + rail.width() * fraction, rect.center().y)
    };
    let thumb = Rect::from_center_size(thumb_center, Vec2::splat(thumb_size));
    let thumb_fill = if enabled {
        color.neutral_background_1
    } else {
        color.neutral_background_disabled
    };
    let thumb_stroke = if !enabled {
        color.neutral_stroke_disabled
    } else if pressed {
        color.neutral_stroke_1_pressed
    } else if hovered {
        color.neutral_stroke_1_hover
    } else {
        color.neutral_stroke_1
    };
    painter.circle_filled(thumb.center(), thumb_size / 2.0, theme::c(thumb_fill));
    painter.circle_stroke(
        thumb.center(),
        thumb_size / 2.0 - 0.5,
        Stroke::new(1.0, theme::c(thumb_stroke)),
    );
    painter.circle_filled(thumb.center(), dot_size / 2.0, theme::c(brand));
    if focused {
        paint_focus_ring(ui, thumb, egui::CornerRadius::same((thumb_size / 2.0) as u8));
    }
}

fn fraction_at(pointer: Pos2, rect: Rect, vertical: bool, thumb: f32) -> f64 {
    // The rail is inset by half a thumb on both ends, matching `paint`.
    let (along, length) = if vertical {
        (rect.bottom() - thumb / 2.0 - pointer.y, rect.height() - thumb)
    } else {
        (pointer.x - rect.left() - thumb / 2.0, rect.width() - thumb)
    };
    if length <= f32::EPSILON {
        return 0.0;
    }
    (along as f64 / length as f64).clamp(0.0, 1.0)
}

fn fraction(value: f64, min: f64, max: f64) -> f32 {
    unit_position(value, min, max) as f32
}

/// Place `t` in `[0, 1]` onto `[min, max]` without forming `max - min`.
///
/// `max - min` overflows to infinity for extreme finite endpoints such as
/// `-f64::MAX..=f64::MAX`, and a later division then becomes NaN.
fn value_at(t: f64, min: f64, max: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    if !min.is_finite() || !max.is_finite() || min == max {
        return min;
    }
    let mixed = min * (1.0 - t) + max * t;
    if mixed.is_finite() {
        return mixed.clamp(min, max);
    }
    // Opposite-sign extremes overflow the weighted sum. Split at zero.
    if min < 0.0 && max > 0.0 {
        let zero = (-min) / (max + (-min));
        if zero.is_finite() {
            let placed = if t <= zero {
                min * (1.0 - t / zero)
            } else {
                max * ((t - zero) / (1.0 - zero))
            };
            if placed.is_finite() {
                return placed.clamp(min, max);
            }
        }
    }
    min
}

/// `(value - min) / (max - min)` without a subtraction that can overflow.
fn unit_position(value: f64, min: f64, max: f64) -> f64 {
    if !value.is_finite() || !min.is_finite() || !max.is_finite() || min == max {
        return 0.0;
    }
    let value = value.clamp(min, max);
    let direct = (value - min) / (max - min);
    if direct.is_finite() {
        return direct.clamp(0.0, 1.0);
    }
    if min < 0.0 && max > 0.0 {
        if value <= 0.0 {
            let to_zero = (-min) / (max + (-min));
            let along = if value == min { 0.0 } else { 1.0 - value / min };
            if to_zero.is_finite() && along.is_finite() {
                return (to_zero * along).clamp(0.0, 1.0);
            }
        } else {
            let from_zero = max / (max + (-min));
            let along = value / max;
            if from_zero.is_finite() && along.is_finite() {
                return (1.0 - from_zero + from_zero * along).clamp(0.0, 1.0);
            }
        }
    }
    0.0
}

fn clamp(value: f64, min: f64, max: f64) -> f64 {
    if !value.is_finite() {
        return min;
    }
    value.clamp(min, max)
}

fn snap(value: f64, min: f64, max: f64, step: Option<f64>) -> f64 {
    let Some(step) = step.filter(|step| step.is_finite() && *step > 0.0) else {
        return clamp(value, min, max);
    };
    let value = clamp(value, min, max);
    let steps = ((value - min) / step).round();
    if !steps.is_finite() {
        return value;
    }
    let snapped = min + steps * step;
    if !snapped.is_finite() {
        return value;
    }
    clamp(snapped, min, max)
}

#[cfg(test)]
mod tests {
    use super::Slider;
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    struct Frame {
        value: f64,
        reported: f64,
        changed: bool,
    }

    fn slider_frame(start: f64, step: Option<f64>, drive: impl FnOnce(&Harness<Frame>)) -> Frame {
        let mut harness = Harness::builder().build_ui_state(
            |ui, frame: &mut Frame| {
                let response = ui.add(Slider::new(&mut frame.value, 0.0..=100.0).step(step));
                let mut reported = f64::NAN;
                ui.ctx().accesskit_node_builder(response.id, |node| {
                    if let Some(value) = node.numeric_value() {
                        reported = value;
                    }
                });
                frame.reported = reported;
                frame.changed = response.changed();
            },
            Frame {
                value: start,
                reported: f64::NAN,
                changed: false,
            },
        );
        drive(&harness);
        harness.step();
        Frame {
            value: harness.state().value,
            reported: harness.state().reported,
            changed: harness.state().changed,
        }
    }

    #[test]
    fn arrow_key_updates_value_and_a11y_in_the_same_frame() {
        let frame = slider_frame(40.0, Some(10.0), |harness| {
            harness.get_by_label("Slider").focus();
            harness.key_down(egui::Key::ArrowRight);
        });
        assert_eq!(frame.value, 50.0);
        assert_eq!(frame.reported, 50.0);
        assert!(frame.changed);
    }

    #[test]
    fn drag_updates_value_and_a11y_in_the_same_frame() {
        let frame = slider_frame(0.0, None, |harness| {
            let node = harness.get_by_label("Slider");
            let rect = node.rect();
            // Press at the rail start, then move to the far end in the same queued step.
            let start = rect.left_center();
            let end = rect.right_center();
            harness.event(egui::Event::PointerMoved(start));
            harness.event(egui::Event::PointerButton {
                pos: start,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            });
            harness.event(egui::Event::PointerMoved(end));
        });
        assert!(frame.value > 90.0, "dragged value {}", frame.value);
        assert!((frame.reported - frame.value).abs() < 1e-9);
        assert!(frame.changed);
    }

    #[test]
    fn unchanged_drag_does_not_mark_changed() {
        // A press that does not move the pointer stays at the snapped start.
        let frame = slider_frame(0.0, Some(10.0), |harness| {
            let rect = harness.get_by_label("Slider").rect();
            let pos = rect.left_center();
            harness.event(egui::Event::PointerMoved(pos));
            harness.event(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::default(),
            });
        });
        assert_eq!(frame.value, 0.0);
        assert_eq!(frame.reported, 0.0);
        assert!(!frame.changed);
    }

    #[test]
    fn key_that_does_not_move_does_not_mark_changed() {
        let frame = slider_frame(0.0, Some(10.0), |harness| {
            harness.get_by_label("Slider").focus();
            harness.key_down(egui::Key::ArrowLeft);
        });
        assert_eq!(frame.value, 0.0);
        assert_eq!(frame.reported, 0.0);
        assert!(!frame.changed);
    }
}
