//! Fluent SplitButton.
//!
//! A SplitButton is a Button-styled main region plus a separate chevron region. The menu
//! popup is framed by [`crate::surfaces::menu_frame`] and filled by the caller.
//! [`super::menu_button::MenuButton`] shares the metrics and painting helpers below.
//!
//! Behaviour follows Fluent UI React SplitButton / MenuButton (the revision named on
//! WI-1544) and the split hit regions described there. Nothing is copied from a reference
//! tree. Colours, radii and spacing come from the installed [`Theme`].

use egui::{
    CornerRadius, Id, Key, Modifiers, Popup, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui, Vec2,
    WidgetInfo, WidgetText, WidgetType,
};

use fluent2_icons::{Icon, Size as IconSize};
use fluent2_tokens::Rgba;
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use super::super::foundation::focus;
use super::super::foundation::text::TypeRamp;
use super::button::{Appearance, ButtonSize, Shape};
use crate::theme::{Theme, c};

/// What [`SplitButton::show`] returns.
pub struct SplitButtonResponse {
    /// The main action region.
    pub main: Response,
    /// The chevron region that opens the menu.
    pub menu: Response,
    /// Whether the menu popup is open this frame.
    pub menu_open: bool,
}

/// A button split into a main action and a menu chevron.
///
/// ```
/// # use fluent2_egui::controls::{Appearance, SplitButton};
/// # fn demo(ui: &mut egui::Ui) {
/// let response = SplitButton::new("Save")
///     .appearance(Appearance::Primary)
///     .show(ui, |ui| {
///         ui.label("Save as…");
///     });
/// if response.main.clicked() {
///     // the app decides what Save does
/// }
/// # }
/// ```
pub struct SplitButton<'a> {
    text: WidgetText,
    icon: Option<Icon>,
    appearance: Appearance,
    size: ButtonSize,
    shape: Shape,
    id_salt: Option<Id>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> SplitButton<'a> {
    /// A split button labelled `text`. The label is the accessible name of the main region.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            icon: None,
            appearance: Appearance::Secondary,
            size: ButtonSize::Medium,
            shape: Shape::Rounded,
            id_salt: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Place an icon before the label.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
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

    /// Extra salt so two split buttons with the same label can coexist.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Show the split button and, when the chevron has opened it, the menu popup.
    ///
    /// `menu` draws the popup contents. It runs only while the menu is open.
    pub fn show(self, ui: &mut Ui, menu: impl FnOnce(&mut Ui)) -> SplitButtonResponse {
        let theme = Theme::get(ui.ctx());
        ensure_type_ramp(ui);
        let metrics = Metrics::of(self.size).at(crate::theme::density_of(ui));
        let enabled = ui.is_enabled();
        let label = self.text.text().to_owned();

        let (id_main, id_menu) = split_ids(ui, self.id_salt);
        let width = natural_width(ui, &self.text, self.icon.is_some(), metrics);
        let desired = Vec2::new(finite(width + metrics.chevron), metrics.min_height);
        let rect = ui.allocate_exact_size(desired, Sense::hover()).0;
        let (main_rect, menu_rect) = split_rects(rect, metrics.chevron);

        let main = ui.interact(main_rect, id_main, Sense::click());
        let menu_response = ui.interact(menu_rect, id_menu, Sense::click());

        let popup_id = Popup::default_response_id(&menu_response);
        let open_key = menu_response.has_focus()
            && ui.input_mut(|input| {
                input.consume_key(Modifiers::ALT, Key::ArrowDown)
                    || input.consume_key(Modifiers::NONE, Key::Enter)
            });
        if open_key || menu_response.clicked() {
            Popup::toggle_id(ui.ctx(), popup_id);
        }

        let menu_open = Popup::is_id_open(ui.ctx(), popup_id);
        if ui.input(|input| input.key_pressed(Key::Escape)) && menu_open {
            Popup::close_id(ui.ctx(), popup_id);
            menu_response.request_focus();
        }

        if ui.is_rect_visible(rect) {
            let radius = corner_radius(self.size, self.shape, rect);
            paint_region(
                ui,
                main_rect,
                main_corners(radius),
                face_of(self.appearance, enabled, &main, &theme),
            );
            paint_region(
                ui,
                menu_rect,
                menu_corners(radius),
                face_of(self.appearance, enabled, &menu_response, &theme),
            );
            paint_divider(ui, main_rect, self.appearance, &theme);
            paint_main_content(
                ui,
                main_rect,
                &self,
                metrics,
                face_of(self.appearance, enabled, &main, &theme),
            );
            paint_chevron(
                ui,
                menu_rect,
                metrics,
                face_of(self.appearance, enabled, &menu_response, &theme),
            );
            if focus::focus_visible(ui, &main) {
                focus::paint_focus_ring(ui, main_rect, main_corners(radius));
            }
            if focus::focus_visible(ui, &menu_response) {
                focus::paint_focus_ring(ui, menu_rect, menu_corners(radius));
            }
        }

        main.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &label));
        let options = format!("{label} options");
        menu_response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &options));
        let expanded = menu_open;
        ui.ctx().accesskit_node_builder(id_menu, |node| {
            node.set_expanded(expanded);
        });

        if menu_open && enabled {
            let _ = Popup::menu(&menu_response).open_memory(None).show(|popup| {
                crate::surfaces::menu_frame(popup).show(popup, menu);
            });
        }

        SplitButtonResponse {
            main,
            menu: menu_response,
            menu_open,
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Metrics {
    pub(super) min_height: f32,
    pub(crate) min_width: f32,
    pub(crate) h_padding: f32,
    pub(crate) icon: f32,
    pub(crate) icon_gap: f32,
    /// Chevron hit region: the icon box of the size (24 / 32 / 40).
    chevron: f32,
    ramp: TypeRamp,
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
                chevron: 24.0,
                ramp: TypeRamp::Caption1,
            },
            ButtonSize::Medium => Self {
                min_height: 32.0,
                min_width: 96.0,
                h_padding: spacing_horizontal::M,
                icon: 20.0,
                icon_gap: spacing_horizontal::S_NUDGE,
                chevron: 32.0,
                ramp: TypeRamp::Body1Strong,
            },
            ButtonSize::Large => Self {
                min_height: 40.0,
                min_width: 96.0,
                h_padding: spacing_horizontal::L,
                icon: 24.0,
                icon_gap: spacing_horizontal::S_NUDGE,
                chevron: 40.0,
                ramp: TypeRamp::Subtitle2,
            },
        }
    }

    /// These metrics at `density`: the height and the chevron box change.
    pub(crate) fn at(mut self, density: crate::theme::Density) -> Self {
        self.min_height = density.height(self.min_height);
        self.chevron = density.height(self.chevron);
        self
    }

    pub(super) fn text_style(&self) -> TextStyle {
        TextStyle::Name(self.ramp.name().into())
    }

    pub(super) fn icon_size(&self) -> IconSize {
        match self.icon as u32 {
            16 => IconSize::S16,
            24 => IconSize::S24,
            _ => IconSize::S20,
        }
    }

    /// Trailing chevron glyph: 16 px on small, 20 px otherwise.
    pub(super) fn chevron_glyph(&self) -> (IconSize, f32) {
        if self.min_height <= 24.0 {
            (IconSize::S16, 16.0)
        } else {
            (IconSize::S20, 20.0)
        }
    }
}

