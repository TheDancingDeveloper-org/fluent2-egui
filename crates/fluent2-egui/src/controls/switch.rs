//! Fluent switch.
//!
//! Metrics follow Fluent UI React `useSwitchStyles` (the revision pinned on WI-1546).
//! The thumb slides with [`crate::foundation::motion::animate`].

use egui::{
    CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, Widget, WidgetInfo,
    WidgetText, Role, epaint::RectShape,
};
use fluent2_tokens::generated::global::{spacing_horizontal, stroke_width};

use crate::foundation::motion::{self, Curve, Duration};
use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

const TRACK_W: f32 = 40.0;
const TRACK_H: f32 = 20.0;
const THUMB: f32 = 14.0;

/// A Fluent switch bound to a `bool`.
///
/// ```
/// # use fluent2_egui::Switch;
/// # fn demo(ui: &mut egui::Ui, on: &mut bool) {
/// ui.add(Switch::new(on, "Notifications"));
/// # }
/// ```
pub struct Switch<'a> {
    value: &'a mut bool,
    label: WidgetText,
    label_before: bool,
}

impl<'a> Switch<'a> {
    /// A switch with a label drawn after the track.
    pub fn new(value: &'a mut bool, label: impl Into<WidgetText>) -> Self {
        Self {
            value,
            label: label.into(),
            label_before: false,
        }
    }

    /// Draw the label before the track.
    #[must_use]
    pub fn label_before(mut self) -> Self {
        self.label_before = true;
        self
    }
}

impl Widget for Switch<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Switch {
            value,
            label,
            label_before,
        } = self;
        let theme = Theme::get(ui.ctx());
        ensure_type_ramp(ui);
        let enabled = ui.is_enabled();
        let gap = spacing_horizontal::S;
        let galley = label.into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            TextStyle::Name(TypeRamp::Body1.name().into()),
        );
        let label_text = galley.text().to_string();
        let density = crate::theme::density_of(ui);
        let height = density.height(TRACK_H.max(galley.size().y));
        let width = (TRACK_W + gap + galley.size().x).max(density.min_target());
        let (rect, response) =
            ui.allocate_exact_size(Vec2::new(finite(width), finite(height)), Sense::click());
        let activated = response.clicked() || (response.has_focus() && space_pressed(ui));
        if enabled && activated {
            *value = !*value;
        }
        let on = *value;

        if ui.is_rect_visible(rect) {
            let track = if label_before {
                Rect::from_min_size(
                    egui::pos2(rect.right() - TRACK_W, rect.center().y - TRACK_H / 2.0),
                    Vec2::new(TRACK_W, TRACK_H),
                )
            } else {
                Rect::from_min_size(
                    egui::pos2(rect.left(), rect.center().y - TRACK_H / 2.0),
                    Vec2::new(TRACK_W, TRACK_H),
                )
            };
            let progress = motion::animate(
                ui.ctx(),
                response.id.with("thumb"),
                if on { 1.0 } else { 0.0 },
                Duration::Normal,
                Curve::EasyEase,
            );
            paint_track(
                ui,
                track,
                TrackPaint {
                    on,
                    progress,
                    enabled,
                    hovered: response.hovered(),
                    pressed: response.is_pointer_button_down_on(),
                },
                &theme,
            );
            let text_x = if label_before {
                rect.left()
            } else {
                rect.left() + TRACK_W + gap
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
                    track,
                    CornerRadius::same(rounded_u8(track.height() / 2.0)),
                    &theme,
                );
            }
        }

        // Role has no switch, so the widget info says checkbox and the AccessKit node
        // is then given the Switch role (ARCHITECTURE "Component API" rule 8). Narrator and
        // Orca announce "switch, on/off" instead of "check box, checked".
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::CheckBox, enabled, label_text.clone());
            info.selected = Some(on);
            info
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Switch);
        });
        response
    }
}

struct TrackPaint {
    on: bool,
    progress: f32,
    enabled: bool,
    hovered: bool,
    pressed: bool,
}

fn paint_track(ui: &Ui, track: Rect, paint: TrackPaint, theme: &Theme) {
    let TrackPaint {
        on,
        progress,
        enabled,
        hovered,
        pressed,
    } = paint;
    let color = &theme.color;
    let radius = CornerRadius::same(rounded_u8(track.height() / 2.0));
    let (fill, stroke) = if !enabled {
        (egui::Color32::TRANSPARENT, c(color.neutral_stroke_disabled))
    } else if on {
        let fill = if pressed {
            color.compound_brand_background_pressed
        } else if hovered {
            color.compound_brand_background_hover
        } else {
            color.compound_brand_background
        };
        (c(fill), c(fill))
    } else {
        let stroke = if pressed {
            color.neutral_stroke_accessible_pressed
        } else if hovered {
            color.neutral_stroke_accessible_hover
        } else {
            color.neutral_stroke_accessible
        };
        (egui::Color32::TRANSPARENT, c(stroke))
    };
    ui.painter().add(RectShape::new(
        track,
        radius,
        fill,
        Stroke::new(stroke_width::THIN, stroke),
        StrokeKind::Inside,
    ));
    let inset = (TRACK_H - THUMB) / 2.0;
    let travel = (TRACK_W - THUMB - inset * 2.0).max(0.0);
    let x = track.left() + inset + travel * progress.clamp(0.0, 1.0);
    let thumb = Rect::from_min_size(egui::pos2(x, track.center().y - THUMB / 2.0), Vec2::splat(THUMB));
    let thumb_color = if !enabled {
        c(color.neutral_foreground_disabled)
    } else if on {
        c(color.neutral_foreground_inverted)
    } else {
        c(color.neutral_foreground_3)
    };
    ui.painter()
        .circle_filled(thumb.center(), THUMB / 2.0, thumb_color);
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
