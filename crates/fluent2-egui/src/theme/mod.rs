//! Fluent theme: token sets installed as egui visuals.

use egui::CornerRadius;

use crate::foundation::text;
use fluent2_tokens::generated::global;
use fluent2_tokens::{AliasColors, BrandRamp, Rgba, Shadows};

/// Which Fluent theme is installed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ThemeKind {
    /// Light theme.
    Light,
    /// Dark theme.
    Dark,
    /// High contrast theme.
    HighContrast,
}

/// A Fluent theme: which kind, its alias colours, its shadows and the brand ramp it was built
/// with.
#[derive(Clone, Debug, PartialEq)]
pub struct Theme {
    /// Light, dark or high contrast.
    pub kind: ThemeKind,
    /// Alias colours for [`Self::kind`].
    pub color: AliasColors,
    /// Elevation shadows for [`Self::kind`].
    pub shadow: Shadows,
    /// Brand ramp the theme was built with.
    pub brand: BrandRamp,
}

impl Theme {
    /// The light theme for a brand ramp.
    #[must_use]
    pub fn light(brand: BrandRamp) -> Self {
        Self {
            kind: ThemeKind::Light,
            color: fluent2_tokens::light(&brand),
            shadow: fluent2_tokens::generated::web_light::SHADOWS,
            brand,
        }
    }

    /// The dark theme for a brand ramp.
    #[must_use]
    pub fn dark(brand: BrandRamp) -> Self {
        Self {
            kind: ThemeKind::Dark,
            color: fluent2_tokens::dark(&brand),
            shadow: fluent2_tokens::generated::web_dark::SHADOWS,
            brand,
        }
    }

    /// The high contrast theme. It has no brand ramp of its own.
    #[must_use]
    pub fn high_contrast() -> Self {
        Self {
            kind: ThemeKind::HighContrast,
            color: fluent2_tokens::high_contrast(),
            shadow: fluent2_tokens::generated::high_contrast::SHADOWS,
            brand: fluent2_tokens::FLUENT,
        }
    }

    /// The theme last passed to [`install`], or [`Theme::default`] when none has.
    pub fn get(ctx: &egui::Context) -> Self {
        ctx.data(|data| data.get_temp(theme_id())).unwrap_or_default()
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::light(fluent2_tokens::FLUENT)
    }
}

/// `Rgba` as an unmultiplied [`egui::Color32`].
#[must_use]
pub fn c(rgba: Rgba) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(rgba.r, rgba.g, rgba.b, rgba.a)
}

fn theme_id() -> egui::Id {
    egui::Id::NULL.with("fluent2_theme")
}

/// Install `theme` as the egui style and remember it for [`Theme::get`].
///
/// Both of egui's built-in themes receive the same visuals, so a later egui theme switch does
/// not drop the Fluent mapping. High contrast is treated as egui's dark theme when its
/// `neutral_background_1` is dark.
pub fn install(ctx: &egui::Context, theme: &Theme) {
    text::install_system_fonts(ctx);
    text::install_text_styles(ctx);
    let visuals = visuals_of(theme);
    let spacing = spacing_of();
    for egui_theme in [egui::Theme::Light, egui::Theme::Dark] {
        ctx.style_mut_of(egui_theme, |style| {
            style.visuals = visuals.clone();
            style.spacing = spacing.clone();
        });
    }
    ctx.set_theme(egui_preference(theme));
    ctx.data_mut(|data| data.insert_temp(theme_id(), theme.clone()));
}

fn egui_preference(theme: &Theme) -> egui::ThemePreference {
    match theme.kind {
        ThemeKind::Light => egui::ThemePreference::Light,
        ThemeKind::Dark => egui::ThemePreference::Dark,
        ThemeKind::HighContrast => {
            if is_dark(theme.color.neutral_background_1) {
                egui::ThemePreference::Dark
            } else {
                egui::ThemePreference::Light
            }
        }
    }
}

/// Whether a background reads as dark. Luminance is unweighted; the call only picks which of
/// egui's two themes high contrast follows.
fn is_dark(rgba: Rgba) -> bool {
    let sum = u16::from(rgba.r) + u16::from(rgba.g) + u16::from(rgba.b);
    sum < 3 * 128
}

