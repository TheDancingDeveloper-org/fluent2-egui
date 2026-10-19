//! Fluent Button. This is the reference control: later controls follow the same shape.
//!
//! Metrics and colours follow Fluent UI React `useButtonStyles` (the revision pinned in
//! the WI-1542 description). Sizes, radii, spacing and colours come from the installed
//! [`Theme`]; nothing here is a literal colour.
//!
//! Icons are painted by [`crate::foundation::icon::paint_icon`]. The focus ring is
//! [`crate::foundation::focus`].

use egui::epaint::{Color32, RectShape};
use egui::{
    CornerRadius, Galley, Id, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2, Widget,
    WidgetInfo, WidgetText, WidgetType,
};

use fluent2_icons::{Icon, Size as IconSize};
use fluent2_tokens::Rgba;
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use super::super::foundation::focus;
use super::super::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

/// Visual weight of a button. [`Secondary`](Self::Secondary) is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Appearance {
    /// Neutral fill and a stroke. The default.
    #[default]
    Secondary,
    /// Brand fill, no stroke.
    Primary,
    /// Transparent fill and a stroke.
    Outline,
    /// Subtle fill, no stroke.
    Subtle,
    /// No fill and no stroke until hover.
    Transparent,
}

/// Height, padding and type step of a button. [`Medium`](Self::Medium) is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ButtonSize {
    /// 24 px, caption1, 8 px horizontal padding.
    Small,
    /// 32 px, body1 strong, 12 px horizontal padding. The default.
    #[default]
    Medium,
    /// 40 px, subtitle2, 16 px horizontal padding.
    Large,
}

/// Corner treatment. [`Rounded`](Self::Rounded) uses the size's Fluent radius.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Shape {
    /// Size-specific radius (`SMALL` / `MEDIUM` / `LARGE`). The default.
    #[default]
    Rounded,
    /// `borderRadiusCircular`.
    Circular,
    /// `borderRadiusNone`.
    Square,
}

