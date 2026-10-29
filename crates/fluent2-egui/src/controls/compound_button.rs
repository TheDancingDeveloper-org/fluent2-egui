//! Fluent CompoundButton: a button with a primary line and a secondary line.
//!
//! Padding is `spacingHorizontalM` / `spacingVerticalM`. The optional icon is 24 px (or a 40 px
//! box) on the left. Colours are the same faces as [`super::button::Button`].

use egui::epaint::RectShape;
use egui::{
    CornerRadius, Id, Rect, Response, Role, Sense, Stroke, StrokeKind, Ui, Vec2, Widget, WidgetInfo,
    WidgetText,
};

use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use super::super::foundation::focus;
use super::super::foundation::text::TypeRamp;
use super::button::{Appearance, Shape, colors_of, ensure_type_ramp, visual_state};
use crate::theme::{Theme, c};

/// A two-line button.
///
/// ```
/// # use fluent2_egui::controls::CompoundButton;
/// # fn demo(ui: &mut egui::Ui) {
/// ui.add(CompoundButton::new("Meeting").secondary_content("Starts in 5 minutes"));
/// # }
/// ```
pub struct CompoundButton<'a> {
    text: WidgetText,
    secondary: Option<WidgetText>,
    icon: Option<Icon>,
    appearance: Appearance,
    shape: Shape,
    id_salt: Option<Id>,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

impl<'a> CompoundButton<'a> {
    /// A compound button with a primary line.
    pub fn new(text: impl Into<WidgetText>) -> Self {
        Self {
            text: text.into(),
            secondary: None,
            icon: None,
            appearance: Appearance::Secondary,
            shape: Shape::Rounded,
            id_salt: None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// The second line, drawn in `caption1` and `neutral_foreground_2`.
    #[must_use]
    pub fn secondary_content(mut self, text: impl Into<WidgetText>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// A 24 px icon on the left. Pass an icon whose box is 40 px when the caller wants that.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Visual weight. Defaults to [`Appearance::Secondary`]. Same set as [`super::button::Button`].
    #[must_use]
    pub fn appearance(mut self, appearance: Appearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Corner treatment. Defaults to [`Shape::Rounded`].
    #[must_use]
    pub fn shape(mut self, shape: Shape) -> Self {
        self.shape = shape;
        self
    }

    /// Extra salt for the widget id.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::unique(salt));
        self
    }
}

const ICON_PX: f32 = 24.0;
/// Room beside a large icon. The spec allows an S24 glyph or a 40 px box.
const ICON_BOX: f32 = 40.0;

impl Widget for CompoundButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        ensure_type_ramp(ui);
        let enabled = ui.is_enabled();
        let h_pad = spacing_horizontal::M;
        let v_pad = spacing_vertical::M;
        let icon_slot = if self.icon.is_some() { ICON_BOX } else { 0.0 };
        let gap = if self.icon.is_some() {
            spacing_horizontal::S_NUDGE
        } else {
            0.0
        };
        let text_budget = (ui.available_width() - h_pad * 2.0 - icon_slot - gap).max(0.0);

        let primary_style = crate::foundation::text::style(TypeRamp::Body1Strong);
        let secondary_style = crate::foundation::text::style(TypeRamp::Caption1);
        let primary =
            self.text
                .clone()
                .into_galley(ui, Some(egui::TextWrapMode::Truncate), text_budget, primary_style);
        let secondary = self.secondary.as_ref().map(|text| {
            text.clone().into_galley(
                ui,
                Some(egui::TextWrapMode::Truncate),
                text_budget,
                secondary_style,
            )
        });
        // Reserve the Fluent line heights (body1 20, caption1 16). A galley measured before
        // the type ramp is installed is only the font size (14 and 12), which is too short.
        let lines = TypeRamp::Body1Strong.line_height()
            + if secondary.is_some() {
                TypeRamp::Caption1.line_height()
            } else {
                0.0
            };
        let height = (lines + v_pad * 2.0).max(icon_slot + v_pad * 2.0);
        let text_w = primary
            .size()
            .x
            .max(secondary.as_ref().map(|g| g.size().x).unwrap_or(0.0));
        let width = (text_w + icon_slot + gap + h_pad * 2.0).max(96.0);
        let desired = Vec2::new(finite(width), finite(height));

        let (auto_id, rect) = ui.allocate_space(desired);
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => auto_id,
        };
        let mut response = ui.interact(rect, id, Sense::click());
        let state = visual_state(enabled, response.hovered(), response.is_pointer_button_down_on());
        let face = colors_of(self.appearance, state, &theme.color);

        if ui.is_rect_visible(rect) {
            let radius = corner_radius(self.shape, rect);
            ui.painter().add(RectShape::new(
                rect,
                radius,
                c(face.background),
                Stroke::new(stroke_width::THIN, c(face.stroke)),
                StrokeKind::Inside,
            ));
            let mut x = rect.left() + h_pad;
            if let Some(icon) = self.icon {
                let origin = egui::pos2(x + (ICON_BOX - ICON_PX) / 2.0, rect.center().y - ICON_PX / 2.0);
                let icon_rect = Rect::from_min_size(origin, Vec2::splat(ICON_PX));
                crate::foundation::icon::paint_icon(
                    ui,
                    icon_rect,
                    icon,
                    IconSize::S24,
                    IconStyle::Regular,
                    Some(c(face.foreground)),
                );
                x += ICON_BOX + gap;
            }
            let primary_row = TypeRamp::Body1Strong.line_height();
            let block_top = rect.center().y - lines / 2.0;
            ui.painter()
                .galley(egui::pos2(x, block_top), primary.clone(), c(face.foreground));
            if let Some(secondary) = &secondary {
                // The second line stays `neutral_foreground_2` unless the button is disabled.
                let secondary_color = if enabled {
                    theme.color.neutral_foreground_2
                } else {
                    theme.color.neutral_foreground_disabled
                };
                ui.painter().galley(
                    egui::pos2(x, block_top + primary_row),
                    secondary.clone(),
                    c(secondary_color),
                );
            }
            if focus::focus_visible(ui, &response) {
                focus::paint_focus_ring(ui, rect, radius);
            }
        }

        let accessible = self.text.text().to_owned();
        if primary.elided {
            response = response.on_hover_text(accessible.clone());
        }
        response.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, accessible.clone()));
        response
    }
}

fn corner_radius(shape: Shape, rect: Rect) -> CornerRadius {
    let points = match shape {
        Shape::Square => border_radius::NONE,
        Shape::Circular => rect.height().max(rect.width()) / 2.0,
        Shape::Rounded => border_radius::LARGE,
    };
    let value = if !points.is_finite() || points <= 0.0 {
        0.0
    } else {
        points.min(f32::from(u8::MAX))
    };
    CornerRadius::same(value.round() as u8)
}

fn finite(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}
