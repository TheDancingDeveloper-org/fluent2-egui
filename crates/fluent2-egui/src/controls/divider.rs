//! Fluent Divider.
//!
//! The rule is `neutral_stroke_2`. Optional text is centred on the rule and
//! drawn in `neutral_foreground_2` at body1.

use egui::{Response, Role, Sense, Stroke, TextStyle, Ui, Vec2, Widget, WidgetInfo, WidgetText};

use fluent2_tokens::generated::global::stroke_width;

use crate::foundation::text::TypeRamp;
use crate::theme::{self, Theme};

/// A horizontal or vertical rule, optionally with centred text.
///
/// ```
/// # use fluent2_egui::controls::Divider;
/// # fn demo(ui: &mut egui::Ui) {
/// let _ = ui.add(Divider::horizontal().text("or"));
/// let _ = ui.add(Divider::vertical());
/// # }
/// ```
pub struct Divider<'a> {
    vertical: bool,
    text: Option<WidgetText>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> Divider<'a> {
    /// A rule that spans the available width.
    #[must_use]
    pub fn horizontal() -> Self {
        Self {
            vertical: false,
            text: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// A rule that spans the available height.
    #[must_use]
    pub fn vertical() -> Self {
        Self {
            vertical: true,
            text: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Centre this label on the rule. Ignored on a vertical divider that has
    /// no width to centre in; the label is still the accessible name.
    #[must_use]
    pub fn text(mut self, text: impl Into<WidgetText>) -> Self {
        self.text = Some(text.into());
        self
    }
}

impl Widget for Divider<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let stroke = Stroke::new(stroke_width::THIN, theme::c(theme.color.neutral_stroke_2));
        let label = self
            .text
            .as_ref()
            .map(|text| text.text().to_owned())
            .unwrap_or_default();
        if self.vertical {
            let height = finite(ui.available_height()).max(1.0);
            let (rect, response) =
                ui.allocate_exact_size(Vec2::new(stroke_width::THIN, height), Sense::hover());
            if ui.is_rect_visible(rect) {
                ui.painter().vline(rect.center().x, rect.y_range(), stroke);
            }
            response.widget_info(|| WidgetInfo::labeled(Role::Label, true, label.clone()));
            return response;
        }
        let width = finite(ui.available_width()).max(1.0);
        let galley = self.text.as_ref().map(|text| {
            text.clone()
                .color(theme::c(theme.color.neutral_foreground_2))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    width * 0.6,
                    TextStyle::Name(TypeRamp::Body1.name().into()),
                )
        });
        let height = galley
            .as_ref()
            .map(|g| g.size().y)
            .unwrap_or(stroke_width::THIN)
            .max(stroke_width::THIN);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, finite(height)), Sense::hover());
        if ui.is_rect_visible(rect) {
            let y = rect.center().y;
            if let Some(galley) = galley {
                let gap = 8.0;
                let text_w = galley.size().x;
                let left_end = (rect.center().x - text_w * 0.5 - gap).max(rect.left());
                let right_start = (rect.center().x + text_w * 0.5 + gap).min(rect.right());
                ui.painter().hline(rect.left()..=left_end, y, stroke);
                ui.painter().hline(right_start..=rect.right(), y, stroke);
                let pos = egui::pos2(
                    rect.center().x - text_w * 0.5,
                    rect.center().y - galley.size().y * 0.5,
                );
                ui.painter()
                    .galley(pos, galley, theme::c(theme.color.neutral_foreground_2));
            } else {
                ui.painter().hline(rect.x_range(), y, stroke);
            }
        }
        response.widget_info(|| WidgetInfo::labeled(Role::Label, true, label.clone()));
        response
    }
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

fn finite(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}
