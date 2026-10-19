//! Fluent radio group.
//!
//! Metrics follow Fluent UI React `Radio` (the revision pinned on WI-1546). Arrow keys move
//! the selection inside the group. Only the selected radio is in the Tab order.

use egui::{
    CornerRadius, Key, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, WidgetText,
    epaint::RectShape,
};
use fluent2_tokens::generated::global::{spacing_horizontal, stroke_width};

use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

const OUTER: f32 = 16.0;
const DOT: f32 = 10.0;

/// A group of mutually exclusive options. The value is the selected option's index.
///
/// ```
/// # use fluent2_egui::RadioGroup;
/// # fn demo(ui: &mut egui::Ui, choice: &mut usize) {
/// ui.add(RadioGroup::new(choice).option("Left").option("Right").horizontal());
/// # }
/// ```
pub struct RadioGroup<'a> {
    selected: &'a mut usize,
    options: Vec<WidgetText>,
    horizontal: bool,
    label: Option<String>,
}

impl<'a> RadioGroup<'a> {
    /// A vertical group whose selection is `selected`.
    pub fn new(selected: &'a mut usize) -> Self {
        Self {
            selected,
            options: Vec::new(),
            horizontal: false,
            label: None,
        }
    }

    /// Append one option. The first option added is index 0.
    #[must_use]
    pub fn option(mut self, label: impl Into<WidgetText>) -> Self {
        self.options.push(label.into());
        self
    }

    /// Accessible name of the group, read by screen readers. Not drawn. Inside a
    /// [`super::Field`] the Field's label is the default.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Lay the options out in a row instead of a column.
    #[must_use]
    pub fn horizontal(mut self) -> Self {
        self.horizontal = true;
        self
    }
}

/// What [`RadioGroup`] added.
pub struct RadioGroupResponse {
    /// The group's response. Its id owns the roving focus.
    pub response: Response,
}

impl RadioGroup<'_> {
    /// Add the group.
    pub fn show(self, ui: &mut Ui) -> RadioGroupResponse {
        let RadioGroup {
            selected,
            options,
            horizontal,
            label: group_label,
        } = self;
        let group_label = group_label.or_else(|| super::field::enclosing_field_label(ui.ctx()));
        let theme = Theme::get(ui.ctx());
        ensure_type_ramp(ui);
        let enabled = ui.is_enabled();
        let count = options.len();
        if count > 0 {
            *selected = (*selected).min(count - 1);
        }

        let gap = spacing_horizontal::S;
        let galleys: Vec<_> = options
            .iter()
            .map(|label| {
                label.clone().into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    TextStyle::Name(TypeRamp::Body1.name().into()),
                )
            })
            .collect();
        let density = crate::theme::density_of(ui);
        let heights: Vec<f32> = galleys
            .iter()
            .map(|galley| density.height(OUTER.max(galley.size().y)))
            .collect();
        let widths: Vec<f32> = galleys
            .iter()
            .map(|galley| (OUTER + gap + galley.size().x).max(density.min_target()))
            .collect();
        let (width, height) = if horizontal {
            let width = widths.iter().sum::<f32>() + gap * (count.saturating_sub(1)) as f32;
            let height = heights.iter().copied().fold(0.0_f32, f32::max);
            (width, height)
        } else {
            let width = widths.iter().copied().fold(0.0_f32, f32::max);
            let height = heights.iter().sum::<f32>() + gap * (count.saturating_sub(1)) as f32;
            (width, height)
        };
        let (rect, mut response) =
            ui.allocate_exact_size(Vec2::new(finite(width), finite(height)), Sense::click());

        // Roving focus: the group is one Tab stop. Arrows move the selection.
        let mut clicked = None;
        if ui.is_rect_visible(rect) {
            let mut cursor = rect.min;
            for (index, galley) in galleys.into_iter().enumerate() {
                let row = Vec2::new(widths[index], heights[index]);
                let item = Rect::from_min_size(cursor, row);
                let circle = Rect::from_center_size(
                    egui::pos2(item.left() + OUTER / 2.0, item.center().y),
                    Vec2::splat(OUTER),
                );
                let pointer = ui.input(|input| input.pointer.interact_pos());
                let over = pointer.is_some_and(|pos| item.contains(pos));
                if response.clicked() && over {
                    clicked = Some(index);
                }
                paint_radio(
                    ui,
                    circle,
                    index == *selected,
                    &theme,
                    enabled,
                    over,
                    response.is_pointer_button_down_on() && over,
                );
                let text_pos = egui::pos2(item.left() + OUTER + gap, item.center().y - galley.size().y / 2.0);
                let foreground = if enabled {
                    c(theme.color.neutral_foreground_1)
                } else {
                    c(theme.color.neutral_foreground_disabled)
                };
                ui.painter().galley(text_pos, galley, foreground);
                let child = response.id.with(("radio", index));
                let label = options_label(&options, index);
                let on = index == *selected;
                ui.ctx().accesskit_node_builder(child, |node| {
                    node.set_role(egui::accesskit::Role::RadioButton);
                    node.set_label(label);
                    node.set_toggled(if on {
                        egui::accesskit::Toggled::True
                    } else {
                        egui::accesskit::Toggled::False
                    });
                    node.set_bounds(accesskit_rect(item));
                });
                if horizontal {
                    cursor.x += row.x + gap;
                } else {
                    cursor.y += row.y + gap;
                }
            }
            if focus_visible(ui, &response) && count > 0 {
                let index = (*selected).min(count - 1);
                let mut origin = rect.min;
                for i in 0..index {
                    if horizontal {
                        origin.x += widths[i] + gap;
                    } else {
                        origin.y += heights[i] + gap;
                    }
                }
                let circle = Rect::from_center_size(
                    egui::pos2(origin.x + OUTER / 2.0, origin.y + heights[index] / 2.0),
                    Vec2::splat(OUTER),
                );
                paint_focus_ring(ui, circle, CornerRadius::same(rounded_u8(OUTER / 2.0)), &theme);
            }
        }

        if enabled {
            if let Some(index) = clicked {
                *selected = index;
                response.mark_changed();
            }
            if response.has_focus() && count > 0 {
                if space_pressed(ui) {
                    response.mark_changed();
                }
                let next = arrow_delta(ui, horizontal);
                if next != 0 {
                    let len = count as i32;
                    let current = (*selected).min(count - 1) as i32;
                    let moved = (current + next).rem_euclid(len) as usize;
                    if moved != *selected {
                        *selected = moved;
                        response.mark_changed();
                    }
                }
            }
        }

        // The allocated widget is the group: one Tab stop, with the radios as children.
        // The child ids hang off this response, so two groups in one Ui do not collide.
        let group_id = response.id;
        // The group holds focus; the active descendant tells screen readers which radio that
        // is, so they announce "Large, radio button, checked" rather than an unnamed group.
        let active = (count > 0).then(|| {
            group_id
                .with(("radio", (*selected).min(count - 1)))
                .accesskit_id()
        });
        ui.ctx().accesskit_node_builder(group_id, |node| {
            node.set_role(egui::accesskit::Role::RadioGroup);
            if let Some(label) = group_label {
                node.set_label(label);
            }
            if let Some(active) = active {
                node.set_active_descendant(active);
            }
        });
        RadioGroupResponse { response }
    }
}