/// A Fluent button.
///
/// ```
/// # use fluent2_egui::controls::{Appearance, Button};
/// # use fluent2_icons::Icon;
/// # fn demo(ui: &mut egui::Ui) {
/// if ui.add(Button::new("Save").appearance(Appearance::Primary).icon(Icon::Save)).clicked() {
///     // the app decides what Save does
/// }
/// let _ = ui.add(Button::icon_only(Icon::Dismiss, "Close"));
/// # }
/// ```
pub struct Button<'a> {
    text: Option<WidgetText>,
    accessible_label: Option<String>,
    icon: Option<Icon>,
    icon_after: bool,
    appearance: Appearance,
    size: ButtonSize,
    shape: Shape,
    id_salt: Option<Id>,
    min_width: Option<f32>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> Button<'a> {
    /// A text button. The label is also the accessible name.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: Some(text.into()),
            accessible_label: None,
            icon: None,
            icon_after: false,
            appearance: Appearance::Secondary,
            size: ButtonSize::Medium,
            shape: Shape::Rounded,
            id_salt: None,
            min_width: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// An icon with no visible text. `accessible_label` is the accessible name.
    pub fn icon_only(icon: Icon, accessible_label: impl Into<String>) -> Self {
        Self {
            text: None,
            accessible_label: Some(accessible_label.into()),
            icon: Some(icon),
            icon_after: false,
            appearance: Appearance::Secondary,
            size: ButtonSize::Medium,
            shape: Shape::Rounded,
            id_salt: None,
            min_width: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Place an icon before the label (or after it once [`Self::icon_after`] is set).
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Draw the icon after the label.
    #[must_use]
    pub fn icon_after(mut self) -> Self {
        self.icon_after = true;
        self
    }

    /// Visual weight. Defaults to [`Appearance::Secondary`].
    #[must_use]
    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Size. Defaults to [`ButtonSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Corner treatment. Defaults to [`Shape::Rounded`].
    #[must_use]
    pub fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    /// Extra salt for the widget id, so two buttons with the same label can coexist.
    ///
    /// The id is never derived from the label alone when a salt is given.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Override the size's minimum width (64 or 96 pt for text buttons), for compact buttons
    /// such as Pager's page numbers. The height and the content width are kept.
    #[must_use]
    pub fn min_width(mut self, width: f32) -> Self {
        self.min_width = Some(width);
        self
    }
}

impl Widget for Button<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        ensure_type_ramp(ui);
        let metrics = Metrics::of(self.size);
        let enabled = ui.is_enabled();
        let has_icon = self.icon.is_some();
        let icon_px = if has_icon { metrics.icon } else { 0.0 };
        let gap = if has_icon && self.text.is_some() {
            metrics.icon_gap
        } else {
            0.0
        };
        let content_budget = (ui.available_width() - metrics.h_padding * 2.0 - icon_px - gap).max(0.0);
        let natural = self
            .text
            .as_ref()
            .map(|text| {
                text.clone()
                    .into_galley(ui, None, f32::INFINITY, metrics.text_style())
                    .size()
                    .x
            })
            .unwrap_or(0.0);
        let text_width = natural.min(content_budget);
        let content = text_width + icon_px + gap;
        let min_width = match self.min_width {
            Some(width) if width.is_finite() => width.max(0.0),
            _ if self.text.is_none() => metrics.min_height,
            _ => metrics.min_width,
        };
        let width = (content + metrics.h_padding * 2.0).max(min_width);
        let desired = Vec2::new(finite(width), metrics.min_height);

        // One widget. `allocate_space` returns the id egui focuses. Interacting with a second
        // id, or allocating with `Sense::click` and then interacting again, adds an invisible
        // Tab stop. A salt replaces that automatic id so the caller can name the button; with
        // no salt the automatic id keeps two identical labels independent.
        let (auto_id, rect) = ui.allocate_space(desired);
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => auto_id,
        };
        let mut response = ui.interact(rect, id, Sense::click());
        let content_width = (rect.width() - metrics.h_padding * 2.0 - icon_px - gap).max(0.0);
        let galley = self.text.as_ref().map(|text| {
            text.clone()
                .color(c(colors_of(
                    self.appearance,
                    visual_state(enabled, response.hovered(), response.is_pointer_button_down_on()),
                    &theme.color,
                )
                .foreground))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    content_width,
                    metrics.text_style(),
                )
        });
        let state = visual_state(enabled, response.hovered(), response.is_pointer_button_down_on());
        paint_button(
            ui,
            rect,
            &response,
            &ButtonStyle {
                appearance: self.appearance,
                size: self.size,
                shape: self.shape,
                state,
                checked: false,
            },
            &ButtonContent {
                icon: self.icon,
                icon_after: self.icon_after,
                galley: galley.as_deref(),
                metrics,
            },
            &theme,
        );

        let accessible = self.label_string();
        if galley.as_ref().is_some_and(|galley| galley.elided) {
            response = response.on_hover_text(accessible.clone());
        }
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, accessible.clone()));
        response
    }
}

/// Face, size and checked state for [`paint_button`].
pub(crate) struct ButtonStyle {
    /// Visual weight.
    pub appearance: Appearance,
    /// Height step.
    pub size: ButtonSize,
    /// Corner treatment.
    pub shape: Shape,
    /// Rest, hover, pressed or disabled.
    pub state: VisualState,
    /// When set, rest uses the Fluent selected colours (ToggleButton).
    pub checked: bool,
}

/// Icon and label already laid out by the caller.
pub(crate) struct ButtonContent<'a> {
    /// Optional catalogue icon.
    pub icon: Option<Icon>,
    /// Draw the icon after the label.
    pub icon_after: bool,
    /// Laid-out label, if any.
    pub galley: Option<&'a Galley>,
    /// Size metrics.
    pub metrics: Metrics,
}

/// Paint a button face, its content and the focus ring.
pub(crate) fn paint_button(
    ui: &Ui,
    rect: Rect,
    response: &Response,
    style: &ButtonStyle,
    content: &ButtonContent<'_>,
    theme: &Theme,
) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let colors = if style.checked {
        checked_colors(style.appearance, style.state, &theme.color)
    } else {
        colors_of(style.appearance, style.state, &theme.color)
    };
    let radius = corner_radius(style.size, style.shape, rect);
    ui.painter().add(RectShape::new(
        rect,
        radius,
        c(colors.background),
        Stroke::new(stroke_width::THIN, c(colors.stroke)),
        StrokeKind::Inside,
    ));
    paint_content(ui, rect, content, c(colors.foreground));
    if focus::focus_visible(ui, response) {
        focus::paint_focus_ring(ui, rect, radius);
    }
}

/// ToggleButton selected colours. Disabled stays the ordinary disabled face.
pub(crate) fn checked_colors(
    appearance: Appearance,
    state: VisualState,
    color: &fluent2_tokens::generated::colors::AliasColors,
) -> Face {
    use Appearance::{Outline, Primary, Secondary, Subtle, Transparent};
    if matches!(state, VisualState::Disabled) {
        return colors_of(appearance, state, color);
    }
    let clear = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    match appearance {
        Secondary | Outline => Face {
            foreground: color.neutral_foreground_1_selected,
            background: color.neutral_background_1_selected,
            stroke: color.neutral_stroke_1,
        },
        Primary => Face {
            foreground: color.neutral_foreground_on_brand,
            background: color.brand_background_selected,
            stroke: clear,
        },
        Subtle | Transparent => Face {
            foreground: color.neutral_foreground_2_brand_selected,
            background: color.subtle_background_selected,
            stroke: clear,
        },
    }
}

pub(crate) fn ensure_type_ramp(ui: &mut Ui) {
    use crate::foundation::text::{font_id, style};
    // Strong steps name the `fluent2-semibold` family. That family exists only after
    // `install_system_fonts` loaded a face. Until then, draw the same sizes in the
    // proportional family so layout does not panic.
    let semibold_ready = ui.fonts(|fonts| {
        fonts.families().iter().any(|family| {
            matches!(family, egui::FontFamily::Name(name) if name.as_ref() == crate::foundation::text::SEMIBOLD_FAMILY)
        })
    });
    if semibold_ready && ui.style().text_styles.contains_key(&style(TypeRamp::Body1Strong)) {
        return;
    }
    let styles = &mut ui.style_mut().text_styles;
    for step in TypeRamp::ALL {
        let mut id = font_id(step);
        if !semibold_ready {
            id.family = egui::FontFamily::Proportional;
        }
        styles.insert(style(step), id);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Metrics {
    pub min_height: f32,
    pub min_width: f32,
    pub h_padding: f32,
    pub icon: f32,
    pub icon_gap: f32,
    pub ramp: TypeRamp,
}

impl Metrics {
    pub(crate) fn of(size: ButtonSize) -> Self {
        match size {
            ButtonSize::Small => Self {
                min_height: 24.0,
                min_width: 64.0,
                h_padding: spacing_horizontal::S,
                icon: 16.0,
                icon_gap: spacing_horizontal::XS,
                ramp: TypeRamp::Caption1,
            },
            ButtonSize::Medium => Self {
                min_height: 32.0,
                min_width: 96.0,
                h_padding: spacing_horizontal::M,
                icon: 20.0,
                icon_gap: spacing_horizontal::S_NUDGE,
                ramp: TypeRamp::Body1Strong,
            },
            ButtonSize::Large => Self {
                min_height: 40.0,
                min_width: 96.0,
                h_padding: spacing_horizontal::L,
                icon: 24.0,
                icon_gap: spacing_horizontal::S_NUDGE,
                ramp: TypeRamp::Subtitle2,
            },
        }
    }

    pub(crate) fn text_style(&self) -> TextStyle {
        TextStyle::Name(self.ramp.name().into())
    }

    fn icon_size(&self) -> IconSize {
        match self.icon as u32 {
            16 => IconSize::S16,
            24 => IconSize::S24,
            _ => IconSize::S20,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum VisualState {
    Rest,
    Hover,
    Pressed,
    Disabled,
}

pub(crate) fn visual_state(enabled: bool, hovered: bool, pressed: bool) -> VisualState {
    if !enabled {
        VisualState::Disabled
    } else if pressed {
        VisualState::Pressed
    } else if hovered {
        VisualState::Hover
    } else {
        VisualState::Rest
    }
}

pub(crate) struct Face {
    pub foreground: Rgba,
    pub background: Rgba,
    pub stroke: Rgba,
}

pub(crate) fn colors_of(
    appearance: Appearance,
    state: VisualState,
    color: &fluent2_tokens::generated::colors::AliasColors,
) -> Face {
    use Appearance::{Outline, Primary, Secondary, Subtle, Transparent};
    let clear = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    match (appearance, state) {
        (_, VisualState::Disabled) => Face {
            foreground: color.neutral_foreground_disabled,
            background: match appearance {
                Primary | Secondary => color.neutral_background_disabled,
                Outline | Subtle | Transparent => clear,
            },
            // Primary keeps a stroke so a disabled primary button still has an edge in high
            // contrast, where the disabled fill is the window colour (as SplitButton does).
            stroke: match appearance {
                Primary | Secondary | Outline => color.neutral_stroke_disabled,
                Subtle | Transparent => clear,
            },
        },
        (Secondary, VisualState::Rest) => Face {
            foreground: color.neutral_foreground_1,
            background: color.neutral_background_1,
            stroke: color.neutral_stroke_1,
        },
        (Secondary, VisualState::Hover) => Face {
            foreground: color.neutral_foreground_1_hover,
            background: color.neutral_background_1_hover,
            stroke: color.neutral_stroke_1_hover,
        },
        (Secondary, VisualState::Pressed) => Face {
            foreground: color.neutral_foreground_1_pressed,
            background: color.neutral_background_1_pressed,
            stroke: color.neutral_stroke_1_pressed,
        },
        (Primary, VisualState::Rest) => Face {
            foreground: color.neutral_foreground_on_brand,
            background: color.brand_background,
            stroke: clear,
        },
        (Primary, VisualState::Hover) => Face {
            foreground: color.neutral_foreground_on_brand,
            background: color.brand_background_hover,
            stroke: clear,
        },
        (Primary, VisualState::Pressed) => Face {
            foreground: color.neutral_foreground_on_brand,
            background: color.brand_background_pressed,
            stroke: clear,
        },
        (Outline, VisualState::Rest) => Face {
            foreground: color.neutral_foreground_1,
            background: clear,
            stroke: color.neutral_stroke_1,
        },
        (Outline, VisualState::Hover) => Face {
            foreground: color.neutral_foreground_1,
            background: clear,
            stroke: color.neutral_stroke_1_hover,
        },
        (Outline, VisualState::Pressed) => Face {
            foreground: color.neutral_foreground_1,
            background: clear,
            stroke: color.neutral_stroke_1_pressed,
        },
        (Subtle, VisualState::Rest) => Face {
            foreground: color.neutral_foreground_2,
            background: color.subtle_background,
            stroke: clear,
        },
        (Subtle, VisualState::Hover) => Face {
            foreground: color.neutral_foreground_2_hover,
            background: color.subtle_background_hover,
            stroke: clear,
        },
        (Subtle, VisualState::Pressed) => Face {
            foreground: color.neutral_foreground_2_pressed,
            background: color.subtle_background_pressed,
            stroke: clear,
        },
        (Transparent, VisualState::Rest) => Face {
            foreground: color.neutral_foreground_2,
            background: clear,
            stroke: clear,
        },
        (Transparent, VisualState::Hover) => Face {
            foreground: color.neutral_foreground_2_brand_hover,
            background: clear,
            stroke: clear,
        },
        (Transparent, VisualState::Pressed) => Face {
            foreground: color.neutral_foreground_2_brand_pressed,
            background: clear,
            stroke: clear,
        },
    }
}

fn corner_radius(size: ButtonSize, shape: Shape, rect: Rect) -> CornerRadius {
    let points = match shape {
        Shape::Square => border_radius::NONE,
        Shape::Circular => rect.height().max(rect.width()) / 2.0,
        Shape::Rounded => match size {
            ButtonSize::Small => border_radius::SMALL,
            ButtonSize::Medium => border_radius::MEDIUM,
            ButtonSize::Large => border_radius::LARGE,
        },
    };
    CornerRadius::same(rounded_u8(points))
}

fn paint_content(ui: &Ui, rect: Rect, content: &ButtonContent<'_>, foreground: Color32) {
    let metrics = content.metrics;
    let icon_px = if content.icon.is_some() { metrics.icon } else { 0.0 };
    let text_w = content.galley.map(|g| g.size().x).unwrap_or(0.0);
    let gap = if icon_px > 0.0 && content.galley.is_some() {
        metrics.icon_gap
    } else {
        0.0
    };
    let total = icon_px + gap + text_w;
    let mut x = rect.center().x - total / 2.0;
    let icon_first = !content.icon_after;
    if icon_first && let Some(icon) = content.icon {
        paint_icon(
            ui,
            icon,
            metrics.icon_size(),
            foreground,
            egui::pos2(x, rect.center().y - icon_px / 2.0),
            icon_px,
        );
        x += icon_px + gap;
    }
    if let Some(galley) = content.galley {
        let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
        ui.painter()
            .galley(pos, std::sync::Arc::new(galley.clone()), foreground);
        x += text_w + gap;
    }
    if !icon_first && let Some(icon) = content.icon {
        paint_icon(
            ui,
            icon,
            metrics.icon_size(),
            foreground,
            egui::pos2(x, rect.center().y - icon_px / 2.0),
            icon_px,
        );
    }
}

/// Draw one catalogue icon. Replace with the icon widget from WI-1536 when it lands.
fn paint_icon(ui: &Ui, icon: Icon, size: IconSize, color: Color32, origin: egui::Pos2, px: f32) {
    let rect = Rect::from_min_size(origin, Vec2::splat(finite(px)));
    crate::foundation::icon::paint_icon(ui, rect, icon, size, fluent2_icons::Style::Regular, Some(color));
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

fn finite(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}

impl Button<'_> {
    fn label_string(&self) -> String {
        self.accessible_label.clone().unwrap_or_else(|| {
            self.text
                .as_ref()
                .map(|text| text.text().to_owned())
                .unwrap_or_default()
        })
    }
}
