//! Fluent MessageBar.
//!
//! Info uses `neutral_background_3` and `neutral_stroke_1` with the Info icon. Success, Warning
//! and Error use `status_*_background_1`, `status_*_border_1` and `status_*_foreground_1` with
//! Checkmark, Warning and ErrorCircle. Radius is `borderRadiusMedium`, minimum height 36,
//! padding 12. When the row is narrower than the content, the actions wrap onto a second line.

use egui::epaint::RectShape;
use egui::{CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, Vec2, WidgetText};

use fluent2_icons::Icon;
use fluent2_tokens::generated::global::{border_radius, stroke_width};

use crate::controls::{Appearance, Button, ButtonSize, Shape};
use crate::foundation::focus;
use crate::foundation::icon::{self, IconSize, IconStyle};
use crate::foundation::text::{TypeRamp, font_id, style};
use crate::theme::{Theme, c};

/// Semantic weight of a message bar. Toasts use the same weights.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Intent {
    /// Neutral information. The default.
    #[default]
    Info,
    /// A completed action.
    Success,
    /// A caution that is not an error.
    Warning,
    /// A failure.
    Error,
}

/// What the user did to a message bar this frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MessageBarResponse {
    /// Which action was clicked, if any. The first action is index `0`.
    pub action_clicked: Option<usize>,
    /// The dismiss control was clicked.
    pub dismissed: bool,
}

/// A Fluent message bar.
///
/// ```
/// # use fluent2_egui::surfaces::{Intent, MessageBar};
/// # fn demo(ui: &mut egui::Ui) {
/// let response = MessageBar::new(Intent::Warning, "Unsaved changes")
///     .title("Document")
///     .action("Save")
///     .dismissible()
///     .show(ui);
/// if response.dismissed {
///     // the app decides whether the bar stays
/// }
/// # }
/// ```
pub struct MessageBar {
    intent: Intent,
    text: WidgetText,
    title: Option<WidgetText>,
    actions: Vec<WidgetText>,
    dismissible: bool,
    id_salt: Option<Id>,
}

impl MessageBar {
    /// A message bar for `intent` whose body is `text`.
    pub fn new(intent: Intent, text: impl Into<WidgetText>) -> Self {
        Self {
            intent,
            text: text.into(),
            title: None,
            actions: Vec::new(),
            dismissible: false,
            id_salt: None,
        }
    }

