//! Fluent checkbox.
//!
//! Metrics follow Fluent UI React `useCheckboxStyles` (the revision pinned on WI-1546).
//! Sizes, radii and colours come from the installed [`Theme`].

use egui::{
    CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, Widget, WidgetInfo,
    WidgetText, WidgetType, epaint::RectShape,
};
use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

/// The three states of a checkbox. Click and Space move [`Unchecked`](Self::Unchecked) and [`Mixed`](Self::Mixed) to
/// [`Checked`](Self::Checked), and [`Checked`](Self::Checked) back to [`Unchecked`](Self::Unchecked).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CheckState {
    /// Empty box.
    #[default]
    Unchecked,
    /// Brand fill with a check mark.
    Checked,
    /// Brand fill with a short bar. The next click becomes [`Checked`](CheckState::Checked).
    Mixed,
}

impl From<bool> for CheckState {
    fn from(value: bool) -> Self {
        if value { Self::Checked } else { Self::Unchecked }
    }
}

/// Box size. [`Medium`] is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum CheckboxSize {
    /// 16 px box.
    #[default]
    Medium,
    /// 20 px box.
    Large,
}

/// A Fluent checkbox bound to a [`CheckState`].
///
/// ```
/// # use fluent2_egui::{CheckState, Checkbox};
/// # fn demo(ui: &mut egui::Ui, state: &mut CheckState, on: &mut bool) {
/// ui.add(Checkbox::new(state, "Show hidden"));
/// ui.add(Checkbox::bool(on, "Enabled").label_before());
/// # }
/// ```
pub struct Checkbox<'a> {
    state: &'a mut CheckState,
    label: WidgetText,
    label_before: bool,
    size: CheckboxSize,
    focusable: bool,
}

impl<'a> Checkbox<'a> {
    /// A checkbox whose value is a [`CheckState`].
    pub fn new(state: &'a mut CheckState, label: impl Into<WidgetText>) -> Self {
        Self {
            state,
            label: label.into(),
            label_before: false,
            size: CheckboxSize::Medium,
            focusable: true,
        }
    }

    /// A checkbox whose value is a `bool`. A `true` value is [`CheckState::Checked`].
    pub fn bool(value: &'a mut bool, label: impl Into<WidgetText>) -> BoolCheckbox<'a> {
        BoolCheckbox {
            value,
            label: label.into(),
            label_before: false,
            size: CheckboxSize::Medium,
            focusable: true,
        }
    }

    /// Draw the label before the box.
    #[must_use]
    pub fn label_before(mut self) -> Self {
        self.label_before = true;
        self
    }

    /// Box size. Defaults to `CheckboxSize::Medium` (16 px).
    #[must_use]
    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    /// Keep the box out of Tab order. For checkboxes inside a container that is one Tab stop,
    /// such as the DataGrid. The box still takes clicks.
    #[must_use]
    pub(crate) fn not_a_tab_stop(mut self) -> Self {
        self.focusable = false;
        self
    }
}

/// A checkbox bound to a `bool`. Built by [`Checkbox::bool`].
pub struct BoolCheckbox<'a> {
    value: &'a mut bool,
    label: WidgetText,
    label_before: bool,
    size: CheckboxSize,
    focusable: bool,
}

impl<'a> BoolCheckbox<'a> {
    /// Draw the label before the box.
    #[must_use]
    pub fn label_before(mut self) -> Self {
        self.label_before = true;
        self
    }

    /// Box size. Defaults to [`CheckboxSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: CheckboxSize) -> Self {
        self.size = size;
        self
    }

    /// Keep the box out of Tab order (see [`Checkbox::not_a_tab_stop`]).
    #[must_use]
    pub(crate) fn not_a_tab_stop(mut self) -> Self {
        self.focusable = false;
        self
    }
}

impl Widget for Checkbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Checkbox {
            state,
            label,
            label_before,
            size,
            focusable,
        } = self;
        show(ui, label, label_before, size, focusable, |activated| {
            if activated {
                *state = match *state {
                    CheckState::Unchecked | CheckState::Mixed => CheckState::Checked,
                    CheckState::Checked => CheckState::Unchecked,
                };
            }
            *state
        })
    }
}

impl Widget for BoolCheckbox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let BoolCheckbox {
            value,
            label,
            label_before,
            size,
            focusable,
        } = self;
        show(ui, label, label_before, size, focusable, |activated| {
            if activated {
                *value = !*value;
            }
            CheckState::from(*value)
        })
    }
}

