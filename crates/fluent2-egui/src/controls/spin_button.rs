//! Fluent SpinButton: an input field with stacked up and down chevrons.
//!
//! The text buffer is edited freely and committed on Enter or focus loss. Invalid text
//! reverts to the last committed value; Escape reverts without committing. Non-finite or
//! inverted ranges are clamped, never asserted.

use egui::{
    Align2, CornerRadius, Key, Response, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2, Widget, WidgetInfo,
};

use crate::foundation::focus::paint_focus_ring;
use crate::theme::{self, Theme};

const FIELD_HEIGHT: f32 = 32.0;
const CHEVRON_WIDTH: f32 = 24.0;

/// A numeric field with stacked step buttons.
///
/// ```
/// # use fluent2_egui::controls::SpinButton;
/// # fn demo(ui: &mut egui::Ui) {
/// let mut value = 1.0;
/// if ui.add(SpinButton::new(&mut value).range(0.0..=10.0).step(0.5).decimals(1)).changed() {
///     // `value` was committed or stepped
/// }
/// # }
/// ```
pub struct SpinButton<'a> {
    value: &'a mut f64,
    min: f64,
    max: f64,
    step: f64,
    decimals: u8,
    suffix: Option<String>,
}

impl<'a> SpinButton<'a> {
    /// A spin button editing `value`. The default range is `0.0..=100.0`, the step is `1.0`
    /// and two decimal places are shown.
    pub fn new(value: &'a mut f64) -> Self {
        Self {
            value,
            min: 0.0,
            max: 100.0,
            step: 1.0,
            decimals: 2,
            suffix: None,
        }
    }

    /// Inclusive range. A non-finite bound is dropped; an inverted range is swapped.
    #[must_use]
    pub fn range(mut self, range: std::ops::RangeInclusive<f64>) -> Self {
        let (start, end) = (*range.start(), *range.end());
        if start.is_finite() {
            self.min = start;
        }
        if end.is_finite() {
            self.max = end;
        }
        if self.min > self.max {
            std::mem::swap(&mut self.min, &mut self.max);
        }
        self
    }

    /// Amount added or subtracted by the chevrons and the arrow keys. Non-finite or
    /// non-positive steps are ignored.
    #[must_use]
    pub fn step(mut self, step: f64) -> Self {
        if step.is_finite() && step > 0.0 {
            self.step = step;
        }
        self
    }

    /// Decimal places shown and accepted on commit.
    #[must_use]
    pub fn decimals(mut self, decimals: u8) -> Self {
        self.decimals = decimals;
        self
    }

    /// Text drawn after the number. It is not part of the edited buffer.
    #[must_use]
    pub fn suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }
}

#[derive(Clone)]
struct Buffer {
    text: String,
    committed: f64,
}

impl Widget for SpinButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let SpinButton {
            value,
            min,
            max,
            step,
            decimals,
            suffix,
        } = self;
        let current = clamp_value(*value, min, max);
        *value = current;

        let id = ui.next_auto_id();
        let buffer_id = id.with("buffer");
        // The text field owns focus. `id` is also the id of the allocation below, so the field
        // takes a child id; sharing one made Tab skip from the field past every later widget.
        let edit_id = id.with("edit");
        let mut buffer = ui.data(|data| {
            data.get_temp::<Buffer>(buffer_id).unwrap_or(Buffer {
                text: format_value(current, decimals),
                committed: current,
            })
        });
        if (buffer.committed - current).abs() > f64::EPSILON * 8.0
            && !ui.memory(|memory| memory.has_focus(edit_id))
        {
            buffer.text = format_value(current, decimals);
            buffer.committed = current;
        }

        let field_width = ui.available_width().clamp(96.0, 160.0);
        let (rect, mut response) = ui.allocate_exact_size(
            Vec2::new(field_width + CHEVRON_WIDTH, FIELD_HEIGHT),
            Sense::hover(),
        );
        let field = egui::Rect::from_min_size(rect.min, Vec2::new(field_width, FIELD_HEIGHT));
        let chevrons = egui::Rect::from_min_max(field.right_top(), rect.right_bottom());
        let up_rect = egui::Rect::from_min_size(chevrons.min, Vec2::new(CHEVRON_WIDTH, FIELD_HEIGHT / 2.0));
        let down_rect = egui::Rect::from_min_max(up_rect.left_bottom(), chevrons.right_bottom());

        // The field is the one Tab stop; the arrows are for the pointer, and the keyboard
        // steps with Up/Down. Focusable arrows were unnamed Tab stops with no ring (WI-1586).
        let up = ui.interact(up_rect, id.with("up"), Sense::CLICK);
        let down = ui.interact(down_rect, id.with("down"), Sense::CLICK);
        let enabled = ui.is_enabled();

        let mut changed = false;
        let mut commit = |buffer: &mut Buffer, next: f64| {
            let next = clamp_value(next, min, max);
            buffer.text = format_value(next, decimals);
            buffer.committed = next;
            *value = next;
            changed = true;
        };

        if enabled && up.clicked() {
            commit(&mut buffer, current + step);
        }
        if enabled && down.clicked() {
            commit(&mut buffer, current - step);
        }

        let text_response = ui
            .scope_builder(egui::UiBuilder::new().max_rect(field).id_salt("edit"), |ui| {
                ui.spacing_mut().text_edit_width = field.width() - 8.0;
                ui.add(
                    TextEdit::singleline(&mut buffer.text)
                        .id(edit_id)
                        .desired_width(field.width() - 8.0)
                        .frame(egui::Frame::NONE)
                        .horizontal_align(egui::Align::RIGHT)
                        .font(egui::TextStyle::Body),
                )
            })
            .inner;

        // TextEdit marks changed on every keystroke. The contract is changed() only on
        // commit or step, so drop that flag and set it ourselves below.
        response.flags -= egui::response::Flags::CHANGED;
        // The returned response carries the field's id, so `request_focus`, `has_focus` and
        // the AccessKit SpinButton node all land on the one Tab stop.
        response = text_response.clone().union(response).union(up).union(down);
        response.flags -= egui::response::Flags::CHANGED;

        if enabled && response.has_focus() {
            let input = ui.input(|input| {
                (
                    input.key_pressed(Key::ArrowUp),
                    input.key_pressed(Key::ArrowDown),
                    input.key_pressed(Key::PageUp),
                    input.key_pressed(Key::PageDown),
                    input.key_pressed(Key::Enter),
                    input.key_pressed(Key::Escape),
                    input.smooth_scroll_delta.y,
                )
            });
            if input.0 {
                commit(&mut buffer, current + step);
            } else if input.1 {
                commit(&mut buffer, current - step);
            } else if input.2 {
                commit(&mut buffer, current + step * 10.0);
            } else if input.3 {
                commit(&mut buffer, current - step * 10.0);
            } else if input.4 {
                commit_text(&mut buffer, decimals, min, max, value, &mut changed);
            } else if input.5 {
                buffer.text = format_value(buffer.committed, decimals);
                *value = buffer.committed;
            } else if input.6.abs() > 0.0 {
                let direction = if input.6 > 0.0 { 1.0 } else { -1.0 };
                commit(&mut buffer, current + step * direction);
            }
        }

        let lost_focus = text_response.lost_focus() && !ui.input(|input| input.key_pressed(Key::Escape));
        if enabled && lost_focus {
            commit_text(&mut buffer, decimals, min, max, value, &mut changed);
        }

        paint(
            ui,
            &theme,
            &Paint {
                field,
                up: up_rect,
                down: down_rect,
                enabled,
                focused: response.has_focus(),
                suffix,
            },
        );
        ui.data_mut(|data| data.insert_temp(buffer_id, buffer));

        if changed {
            response.mark_changed();
        }
        response.widget_info(|| WidgetInfo::drag_value(enabled, *value));
        response
    }
}