fn spacing_of() -> egui::Spacing {
    let mut spacing = egui::Spacing {
        item_spacing: egui::vec2(global::spacing_horizontal::S, global::spacing_vertical::S),
        button_padding: egui::vec2(global::spacing_horizontal::M, global::spacing_vertical::XS),
        ..Default::default()
    };
    spacing.interact_size.y = 32.0;
    spacing
}

fn visuals_of(theme: &Theme) -> egui::Visuals {
    let t = &theme.color;
    let mut visuals = egui::Visuals::dark();
    visuals.dark_mode = theme.kind != ThemeKind::Light;
    visuals.panel_fill = c(t.neutral_background_2);
    visuals.window_fill = c(t.neutral_background_1);
    visuals.window_stroke = stroke(t.neutral_stroke_2);
    visuals.faint_bg_color = c(t.neutral_background_3);
    visuals.extreme_bg_color = c(t.neutral_background_1);
    visuals.code_bg_color = c(t.neutral_background_4);
    visuals.hyperlink_color = c(t.brand_foreground_link);
    visuals.warn_fg_color = c(t.status_warning_foreground_1);
    visuals.error_fg_color = c(t.status_danger_foreground_1);
    visuals.selection.bg_fill = c(t.brand_background_2);
    visuals.selection.stroke = stroke(t.brand_stroke_1);
    visuals.override_text_color = None;

    widget(
        &mut visuals.widgets.noninteractive,
        t.neutral_background_1,
        t.neutral_stroke_2,
        t.neutral_foreground_1,
    );
    widget(
        &mut visuals.widgets.inactive,
        t.neutral_background_1,
        t.neutral_stroke_1,
        t.neutral_foreground_1,
    );
    widget(
        &mut visuals.widgets.hovered,
        t.neutral_background_1_hover,
        t.neutral_stroke_1_hover,
        t.neutral_foreground_1,
    );
    widget(
        &mut visuals.widgets.active,
        t.neutral_background_1_pressed,
        t.neutral_stroke_1_pressed,
        t.neutral_foreground_1,
    );
    widget(
        &mut visuals.widgets.open,
        t.neutral_background_1_selected,
        t.neutral_stroke_1,
        t.neutral_foreground_1,
    );

    let medium = radius(global::border_radius::MEDIUM);
    for widget_visuals in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget_visuals.corner_radius = medium;
    }
    visuals.window_corner_radius = radius(global::border_radius::X_LARGE);
    visuals.menu_corner_radius = medium;

    visuals.window_shadow = shadow_of(theme.shadow.shadow_64);
    visuals.popup_shadow = shadow_of(theme.shadow.shadow_16);
    visuals
}

fn widget(visuals: &mut egui::style::WidgetVisuals, bg: Rgba, bg_stroke: Rgba, fg: Rgba) {
    visuals.bg_fill = c(bg);
    visuals.weak_bg_fill = c(bg);
    visuals.bg_stroke = stroke(bg_stroke);
    visuals.fg_stroke = stroke(fg);
}

fn stroke(rgba: Rgba) -> egui::Stroke {
    egui::Stroke::new(1.0, c(rgba))
}

fn radius(points: f32) -> CornerRadius {
    CornerRadius::same(rounded_u8(points))
}

/// The layer with the largest blur. An empty shadow is no shadow.
fn shadow_of(layers: &[fluent2_tokens::ShadowLayer]) -> egui::Shadow {
    let Some(layer) = layers.iter().max_by(|left, right| {
        left.blur
            .total_cmp(&right.blur)
            .then(left.spread.total_cmp(&right.spread))
    }) else {
        return egui::Shadow::NONE;
    };
    egui::Shadow {
        offset: [rounded_i8(layer.x), rounded_i8(layer.y)],
        blur: rounded_u8(layer.blur),
        spread: rounded_u8(layer.spread),
        color: c(layer.color),
    }
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

fn rounded_i8(value: f32) -> i8 {
    if !value.is_finite() {
        0
    } else if value >= f32::from(i8::MAX) {
        i8::MAX
    } else if value <= f32::from(i8::MIN) {
        i8::MIN
    } else {
        value.round() as i8
    }
}
