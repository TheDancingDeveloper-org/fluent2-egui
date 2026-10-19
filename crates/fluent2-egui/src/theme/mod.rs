//! Fluent theme: token sets installed as egui visuals.

use egui::CornerRadius;

mod density;

pub(crate) use density::of as density_of;
pub use density::{Density, TOUCH_TARGET};

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
    /// Office chrome sizes. These are not Fluent tokens, so they are the same for every kind.
    pub chrome: ChromeMetrics,
    /// Data view sizes (DataGrid, TreeView, List, NavigationView). Not Fluent tokens, so they
    /// are the same for every kind. See `docs/specs/data-views.md` §10 and D6.
    pub data: DataMetrics,
    /// Compact, medium or touch sizing (`docs/specs/density.md`). Medium by default. Change it
    /// with [`Self::with_density`], which also resizes [`Self::data`].
    pub density: Density,
}

/// Sizes of the Office chrome, in points. Fixed by `docs/specs/office-chrome.md` §10 (D2).
///
/// ```
/// let metrics = fluent2_egui::theme::ChromeMetrics::default();
/// assert_eq!(metrics.title_bar_height, 32.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChromeMetrics {
    /// Title bar height with no QAT and no search box.
    pub title_bar_height: f32,
    /// Title bar height with a QAT or a search box.
    pub title_bar_height_extended: f32,
    /// Width of one caption button.
    pub caption_button_width: f32,
    /// Height of one caption button.
    pub caption_button_height: f32,
    /// Thickness of the resize zone along each window edge (§2). The top of the title bar
    /// belongs to this zone, not to the drag region.
    pub resize_zone: f32,
    /// Space left empty at the left of the title bar on macOS for the traffic-light buttons.
    /// Windows and Linux use no inset (§1).
    pub macos_traffic_light_inset: f32,
    /// Height of the ribbon tab strip.
    pub tab_strip_height: f32,
    /// Height of the ribbon body, including group labels.
    pub ribbon_body_height: f32,
    /// Gap between ribbon groups.
    pub group_separator: f32,
    /// Gap between CommandBar items.
    pub command_bar_gap: f32,
    /// Hysteresis band used when scaling the ribbon and CommandBar (§5.3).
    pub hysteresis: f32,
    /// Height of the status bar.
    pub status_bar_height: f32,
    /// Width of the Backstage navigation column.
    pub backstage_nav_width: f32,
    /// Side of a KeyTip badge for one letter.
    pub keytip_badge: f32,
    /// Small icon size.
    pub icon_small: f32,
    /// Large icon size.
    pub icon_large: f32,
    /// Maximum width of a large ribbon button label.
    pub large_label_max: f32,
    /// Side of a collapsed ribbon button.
    pub collapsed_button: f32,
}

impl Default for ChromeMetrics {
    fn default() -> Self {
        Self {
            title_bar_height: 32.0,
            title_bar_height_extended: 40.0,
            caption_button_width: 46.0,
            caption_button_height: 32.0,
            resize_zone: 6.0,
            macos_traffic_light_inset: 78.0,
            tab_strip_height: 32.0,
            ribbon_body_height: 96.0,
            group_separator: 8.0,
            command_bar_gap: 4.0,
            hysteresis: 8.0,
            status_bar_height: 24.0,
            backstage_nav_width: 240.0,
            keytip_badge: 16.0,
            icon_small: 16.0,
            icon_large: 32.0,
            large_label_max: 72.0,
            collapsed_button: 36.0,
        }
    }
}