    /// A short heading before the body.
    #[must_use]
    pub fn title(mut self, title: impl Into<WidgetText>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Add an action button. The first action is reported as index `0`.
    #[must_use]
    pub fn action(mut self, label: impl Into<WidgetText>) -> Self {
        self.actions.push(label.into());
        self
    }

    /// Show a dismiss control. The bar does not hide itself; the app does.
    #[must_use]
    pub fn dismissible(mut self) -> Self {
        self.dismissible = true;
        self
    }

    /// Extra salt so two bars with the same text can coexist.
    #[must_use]
    pub fn id_salt(mut self, salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        self.id_salt = Some(Id::new(salt));
        self
    }

    /// Paint the bar and report action clicks and dismissal.
    pub fn show(self, ui: &mut Ui) -> MessageBarResponse {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let palette = palette_of(self.intent, &theme);
        let padding = 12.0;
        let icon_px = 20.0;
        let gap = 8.0;
        let min_height = 36.0;
        let action_height = 32.0;
        let available = finite_positive(ui.available_width());

        let body_font = font_id(TypeRamp::Body1);
        let title_style = style(TypeRamp::Body1Strong);

        let action_widths: Vec<f32> = self
            .actions
            .iter()
            .map(|label| {
                let text_w = label
                    .clone()
                    .into_galley(ui, None, f32::INFINITY, body_font.clone())
                    .size()
                    .x;
                (text_w + 24.0).clamp(64.0, 240.0)
            })
            .collect();
        let dismiss_w = if self.dismissible { action_height } else { 0.0 };
        let actions_w = action_widths.iter().sum::<f32>()
            + gap * action_widths.len().saturating_sub(1) as f32
            + if self.dismissible && !action_widths.is_empty() {
                gap
            } else {
                0.0
            }
            + dismiss_w;
        let unwrapped_title_w = self
            .title
            .as_ref()
            .map(|title| {
                title
                    .clone()
                    .into_galley(ui, None, f32::INFINITY, title_style.clone())
                    .size()
                    .x
                    + gap
            })
            .unwrap_or(0.0);
        let unwrapped_body_w = self
            .text
            .clone()
            .into_galley(ui, None, f32::INFINITY, body_font.clone())
            .size()
            .x;
        let row_w = padding * 2.0 + icon_px + gap + unwrapped_title_w + unwrapped_body_w + gap + actions_w;
        let wrap = row_w > available;

        let text_budget = if wrap {
            (available - padding * 2.0 - icon_px - gap).max(1.0)
        } else {
            (available - padding * 2.0 - icon_px - gap * 2.0 - actions_w).max(1.0)
        };
        let title_galley = self.title.as_ref().map(|title| {
            title.clone().color(palette.foreground).into_galley(
                ui,
                Some(egui::TextWrapMode::Wrap),
                text_budget,
                title_style,
            )
        });
        let body_galley = self.text.clone().color(palette.foreground).into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            text_budget,
            body_font,
        );

        let title_gap = if title_galley.is_some() { 2.0 } else { 0.0 };
        let text_height =
            title_galley.as_ref().map(|g| g.size().y).unwrap_or(0.0) + title_gap + body_galley.size().y;
        let actions_height = if self.actions.is_empty() && !self.dismissible {
            0.0
        } else {
            action_height
        };
        let height = if wrap {
            (padding + text_height.max(icon_px) + gap + actions_height + padding).max(min_height)
        } else {
            (text_height + padding * 2.0).max(min_height)
        };

        let (auto_id, rect) = ui.allocate_space(Vec2::new(available, finite_positive(height)));
        let id = self
            .id_salt
            .map(|salt| ui.make_persistent_id(salt))
            .unwrap_or(auto_id);
        let response = ui.interact(rect, id, Sense::hover());

        if ui.is_rect_visible(rect) {
            let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
            ui.painter().add(RectShape::new(
                rect,
                radius,
                palette.background,
                Stroke::new(stroke_width::THIN, palette.stroke),
                StrokeKind::Inside,
            ));
            let icon_rect = Rect::from_min_size(
                egui::pos2(rect.left() + padding, rect.top() + padding),
                Vec2::splat(icon_px),
            );
            icon::paint_icon(
                ui,
                icon_rect,
                palette.icon,
                IconSize::S20,
                IconStyle::Regular,
                Some(palette.foreground),
            );
            let text_x = icon_rect.right() + gap;
            let mut y = rect.top() + padding;
            if let Some(galley) = &title_galley {
                let title_h = galley.size().y;
                ui.painter()
                    .galley(egui::pos2(text_x, y), galley.clone(), palette.foreground);
                y += title_h + title_gap;
            }
            ui.painter()
                .galley(egui::pos2(text_x, y), body_galley, palette.foreground);
            if focus::focus_visible(ui, &response) {
                focus::paint_focus_ring(ui, rect, radius);
            }
        }

        let action_top = if wrap {
            rect.bottom() - padding - action_height
        } else {
            rect.center().y - action_height / 2.0
        };
        let mut x = if wrap {
            rect.left() + padding + icon_px + gap
        } else {
            rect.right() - padding - actions_w
        };

        let mut action_clicked = None;
        for (index, label) in self.actions.iter().enumerate() {
            let width = action_widths.get(index).copied().unwrap_or(64.0);
            let slot = Rect::from_min_size(egui::pos2(x, action_top), Vec2::new(width, action_height));
            let label_text = label.text().to_owned();
            let clicked = ui
                .scope_builder(egui::UiBuilder::new().max_rect(slot), |ui| {
                    ui.add_sized(
                        slot.size(),
                        Button::new(label_text)
                            .appearance(Appearance::Transparent)
                            .size(ButtonSize::Medium),
                    )
                    .clicked()
                })
                .inner;
            if clicked {
                action_clicked = Some(index);
            }
            x += width + gap;
        }

        let mut dismissed = false;
        if self.dismissible {
            let slot = Rect::from_min_size(egui::pos2(x, action_top), Vec2::splat(action_height));
            dismissed = ui
                .scope_builder(egui::UiBuilder::new().max_rect(slot), |ui| {
                    ui.add_sized(
                        slot.size(),
                        Button::icon_only(Icon::Dismiss, "Dismiss")
                            .appearance(Appearance::Transparent)
                            .size(ButtonSize::Medium)
                            .shape(Shape::Rounded),
                    )
                    .clicked()
                })
                .inner;
        }

        let label = self
            .title
            .as_ref()
            .map(|title| format!("{}. {}", title.text(), self.text.text()))
            .unwrap_or_else(|| self.text.text().to_owned());
        let (role, live) = match self.intent {
            Intent::Warning | Intent::Error => {
                (egui::accesskit::Role::Alert, egui::accesskit::Live::Assertive)
            }
            Intent::Info | Intent::Success => (egui::accesskit::Role::Status, egui::accesskit::Live::Polite),
        };
        // Set the role directly. `widget_info` would overwrite it with a plain label.
        ui.ctx().accesskit_node_builder(id, |node| {
            node.set_role(role);
            node.set_live(live);
            node.set_label(label.clone());
        });

        MessageBarResponse {
            action_clicked,
            dismissed,
        }
    }
}

/// Colours and icon for one intent. Shared with the toaster.
pub(crate) struct Palette {
    pub background: egui::Color32,
    pub stroke: egui::Color32,
    pub foreground: egui::Color32,
    pub icon: Icon,
}

pub(crate) fn palette_of(intent: Intent, theme: &Theme) -> Palette {
    let color = &theme.color;
    match intent {
        Intent::Info => Palette {
            background: c(color.neutral_background_3),
            stroke: c(color.neutral_stroke_1),
            foreground: c(color.neutral_foreground_1),
            icon: Icon::Info,
        },
        Intent::Success => Palette {
            background: c(color.status_success_background_1),
            stroke: c(color.status_success_border_1),
            foreground: c(color.status_success_foreground_1),
            icon: Icon::Checkmark,
        },
        Intent::Warning => Palette {
            background: c(color.status_warning_background_1),
            stroke: c(color.status_warning_border_1),
            foreground: c(color.status_warning_foreground_1),
            icon: Icon::Warning,
        },
        Intent::Error => Palette {
            background: c(color.status_danger_background_1),
            stroke: c(color.status_danger_border_1),
            foreground: c(color.status_danger_foreground_1),
            icon: Icon::ErrorCircle,
        },
    }
}

/// Make sure the Fluent text styles exist before a galley asks for them.
///
/// `install` writes them. Without it, the strong steps fall back to the proportional family
/// when the semibold face is not loaded, so layout does not panic. Mirrors the private helper
/// in `controls::button`.
pub(crate) fn ensure_type_ramp(ui: &mut Ui) {
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
    if !value.is_finite() {
        return 0;
    }
    value.clamp(0.0, 255.0).round() as u8
}

fn finite_positive(value: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        1.0
    }
}