fn commit_text(buffer: &mut Buffer, decimals: u8, min: f64, max: f64, value: &mut f64, changed: &mut bool) {
    let parsed = buffer.text.trim().parse::<f64>();
    match parsed {
        Ok(number) if number.is_finite() => {
            let next = clamp_value(number, min, max);
            buffer.text = format_value(next, decimals);
            buffer.committed = next;
            *value = next;
            *changed = true;
        }
        _ => {
            buffer.text = format_value(buffer.committed, decimals);
            *value = buffer.committed;
        }
    }
}

struct Paint {
    field: egui::Rect,
    up: egui::Rect,
    down: egui::Rect,
    enabled: bool,
    focused: bool,
    suffix: Option<String>,
}

fn paint(ui: &Ui, theme: &Theme, parts: &Paint) {
    let Paint {
        field,
        up,
        down,
        enabled,
        focused,
        suffix,
    } = parts;
    let (field, up, down, enabled, focused) = (*field, *up, *down, *enabled, *focused);
    let color = &theme.color;
    let painter = ui.painter();
    let radius = CornerRadius::same(4);
    let fill = theme::c(if enabled {
        color.neutral_background_1
    } else {
        color.neutral_background_disabled
    });
    let stroke = Stroke::new(
        1.0,
        theme::c(if enabled {
            color.neutral_stroke_accessible
        } else {
            color.neutral_stroke_disabled
        }),
    );
    painter.rect(
        field.union(up.union(down)),
        radius,
        fill,
        stroke,
        StrokeKind::Inside,
    );
    painter.line_segment(
        [up.left_top(), up.left_bottom()],
        Stroke::new(1.0, theme::c(color.neutral_stroke_accessible)),
    );
    painter.line_segment(
        [up.left_bottom(), up.right_bottom()],
        Stroke::new(1.0, theme::c(color.neutral_stroke_accessible)),
    );
    chevron(painter, up.center(), true, theme::c(color.neutral_foreground_1));
    chevron(
        painter,
        down.center(),
        false,
        theme::c(color.neutral_foreground_1),
    );
    if let Some(suffix) = suffix.as_deref() {
        painter.text(
            field.right_center() - Vec2::new(6.0, 0.0),
            Align2::RIGHT_CENTER,
            suffix,
            egui::TextStyle::Body.resolve(ui.style()),
            theme::c(color.neutral_foreground_2),
        );
    }
    if focused {
        paint_focus_ring(ui, field.union(up.union(down)), radius);
    }
}

fn chevron(painter: &egui::Painter, center: egui::Pos2, up: bool, color: egui::Color32) {
    let dy = if up { -2.0 } else { 2.0 };
    let stroke = Stroke::new(1.25, color);
    painter.line_segment(
        [center + Vec2::new(-4.0, dy), center + Vec2::new(0.0, -dy)],
        stroke,
    );
    painter.line_segment(
        [center + Vec2::new(4.0, dy), center + Vec2::new(0.0, -dy)],
        stroke,
    );
}

fn clamp_value(value: f64, min: f64, max: f64) -> f64 {
    if !value.is_finite() {
        return min;
    }
    value.clamp(min, max)
}

fn format_value(value: f64, decimals: u8) -> String {
    format!("{value:.prec$}", prec = decimals as usize)
}