#[derive(Clone, Copy)]
pub(super) struct Face {
    pub(super) foreground: Rgba,
    pub(super) background: Rgba,
    pub(super) stroke: Rgba,
}

pub(super) fn face_of(appearance: Appearance, enabled: bool, response: &Response, theme: &Theme) -> Face {
    let color = &theme.color;
    let clear = Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    };
    let pressed = response.is_pointer_button_down_on();
    let hovered = response.hovered();
    if !enabled {
        return Face {
            foreground: color.neutral_foreground_disabled,
            background: color.neutral_background_disabled,
            stroke: color.neutral_stroke_disabled,
        };
    }
    use Appearance::{Outline, Primary, Secondary, Subtle, Transparent};
    match (appearance, hovered, pressed) {
        (Secondary, _, true) => Face {
            foreground: color.neutral_foreground_1_pressed,
            background: color.neutral_background_1_pressed,
            stroke: color.neutral_stroke_1_pressed,
        },
        (Secondary, true, false) => Face {
            foreground: color.neutral_foreground_1_hover,
            background: color.neutral_background_1_hover,
            stroke: color.neutral_stroke_1_hover,
        },
        (Secondary, false, false) => Face {
            foreground: color.neutral_foreground_1,
            background: color.neutral_background_1,
            stroke: color.neutral_stroke_1,
        },
        (Primary, _, true) => Face {
            foreground: color.neutral_foreground_on_brand,
            background: color.brand_background_pressed,
            stroke: clear,
        },
        (Primary, true, false) => Face {
            foreground: color.neutral_foreground_on_brand,
            background: color.brand_background_hover,
            stroke: clear,
        },
        (Primary, false, false) => Face {
            foreground: color.neutral_foreground_on_brand,
            background: color.brand_background,
            stroke: clear,
        },
        (Outline, _, true) => Face {
            foreground: color.neutral_foreground_1,
            background: clear,
            stroke: color.neutral_stroke_1_pressed,
        },
        (Outline, true, false) => Face {
            foreground: color.neutral_foreground_1,
            background: clear,
            stroke: color.neutral_stroke_1_hover,
        },
        (Outline, false, false) => Face {
            foreground: color.neutral_foreground_1,
            background: clear,
            stroke: color.neutral_stroke_1,
        },
        (Subtle, _, true) => Face {
            foreground: color.neutral_foreground_2_pressed,
            background: color.subtle_background_pressed,
            stroke: clear,
        },
        (Subtle, true, false) => Face {
            foreground: color.neutral_foreground_2_hover,
            background: color.subtle_background_hover,
            stroke: clear,
        },
        (Subtle, false, false) => Face {
            foreground: color.neutral_foreground_2,
            background: color.subtle_background,
            stroke: clear,
        },
        (Transparent, _, true) => Face {
            foreground: color.neutral_foreground_2_brand_pressed,
            background: clear,
            stroke: clear,
        },
        (Transparent, true, false) => Face {
            foreground: color.neutral_foreground_2_brand_hover,
            background: clear,
            stroke: clear,
        },
        (Transparent, false, false) => Face {
            foreground: color.neutral_foreground_2,
            background: clear,
            stroke: clear,
        },
    }
}