impl egui::Widget for RadioGroup<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

fn ensure_type_ramp(ui: &mut Ui) {
    crate::foundation::text::install_text_styles(ui.ctx());
    if ui
        .style()
        .text_styles
        .contains_key(&crate::foundation::text::style(TypeRamp::Body1))
    {
        return;
    }
    let styles = &mut ui.style_mut().text_styles;
    for step in TypeRamp::ALL {
        styles.insert(
            crate::foundation::text::style(step),
            crate::foundation::text::font_id(step),
        );
    }
}

fn options_label(options: &[WidgetText], index: usize) -> String {
    options
        .get(index)
        .map(|text| text.text().to_string())
        .unwrap_or_default()
}

fn paint_radio(
    ui: &Ui,
    rect: Rect,
    selected: bool,
    theme: &Theme,
    enabled: bool,
    hovered: bool,
    pressed: bool,
) {
    let color = &theme.color;
    let stroke_color = if !enabled {
        c(color.neutral_stroke_disabled)
    } else if selected {
        let token = if pressed {
            color.compound_brand_stroke_pressed
        } else if hovered {
            color.compound_brand_stroke_hover
        } else {
            color.compound_brand_stroke
        };
        c(token)
    } else if pressed {
        c(color.neutral_stroke_accessible_pressed)
    } else if hovered {
        c(color.neutral_stroke_accessible_hover)
    } else {
        c(color.neutral_stroke_accessible)
    };
    ui.painter().add(RectShape::new(
        rect,
        CornerRadius::same(rounded_u8(rect.width() / 2.0)),
        egui::Color32::TRANSPARENT,
        Stroke::new(stroke_width::THIN, stroke_color),
        StrokeKind::Inside,
    ));
    if selected {
        let dot = if enabled {
            c(color.compound_brand_foreground_1)
        } else {
            c(color.neutral_foreground_disabled)
        };
        ui.painter().circle_filled(rect.center(), DOT / 2.0, dot);
    }
}

fn space_pressed(ui: &Ui) -> bool {
    ui.input(|input| input.key_pressed(Key::Space))
}

fn arrow_delta(ui: &Ui, horizontal: bool) -> i32 {
    ui.input(|input| {
        let mut delta = 0;
        if horizontal {
            if input.key_pressed(Key::ArrowRight) {
                delta += 1;
            }
            if input.key_pressed(Key::ArrowLeft) {
                delta -= 1;
            }
        } else {
            if input.key_pressed(Key::ArrowDown) {
                delta += 1;
            }
            if input.key_pressed(Key::ArrowUp) {
                delta -= 1;
            }
        }
        delta
    })
}

/// The shared sticky rule: a ring from the last key press until the next pointer press, not
/// only on the frame of the key event.
fn focus_visible(ui: &Ui, response: &Response) -> bool {
    crate::foundation::focus::focus_visible(ui, response)
}

fn paint_focus_ring(ui: &Ui, rect: Rect, radius: CornerRadius, theme: &Theme) {
    ui.painter().rect_stroke(
        rect.expand(1.0),
        grow(radius, 1),
        Stroke::new(1.0, c(theme.color.stroke_focus_1)),
        StrokeKind::Middle,
    );
    ui.painter().rect_stroke(
        rect.expand(2.0),
        grow(radius, 2),
        Stroke::new(2.0, c(theme.color.stroke_focus_2)),
        StrokeKind::Middle,
    );
}

fn grow(radius: CornerRadius, by: u8) -> CornerRadius {
    CornerRadius {
        nw: radius.nw.saturating_add(by),
        ne: radius.ne.saturating_add(by),
        sw: radius.sw.saturating_add(by),
        se: radius.se.saturating_add(by),
    }
}

fn accesskit_rect(rect: Rect) -> egui::accesskit::Rect {
    egui::accesskit::Rect::new(
        rect.min.x.into(),
        rect.min.y.into(),
        rect.max.x.into(),
        rect.max.y.into(),
    )
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value.max(0.0) } else { 0.0 }
}

fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= f32::from(u8::MAX) {
        u8::MAX
    } else {
        value.round() as u8
    }
}
