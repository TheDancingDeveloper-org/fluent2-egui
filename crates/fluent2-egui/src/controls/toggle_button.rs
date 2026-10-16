//! Fluent ToggleButton: a button that stays pressed.
//!
//! Painting is [`super::button::paint_button`]. Checked colours follow the WI-1543 spec:
//! secondary and outline use `neutral_background_1_selected` with `neutral_stroke_1`, primary
//! uses `brand_background_selected`, and subtle and transparent use `subtle_background_selected`
//! with `neutral_foreground_2_brand_selected`.

use egui::{Id, Response, Sense, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType};

use fluent2_icons::Icon;

use super::button::{
    Appearance, ButtonContent, ButtonSize, ButtonStyle, Shape, ensure_type_ramp, paint_button, visual_state,
};
use crate::theme::Theme;

/// A button whose checked state is owned by the caller.
///
/// ```
/// # use fluent2_egui::controls::{Appearance, ToggleButton};
/// # fn demo(ui: &mut egui::Ui, checked: &mut bool) {
/// ui.add(ToggleButton::new(checked, "Bold").appearance(Appearance::Primary));
/// # }
/// ```
pub struct ToggleButton<'a> {
    checked: &'a mut bool,
    text: Option<WidgetText>,
    accessible_label: Option<String>,
    icon: Option<Icon>,
    icon_after: bool,
    appearance: Appearance,
    size: ButtonSize,
    shape: Shape,
    id_salt: Option<Id>,
}

impl<'a> ToggleButton<'a> {
    /// A text toggle. `checked` is flipped when the control is activated.
    pub fn new(checked: &'a mut bool, text: impl Into<WidgetText>) -> Self {
        Self {
            checked,
            text: Some(text.into()),
            accessible_label: None,
            icon: None,
            icon_after: false,
            appearance: Appearance::Secondary,
            size: ButtonSize::Medium,
            shape: Shape::Rounded,
            id_salt: None,
        }
    }

    /// An icon with no visible text. `accessible_label` is the accessible name.
    pub fn icon_only(checked: &'a mut bool, icon: Icon, accessible_label: impl Into<String>) -> Self {
        Self {
            checked,
            text: None,
            accessible_label: Some(accessible_label.into()),
            icon: Some(icon),
            icon_after: false,
            appearance: Appearance::Secondary,
            size: ButtonSize::Medium,
            shape: Shape::Rounded,
            id_salt: None,
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

    /// Extra salt for the widget id, so two toggles with the same label can coexist.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    fn label_string(&self) -> String {
        self.accessible_label.clone().unwrap_or_else(|| {
            self.text
                .as_ref()
                .map(|text| text.text().to_owned())
                .unwrap_or_default()
        })
    }
}

impl Widget for ToggleButton<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        ensure_type_ramp(ui);
        let metrics = super::button::Metrics::of(self.size);
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
        let min_width = if self.text.is_none() {
            metrics.min_height
        } else {
            metrics.min_width
        };
        let width = (content + metrics.h_padding * 2.0).max(min_width);
        let desired = Vec2::new(finite(width), metrics.min_height);

        let (auto_id, rect) = ui.allocate_space(desired);
        let id = match self.id_salt {
            Some(salt) => ui.make_persistent_id(salt),
            None => auto_id,
        };
        let mut response = ui.interact(rect, id, Sense::click());
        if response.clicked() && enabled {
            *self.checked = !*self.checked;
            response.mark_changed();
        }
        let content_width = (rect.width() - metrics.h_padding * 2.0 - icon_px - gap).max(0.0);
        let galley = self.text.as_ref().map(|text| {
            text.clone().into_galley(
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
                checked: *self.checked,
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
        let selected = *self.checked;
        response
            .widget_info(|| WidgetInfo::selected(WidgetType::Button, enabled, selected, accessible.clone()));
        response
    }
}

fn finite(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        0.0
    }
}
