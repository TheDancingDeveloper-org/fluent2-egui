//! Fluent Badge. Not interactive.
//!
//! Colour table (appearance × colour). Names are `AliasColors` fields.
//!
//! | colour       | filled bg / fg                                         | ghost fg                         | outline stroke + fg                 | tint bg / fg                          |
//! |--------------|--------------------------------------------------------|----------------------------------|-------------------------------------|---------------------------------------|
//! | Brand        | brand_background / neutral_foreground_on_brand         | brand_foreground_1               | brand_stroke_1 / brand_foreground_1 | brand_background_2 / brand_foreground_1 |
//! | Danger       | status_danger_background_3 / status_danger_foreground_inverted | status_danger_foreground_1 | status_danger_border_1 / status_danger_foreground_1 | status_danger_background_1 / status_danger_foreground_1 |
//! | Important    | neutral_foreground_1 / neutral_background_1            | neutral_foreground_1             | neutral_stroke_accessible / neutral_foreground_1 | neutral_background_5 / neutral_foreground_1 |
//! | Informative  | neutral_foreground_2 / neutral_background_1            | neutral_foreground_2             | neutral_stroke_2 / neutral_foreground_2 | neutral_background_3 / neutral_foreground_2 |
//! | Severe       | status_warning_background_3 / status_warning_foreground_inverted | status_warning_foreground_1 | status_warning_border_1 / status_warning_foreground_1 | status_warning_background_1 / status_warning_foreground_2 |
//! | Subtle       | neutral_background_5 / neutral_foreground_3            | neutral_foreground_3             | neutral_stroke_2 / neutral_foreground_3 | neutral_background_3 / neutral_foreground_3 |
//! | Success      | status_success_background_3 / status_success_foreground_inverted | status_success_foreground_1 | status_success_border_1 / status_success_foreground_1 | status_success_background_1 / status_success_foreground_1 |
//! | Warning      | status_warning_background_3 / status_warning_foreground_inverted | status_warning_foreground_2 | status_warning_border_2 / status_warning_foreground_2 | status_warning_background_1 / status_warning_foreground_2 |
//!
//! Ghost has no fill. Outline has no fill. Tint and filled paint a fill.
//! Severe and Warning share the warning status ramp; they differ in which
//! foreground step they pick.

use egui::{
    CornerRadius, Response, Sense, Stroke, TextStyle, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType,
};

use fluent2_tokens::Rgba;
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::foundation::text::TypeRamp;
use crate::theme::{self, Theme};

/// Fill treatment of a badge. [`Tint`] is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BadgeAppearance {
    /// Solid status fill.
    Filled,
    /// Foreground only.
    Ghost,
    /// Stroke and foreground, no fill.
    Outline,
    /// Soft status fill. The default.
    #[default]
    Tint,
}

/// Status colour of a badge. [`Brand`] is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BadgeColor {
    /// Brand ramp.
    #[default]
    Brand,
    /// Danger status.
    Danger,
    /// Highest neutral emphasis.
    Important,
    /// Secondary neutral emphasis.
    Informative,
    /// Warning status, stronger foreground.
    Severe,
    /// Lowest emphasis.
    Subtle,
    /// Success status.
    Success,
    /// Warning status.
    Warning,
}

/// Height of a badge. [`Medium`] is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum BadgeSize {
    /// Caption2, tighter padding.
    Small,
    /// Caption1. The default.
    #[default]
    Medium,
    /// Body1.
    Large,
}

/// A non-interactive Fluent badge.
///
/// ```
/// # use fluent2_egui::controls::{Badge, BadgeAppearance, BadgeColor};
/// # fn demo(ui: &mut egui::Ui) {
/// let _ = ui.add(Badge::new("3").appearance(BadgeAppearance::Filled).color(BadgeColor::Danger));
/// # }
/// ```
pub struct Badge<'a> {
    text: WidgetText,
    appearance: BadgeAppearance,
    color: BadgeColor,
    size: BadgeSize,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> Badge<'a> {
    /// A badge whose accessible name is `text`.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            appearance: BadgeAppearance::Tint,
            color: BadgeColor::Brand,
            size: BadgeSize::Medium,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Fill treatment. Defaults to [`BadgeAppearance::Tint`].
    #[must_use]
    pub fn appearance(mut self, appearance: BadgeAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Status colour. Defaults to [`BadgeColor::Brand`].
    #[must_use]
    pub fn color(mut self, color: BadgeColor) -> Self {
        self.color = color;
        self
    }

    /// Size. Defaults to [`BadgeSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: BadgeSize) -> Self {
        self.size = size;
        self
    }
}

impl Widget for Badge<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let metrics = Metrics::of(self.size);
        let pair = colors_of(self.appearance, self.color, &theme);
        let galley = self.text.clone().color(theme::c(pair.foreground)).into_galley(
            ui,
            Some(egui::TextWrapMode::Extend),
            f32::INFINITY,
            metrics.text_style(),
        );
        let width = finite(galley.size().x + metrics.h_padding * 2.0).max(metrics.min_height);
        let desired = Vec2::new(width, metrics.min_height);
        let (rect, response) = ui.allocate_exact_size(desired, Sense::hover());
        if ui.is_rect_visible(rect) {
            let radius = CornerRadius::same(rounded_u8(border_radius::CIRCULAR.min(rect.height() / 2.0)));
            let fill = pair
                .background
                .map(theme::c)
                .unwrap_or(egui::Color32::TRANSPARENT);
            let stroke = Stroke::new(
                if pair.stroke.is_some() {
                    stroke_width::THIN
                } else {
                    0.0
                },
                pair.stroke.map(theme::c).unwrap_or(egui::Color32::TRANSPARENT),
            );
            ui.painter()
                .rect(rect, radius, fill, stroke, egui::StrokeKind::Inside);
            let text_pos = egui::pos2(
                rect.center().x - galley.size().x * 0.5,
                rect.center().y - galley.size().y * 0.5,
            );
            ui.painter().galley(text_pos, galley, theme::c(pair.foreground));
        }
        let label = self.text.text().to_owned();
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, label.clone()));
        response
    }
}