pub(super) fn paint_region(ui: &Ui, rect: Rect, radius: CornerRadius, face: Face) {
    ui.painter().rect(
        rect,
        radius,
        c(face.background),
        Stroke::new(stroke_width::THIN, c(face.stroke)),
        StrokeKind::Inside,
    );
}

fn paint_divider(ui: &Ui, main: Rect, appearance: Appearance, theme: &Theme) {
    let color = if appearance == Appearance::Primary {
        c(theme.color.neutral_foreground_on_brand).gamma_multiply(0.3)
    } else {
        c(theme.color.neutral_stroke_1)
    };
    let x = main.right();
    ui.painter().vline(x, main.y_range(), Stroke::new(1.0, color));
}

fn paint_main_content(ui: &Ui, rect: Rect, button: &SplitButton<'_>, metrics: Metrics, face: Face) {
    let foreground = c(face.foreground);
    let icon_px = if button.icon.is_some() { metrics.icon } else { 0.0 };
    let galley = button.text.clone().into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        (rect.width() - metrics.h_padding * 2.0 - icon_px - metrics.icon_gap).max(0.0),
        metrics.text_style(),
    );
    let gap = if icon_px > 0.0 { metrics.icon_gap } else { 0.0 };
    let total = icon_px + gap + galley.size().x;
    let mut x = rect.center().x - total / 2.0;
    if let Some(icon) = button.icon {
        paint_glyph(
            ui,
            icon,
            metrics.icon_size(),
            foreground,
            egui::pos2(x, rect.center().y - icon_px / 2.0),
            icon_px,
        );
        x += icon_px + gap;
    }
    let pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, foreground);
}

fn paint_chevron(ui: &Ui, rect: Rect, metrics: Metrics, face: Face) {
    let (glyph, px) = metrics.chevron_glyph();
    let origin = egui::pos2(rect.center().x - px / 2.0, rect.center().y - px / 2.0);
    paint_glyph(ui, Icon::ChevronDown, glyph, c(face.foreground), origin, px);
}

pub(super) fn paint_glyph(
    ui: &Ui,
    icon: Icon,
    size: IconSize,
    color: egui::Color32,
    origin: egui::Pos2,
    px: f32,
) {
    let rect = Rect::from_min_size(origin, Vec2::splat(finite(px)));
    crate::foundation::icon::paint_icon(ui, rect, icon, size, fluent2_icons::Style::Regular, Some(color));
}

fn split_ids(ui: &Ui, salt: Option<Id>) -> (Id, Id) {
    let base = salt.unwrap_or_else(|| ui.next_auto_id());
    (base.with("main"), base.with("menu"))
}

fn split_rects(rect: Rect, chevron: f32) -> (Rect, Rect) {
    let chevron = finite(chevron).min(rect.width());
    let split = rect.right() - chevron;
    let main = Rect::from_min_max(rect.min, egui::pos2(split, rect.bottom()));
    let menu = Rect::from_min_max(egui::pos2(split, rect.top()), rect.max);
    (main, menu)
}

pub(crate) fn natural_width(ui: &Ui, text: &WidgetText, has_icon: bool, metrics: Metrics) -> f32 {
    let text_w = text
        .clone()
        .into_galley(ui, None, f32::INFINITY, metrics.text_style())
        .size()
        .x;
    let icon = if has_icon {
        metrics.icon + metrics.icon_gap
    } else {
        0.0
    };
    (text_w + icon + metrics.h_padding * 2.0).max(metrics.min_width)
}

pub(super) fn corner_radius(size: ButtonSize, shape: Shape, rect: Rect) -> CornerRadius {
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

fn main_corners(radius: CornerRadius) -> CornerRadius {
    CornerRadius {
        ne: 0,
        se: 0,
        ..radius
    }
}

fn menu_corners(radius: CornerRadius) -> CornerRadius {
    CornerRadius {
        nw: 0,
        sw: 0,
        ..radius
    }
}

pub(crate) fn ensure_type_ramp(ui: &mut Ui) {
    use crate::foundation::text::{font_id, style};
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