/// Sizes of the data views, in points. Fixed by `docs/specs/data-views.md` §10 (D6).
///
/// ```
/// let metrics = fluent2_egui::theme::DataMetrics::default();
/// assert_eq!(metrics.grid_row_height_medium, 44.0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DataMetrics {
    /// DataGrid row height at `DataGridSize::Medium`.
    pub grid_row_height_medium: f32,
    /// DataGrid row height at `DataGridSize::Small`.
    pub grid_row_height_small: f32,
    /// DataGrid row height at `DataGridSize::ExtraSmall`.
    pub grid_row_height_extra_small: f32,
    /// DataGrid header row height.
    pub grid_header_height: f32,
    /// Width of the selection checkbox column in Multi selection mode.
    pub selection_cell_width: f32,
    /// TreeView row height at Medium.
    pub tree_row_height_medium: f32,
    /// TreeView row height at Small.
    pub tree_row_height_small: f32,
    /// List row height.
    pub list_row_height: f32,
    /// NavigationView pane width when expanded.
    pub nav_pane_width_expanded: f32,
    /// NavigationView pane width when compact.
    pub nav_pane_width_compact: f32,
    /// Window width below which the NavigationView pane is compact (§7.2).
    pub nav_breakpoint_expanded: f32,
    /// Window width below which the NavigationView pane is an overlay (§7.2).
    pub nav_breakpoint_compact: f32,
    /// NavigationView item height.
    pub nav_item_height: f32,
}

impl DataMetrics {
    /// The default metrics with row heights and hit targets for `density`. Pane widths and
    /// breakpoints do not change.
    #[must_use]
    pub fn for_density(density: Density) -> Self {
        let base = Self::default();
        let row = |h: f32| density.row(h);
        Self {
            grid_row_height_medium: row(base.grid_row_height_medium),
            grid_row_height_small: row(base.grid_row_height_small),
            grid_row_height_extra_small: row(base.grid_row_height_extra_small),
            grid_header_height: row(base.grid_header_height),
            selection_cell_width: base.selection_cell_width.max(row(0.0)),
            tree_row_height_medium: row(base.tree_row_height_medium),
            tree_row_height_small: row(base.tree_row_height_small),
            list_row_height: row(base.list_row_height),
            nav_item_height: row(base.nav_item_height),
            ..base
        }
    }
}

impl Default for DataMetrics {
    fn default() -> Self {
        Self {
            grid_row_height_medium: 44.0,
            grid_row_height_small: 34.0,
            grid_row_height_extra_small: 24.0,
            grid_header_height: 32.0,
            selection_cell_width: 44.0,
            tree_row_height_medium: 32.0,
            tree_row_height_small: 24.0,
            list_row_height: 32.0,
            nav_pane_width_expanded: 260.0,
            nav_pane_width_compact: 48.0,
            nav_breakpoint_expanded: 1008.0,
            nav_breakpoint_compact: 641.0,
            nav_item_height: 40.0,
        }
    }
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
            chrome: ChromeMetrics::default(),
            data: DataMetrics::default(),
            density: Density::Medium,
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
            chrome: ChromeMetrics::default(),
            data: DataMetrics::default(),
            density: Density::Medium,
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
            chrome: ChromeMetrics::default(),
            data: DataMetrics::default(),
            density: Density::Medium,
        }
    }

    /// This theme at `density`. Data-view row heights follow ([`DataMetrics::for_density`]).
    ///
    /// ```
    /// use fluent2_egui::Theme;
    /// use fluent2_egui::theme::Density;
    ///
    /// let touch = Theme::default().with_density(Density::Touch);
    /// assert_eq!(touch.data.list_row_height, 44.0);
    /// ```
    #[must_use]
    pub fn with_density(mut self, density: Density) -> Self {
        self.density = density;
        self.data = DataMetrics::for_density(density);
        self
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
    let spacing = spacing_of(theme.density);
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

fn spacing_of(density: Density) -> egui::Spacing {
    let mut spacing = egui::Spacing {
        item_spacing: density.item_spacing(),
        button_padding: egui::vec2(global::spacing_horizontal::M, global::spacing_vertical::XS),
        ..Default::default()
    };
    spacing.interact_size.y = density.height(32.0);
    if density == Density::Touch {
        spacing.interact_size.x = spacing.interact_size.x.max(TOUCH_TARGET);
    }
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