struct Pair {
    background: Option<Rgba>,
    foreground: Rgba,
    stroke: Option<Rgba>,
}

fn colors_of(appearance: BadgeAppearance, color: BadgeColor, theme: &Theme) -> Pair {
    let c = &theme.color;
    let (filled_bg, filled_fg, ghost_fg, outline, tint_bg, tint_fg) = match color {
        BadgeColor::Brand => (
            c.brand_background,
            c.neutral_foreground_on_brand,
            c.brand_foreground_1,
            c.brand_stroke_1,
            c.brand_background_2,
            c.brand_foreground_1,
        ),
        BadgeColor::Danger => (
            c.status_danger_background_3,
            c.status_danger_foreground_inverted,
            c.status_danger_foreground_1,
            c.status_danger_border_1,
            c.status_danger_background_1,
            c.status_danger_foreground_1,
        ),
        BadgeColor::Important => (
            c.neutral_foreground_1,
            c.neutral_background_1,
            c.neutral_foreground_1,
            c.neutral_stroke_accessible,
            c.neutral_background_5,
            c.neutral_foreground_1,
        ),
        BadgeColor::Informative => (
            c.neutral_foreground_2,
            c.neutral_background_1,
            c.neutral_foreground_2,
            c.neutral_stroke_2,
            c.neutral_background_3,
            c.neutral_foreground_2,
        ),
        BadgeColor::Severe => (
            c.status_warning_background_3,
            c.status_warning_foreground_inverted,
            c.status_warning_foreground_1,
            c.status_warning_border_1,
            c.status_warning_background_1,
            c.status_warning_foreground_2,
        ),
        BadgeColor::Subtle => (
            c.neutral_background_5,
            c.neutral_foreground_3,
            c.neutral_foreground_3,
            c.neutral_stroke_2,
            c.neutral_background_3,
            c.neutral_foreground_3,
        ),
        BadgeColor::Success => (
            c.status_success_background_3,
            c.status_success_foreground_inverted,
            c.status_success_foreground_1,
            c.status_success_border_1,
            c.status_success_background_1,
            c.status_success_foreground_1,
        ),
        BadgeColor::Warning => (
            c.status_warning_background_3,
            c.status_warning_foreground_inverted,
            c.status_warning_foreground_2,
            c.status_warning_border_2,
            c.status_warning_background_1,
            c.status_warning_foreground_2,
        ),
    };
    match appearance {
        BadgeAppearance::Filled => Pair {
            background: Some(filled_bg),
            foreground: filled_fg,
            stroke: None,
        },
        BadgeAppearance::Ghost => Pair {
            background: None,
            foreground: ghost_fg,
            stroke: None,
        },
        BadgeAppearance::Outline => Pair {
            background: None,
            foreground: outline_fg(color, theme),
            stroke: Some(outline),
        },
        BadgeAppearance::Tint => Pair {
            background: Some(tint_bg),
            foreground: tint_fg,
            stroke: None,
        },
    }
}

fn outline_fg(color: BadgeColor, theme: &Theme) -> Rgba {
    let c = &theme.color;
    match color {
        BadgeColor::Brand => c.brand_foreground_1,
        BadgeColor::Danger => c.status_danger_foreground_1,
        BadgeColor::Important => c.neutral_foreground_1,
        BadgeColor::Informative => c.neutral_foreground_2,
        BadgeColor::Severe => c.status_warning_foreground_1,
        BadgeColor::Subtle => c.neutral_foreground_3,
        BadgeColor::Success => c.status_success_foreground_1,
        BadgeColor::Warning => c.status_warning_foreground_2,
    }
}

struct Metrics {
    min_height: f32,
    h_padding: f32,
    ramp: TypeRamp,
}

impl Metrics {
    fn of(size: BadgeSize) -> Self {
        match size {
            BadgeSize::Small => Self {
                min_height: 16.0,
                h_padding: spacing_horizontal::XS,
                ramp: TypeRamp::Caption2,
            },
            BadgeSize::Medium => Self {
                min_height: 20.0,
                h_padding: spacing_horizontal::S,
                ramp: TypeRamp::Caption1,
            },
            BadgeSize::Large => Self {
                min_height: 24.0,
                h_padding: spacing_horizontal::M,
                ramp: TypeRamp::Body1,
            },
        }
    }

    fn text_style(&self) -> TextStyle {
        TextStyle::Name(self.ramp.name().into())
    }
}

fn ensure_type_ramp(ui: &mut Ui) {
    use crate::foundation::text::{font_id, style};
    if ui.style().text_styles.contains_key(&style(TypeRamp::Caption1)) {
        return;
    }
    let styles = &mut ui.style_mut().text_styles;
    for step in TypeRamp::ALL {
        styles.insert(style(step), font_id(step));
    }
}

fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() {
        return 0;
    }
    value.round().clamp(0.0, f32::from(u8::MAX)) as u8
}