fn show(
    ui: &mut Ui,
    label: WidgetText,
    label_before: bool,
    size: CheckboxSize,
    focusable: bool,
    update: impl FnOnce(bool) -> CheckState,
) -> Response {
    let theme = Theme::get(ui.ctx());
    ensure_type_ramp(ui);
    let enabled = ui.is_enabled();
    let box_px: f32 = match size {
        CheckboxSize::Medium => 16.0,
        CheckboxSize::Large => 20.0,
    };
    let gap = spacing_horizontal::S;
    let galley = label.into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Name(TypeRamp::Body1.name().into()),
    );
    let label_text = galley.text().to_string();
    let density = crate::theme::density_of(ui);
    let height = density.height(box_px.max(galley.size().y));
    let width = (box_px + gap + galley.size().x).max(density.min_target());
    let sense = if focusable { Sense::click() } else { Sense::CLICK };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(finite(width), finite(height)), sense);
    let activated = response.clicked() || (response.has_focus() && space_pressed(ui));
    let state = update(enabled && activated);

    if ui.is_rect_visible(rect) {
        let box_rect = if label_before {
            Rect::from_min_size(
                egui::pos2(rect.right() - box_px, rect.center().y - box_px / 2.0),
                Vec2::splat(box_px),
            )
        } else {
            Rect::from_min_size(
                egui::pos2(rect.left(), rect.center().y - box_px / 2.0),
                Vec2::splat(box_px),
            )
        };
        paint_box(
            ui,
            box_rect,
            state,
            &theme,
            enabled,
            response.hovered(),
            response.is_pointer_button_down_on(),
        );
        let text_x = if label_before {
            rect.left()
        } else {
            rect.left() + box_px + gap
        };
        let text_pos = egui::pos2(text_x, rect.center().y - galley.size().y / 2.0);
        let foreground = if enabled {
            c(theme.color.neutral_foreground_1)
        } else {
            c(theme.color.neutral_foreground_disabled)
        };
        ui.painter().galley(text_pos, galley, foreground);
        if focus_visible(ui, &response) {
            paint_focus_ring(
                ui,
                box_rect,
                CornerRadius::same(rounded_u8(border_radius::SMALL)),
                &theme,
            );
        }
    }

    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::Checkbox, enabled, label_text.clone());
        info.selected = Some(state != CheckState::Unchecked);
        // Mixed is neither checked nor unchecked. AccessKit's tristate is `Toggled::Mixed`.
        if state == CheckState::Mixed {
            info.selected = Some(false);
        }
        info
    });
    let id = response.id;
    let mixed = state == CheckState::Mixed;
    let checked = state == CheckState::Checked;
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_toggled(if mixed {
            egui::accesskit::Toggled::Mixed
        } else if checked {
            egui::accesskit::Toggled::True
        } else {
            egui::accesskit::Toggled::False
        });
    });
    response
}

pub(crate) fn paint_box(
    ui: &Ui,
    rect: Rect,
    state: CheckState,
    theme: &Theme,
    enabled: bool,
    hovered: bool,
    pressed: bool,
) {
    let color = &theme.color;
    let radius = CornerRadius::same(rounded_u8(border_radius::SMALL));
    let (fill, stroke) = if !enabled {
        (
            if state == CheckState::Unchecked {
                egui::Color32::TRANSPARENT
            } else {
                c(color.neutral_background_disabled)
            },
            c(color.neutral_stroke_disabled),
        )
    } else {
        match state {
            CheckState::Unchecked => {
                let stroke = if pressed {
                    color.neutral_stroke_accessible_pressed
                } else if hovered {
                    color.neutral_stroke_accessible_hover
                } else {
                    color.neutral_stroke_accessible
                };
                (egui::Color32::TRANSPARENT, c(stroke))
            }
            CheckState::Checked | CheckState::Mixed => {
                let fill = if pressed {
                    color.compound_brand_background_pressed
                } else if hovered {
                    color.compound_brand_background_hover
                } else {
                    color.compound_brand_background
                };
                (c(fill), c(fill))
            }
        }
    };
    ui.painter().add(RectShape::new(
        rect,
        radius,
        fill,
        Stroke::new(stroke_width::THIN, stroke),
        StrokeKind::Inside,
    ));
    let mark = c(if enabled {
        color.neutral_foreground_inverted
    } else {
        color.neutral_foreground_disabled
    });
    match state {
        CheckState::Checked => paint_check(ui, rect, mark),
        CheckState::Mixed => {
            let bar = Rect::from_center_size(rect.center(), Vec2::new(8.0, 2.0));
            ui.painter().rect_filled(bar, CornerRadius::ZERO, mark);
        }
        CheckState::Unchecked => {}
    }
}

fn paint_check(ui: &Ui, rect: Rect, color: egui::Color32) {
    // The catalogue has a 16 px check, not a 12 px one. Paint that glyph inside the box.
    let Some(bytes) = fluent2_icons::svg(Icon::Checkmark, IconSize::S16, IconStyle::Regular) else {
        ui.painter().add(egui::Shape::line(
            vec![
                lerp_inside(rect, Vec2::new(0.25, 0.50)),
                lerp_inside(rect, Vec2::new(0.42, 0.68)),
                lerp_inside(rect, Vec2::new(0.75, 0.32)),
            ],
            Stroke::new(stroke_width::THICK, color),
        ));
        return;
    };
    let px = rect.width().min(rect.height());
    let origin = rect.center() - Vec2::splat(px / 2.0);
    let icon_rect = Rect::from_min_size(origin, Vec2::splat(finite(px)));
    let uri = "bytes://fluent2-icon/Checkmark/S16/regular.svg".to_owned();
    ui.ctx().include_bytes(uri.clone(), bytes.to_vec());
    egui::Image::from_uri(uri)
        .tint(color)
        .fit_to_exact_size(Vec2::splat(finite(px)))
        .paint_at(ui, icon_rect);
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

fn space_pressed(ui: &Ui) -> bool {
    ui.input(|input| input.key_pressed(egui::Key::Space))
}

/// The shared sticky rule: a ring from the last key press until the next pointer press, not
/// only on the frame of the key event.
fn focus_visible(ui: &Ui, response: &Response) -> bool {
    crate::foundation::focus::focus_visible(ui, response)
}

fn paint_focus_ring(ui: &Ui, rect: Rect, radius: CornerRadius, theme: &Theme) {
    let painter = ui.painter();
    painter.rect_stroke(
        rect.expand(1.0),
        grow(radius, 1),
        Stroke::new(1.0, c(theme.color.stroke_focus_1)),
        StrokeKind::Middle,
    );
    painter.rect_stroke(
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

fn lerp_inside(rect: Rect, factor: Vec2) -> egui::Pos2 {
    egui::pos2(
        rect.left() + rect.width() * factor.x.clamp(0.0, 1.0),
        rect.top() + rect.height() * factor.y.clamp(0.0, 1.0),
    )
}
