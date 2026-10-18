//! Fluent Spinner.
//!
//! A `brand_stroke_1` arc on a `neutral_stroke_2` track. The arc rotates with
//! [`foundation::motion::animate`] over `durationSlow`. Reduced motion snaps
//! that animation, so the arc stays still.

use egui::{Pos2, Response, Sense, Stroke, TextStyle, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType};

use fluent2_tokens::generated::global::stroke_width;

use crate::foundation::motion::{self, Curve, Duration};
use crate::foundation::text::TypeRamp;
use crate::theme::{self, Theme};

/// Diameter of the spinner, matching Fluent's size ramp.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SpinnerSize {
    /// 12 px, stroke 1.
    Tiny,
    /// 16 px, stroke 1.
    ExtraSmall,
    /// 20 px, stroke 2.
    Small,
    /// 24 px, stroke 2.
    #[default]
    Medium,
    /// 28 px, stroke 3.
    Large,
    /// 32 px, stroke 3.
    ExtraLarge,
    /// 36 px, stroke 4.
    Huge,
}

/// A spinning progress arc, with an optional label.
///
/// ```
/// # use fluent2_egui::controls::{Spinner, SpinnerSize};
/// # fn demo(ui: &mut egui::Ui) {
/// let _ = ui.add(Spinner::new().size(SpinnerSize::Small).label("Loading"));
/// # }
/// ```
pub struct Spinner<'a> {
    size: SpinnerSize,
    label: Option<WidgetText>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> Spinner<'a> {
    /// A medium spinner with no label.
    #[must_use]
    pub fn new() -> Self {
        Self {
            size: SpinnerSize::Medium,
            label: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Diameter. Defaults to [`SpinnerSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: SpinnerSize) -> Self {
        self.size = size;
        self
    }

    /// Text after the arc. Also the accessible name.
    #[must_use]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = Some(label.into());
        self
    }
}

impl Default for Spinner<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Spinner<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let (diameter, stroke_w) = metrics(self.size);
        let galley = self.label.as_ref().map(|label| {
            label
                .clone()
                .color(theme::c(theme.color.neutral_foreground_1))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    TextStyle::Name(TypeRamp::Body1.name().into()),
                )
        });
        let gap = if galley.is_some() { 8.0 } else { 0.0 };
        let text_w = galley.as_ref().map(|g| g.size().x).unwrap_or(0.0);
        let text_h = galley.as_ref().map(|g| g.size().y).unwrap_or(0.0);
        let desired = Vec2::new(
            finite_nonneg(diameter + gap + text_w),
            finite_nonneg(diameter.max(text_h)),
        );
        let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
        let visible = ui.is_rect_visible(rect);
        if visible {
            let center = Pos2::new(rect.left() + diameter * 0.5, rect.center().y);
            let radius = (diameter - stroke_w) * 0.5;
            let track = Stroke::new(stroke_w, theme::c(theme.color.neutral_stroke_2));
            let arc = Stroke::new(stroke_w, theme::c(theme.color.brand_stroke_1));
            ui.painter().circle_stroke(center, radius, track);
            let going = ui.ctx().data(|data| {
                data.get_temp::<bool>(response.id.with("spin_dir"))
                    .unwrap_or(true)
            });
            let target = if going { 1.0 } else { 0.0 };
            let phase = motion::animate(
                ui.ctx(),
                response.id.with("spin"),
                target,
                Duration::Slow,
                Curve::Linear,
            );
            let origin = if going { 0.0 } else { 1.0 };
            if (phase - origin).abs() > 0.01 && (phase - target).abs() < 0.01 {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(response.id.with("spin_dir"), !going));
            }
            let angle = phase * std::f32::consts::TAU;
            paint_arc(
                ui,
                center,
                radius,
                angle,
                angle + std::f32::consts::FRAC_PI_2 * 1.5,
                arc,
            );
            if let Some(galley) = galley {
                let pos = egui::pos2(
                    rect.left() + diameter + gap,
                    rect.center().y - galley.size().y * 0.5,
                );
                ui.painter()
                    .galley(pos, galley, theme::c(theme.color.neutral_foreground_1));
            }
        }
        if visible && !motion::reduced_motion(ui.ctx()) {
            ui.ctx().request_repaint();
        }
        let label = self
            .label
            .as_ref()
            .map(|text| text.text().to_owned())
            .unwrap_or_else(|| "Loading".to_owned());
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::ProgressIndicator, ui.is_enabled(), label.clone())
        });
        response
    }
}

fn metrics(size: SpinnerSize) -> (f32, f32) {
    match size {
        SpinnerSize::Tiny => (12.0, stroke_width::THIN),
        SpinnerSize::ExtraSmall => (16.0, stroke_width::THIN),
        SpinnerSize::Small => (20.0, stroke_width::THICK),
        SpinnerSize::Medium => (24.0, stroke_width::THICK),
        SpinnerSize::Large => (28.0, stroke_width::THICKER),
        SpinnerSize::ExtraLarge => (32.0, stroke_width::THICKER),
        SpinnerSize::Huge => (36.0, stroke_width::THICKEST),
    }
}

fn paint_arc(ui: &Ui, center: Pos2, radius: f32, start: f32, end: f32, stroke: Stroke) {
    let steps = 24;
    let mut previous = point_on(center, radius, start);
    for step in 1..=steps {
        let t = start + (end - start) * (step as f32 / steps as f32);
        let next = point_on(center, radius, t);
        ui.painter().line_segment([previous, next], stroke);
        previous = next;
    }
}

fn point_on(center: Pos2, radius: f32, angle: f32) -> Pos2 {
    Pos2::new(center.x + radius * angle.cos(), center.y + radius * angle.sin())
}

fn ensure_type_ramp(ui: &mut Ui) {
    use crate::foundation::text::{font_id, style};
    if ui.style().text_styles.contains_key(&style(TypeRamp::Body1)) {
        return;
    }
    let styles = &mut ui.style_mut().text_styles;
    for step in TypeRamp::ALL {
        styles.insert(style(step), font_id(step));
    }
}

fn finite_nonneg(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}
