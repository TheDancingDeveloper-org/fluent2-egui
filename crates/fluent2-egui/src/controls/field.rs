//! Fluent Field: a label, a child, an optional hint and a validation message.
//!
//! The label is body1. A required field appends `" *"` in
//! `status_danger_foreground_1`. The hint is caption1 in `neutral_foreground_3`.
//! The validation line uses the matching status foreground and a status icon
//! (`ErrorCircle`, `Warning` or `Checkmark`).
//!
//! # Accessible name
//!
//! egui 0.36's [`egui::Widget`] does not accept a name from its parent, and
//! AccessKit node builders are not exposed on the public [`egui::Context`].
//! The field therefore announces its own label (with the required marker) as
//! a label widget. The child keeps the accessible name its own widget sets.
//! Callers that need the child named should pass that label into the child.

use egui::{Sense, TextStyle, Ui, WidgetInfo, WidgetText, WidgetType};

use fluent2_tokens::generated::global::spacing_vertical;

use crate::foundation::icon::{self, IconName, IconSize};
use crate::foundation::text::TypeRamp;
use crate::theme::{self, Theme};

/// Validation shown under the child.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub enum ValidationState {
    /// No validation message.
    #[default]
    None,
    /// An error. `status_danger_foreground_1` and `ErrorCircle`.
    Error(String),
    /// A warning. `status_warning_foreground_1` and `Warning`.
    Warning(String),
    /// Success. `status_success_foreground_1` and `Checkmark`.
    Success(String),
}

/// A labelled field.
///
/// ```
/// # use fluent2_egui::controls::{Field, ValidationState};
/// # fn demo(ui: &mut egui::Ui) {
/// Field::new("Name").required(true).hint("As it appears on the account")
///     .validation(ValidationState::Error("Required".into()))
///     .show(ui, |ui| { ui.label("child"); });
/// # }
/// ```
pub struct Field<'a> {
    label: WidgetText,
    required: bool,
    hint: Option<String>,
    validation: ValidationState,
    _lifetime: std::marker::PhantomData<&'a ()>,
}

fn field_label_key() -> egui::Id {
    egui::Id::new("fluent2_field_label")
}

/// The label of the [`Field`] whose child is running, if any. Controls use it as their
/// accessible name when they have no label of their own.
pub(crate) fn enclosing_field_label(ctx: &egui::Context) -> Option<String> {
    ctx.data(|data| data.get_temp::<String>(field_label_key()))
}

impl<'a> Field<'a> {
    /// A field labelled `label`.
    pub fn new(label: impl Into<WidgetText>) -> Self {
        Self {
            label: label.into(),
            required: false,
            hint: None,
            validation: ValidationState::None,
            _lifetime: std::marker::PhantomData,
        }
    }

    /// Append a red `" *"` to the label.
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    /// Caption under the child, in `neutral_foreground_3`.
    #[must_use]
    pub fn hint(mut self, hint: impl Into<String>) -> Self {
        self.hint = Some(hint.into());
        self
    }

    /// Validation message under the hint (or under the child when there is no hint).
    #[must_use]
    pub fn validation(mut self, validation: ValidationState) -> Self {
        self.validation = validation;
        self
    }

    /// Show the label, then `child`, then the hint and validation lines.
    ///
    /// Returns whatever `child` returns.
    pub fn show<R>(self, ui: &mut Ui, child: impl FnOnce(&mut Ui) -> R) -> R {
        ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let label_text = self.label.text().to_owned();
        let accessible = if self.required {
            format!("{label_text} *")
        } else {
            label_text
        };
        ui.horizontal(|ui| {
            let galley = self
                .label
                .clone()
                .color(theme::c(theme.color.neutral_foreground_1))
                .into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    TextStyle::Name(TypeRamp::Body1.name().into()),
                );
            let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
            if ui.is_rect_visible(rect) {
                ui.painter().galley(
                    rect.left_top(),
                    galley,
                    theme::c(theme.color.neutral_foreground_1),
                );
            }
            if self.required {
                let star = WidgetText::from(" *")
                    .color(theme::c(theme.color.status_danger_foreground_1))
                    .into_galley(
                        ui,
                        Some(egui::TextWrapMode::Extend),
                        f32::INFINITY,
                        TextStyle::Name(TypeRamp::Body1.name().into()),
                    );
                let (star_rect, _) = ui.allocate_exact_size(star.size(), Sense::hover());
                if ui.is_rect_visible(star_rect) {
                    ui.painter().galley(
                        star_rect.left_top(),
                        star,
                        theme::c(theme.color.status_danger_foreground_1),
                    );
                }
            }
            response
                .widget_info(|| WidgetInfo::labeled(WidgetType::Label, ui.is_enabled(), accessible.clone()));
        });
        ui.add_space(spacing_vertical::XXS);
        // Controls in the child without a label of their own take the Field's label as their
        // accessible name (WI-1843). The previous value is restored, so nested Fields work.
        let key = field_label_key();
        let outer = ui.ctx().data(|data| data.get_temp::<String>(key));
        ui.ctx()
            .data_mut(|data| data.insert_temp(key, accessible.clone()));
        let result = child(ui);
        ui.ctx().data_mut(|data| {
            if let Some(outer) = outer {
                data.insert_temp(key, outer);
            } else {
                data.remove::<String>(key);
            }
        });
        if let Some(hint) = &self.hint {
            ui.add_space(spacing_vertical::XXS);
            paint_line(
                ui,
                &theme,
                hint,
                theme.color.neutral_foreground_3,
                TypeRamp::Caption1,
                None,
            );
        }
        if let Some((message, color, icon)) = validation_line(&self.validation, &theme) {
            ui.add_space(spacing_vertical::XXS);
            paint_line(ui, &theme, &message, color, TypeRamp::Caption1, Some(icon));
        }
        result
    }
}

fn validation_line(
    state: &ValidationState,
    theme: &Theme,
) -> Option<(String, fluent2_tokens::Rgba, IconName)> {
    let c = &theme.color;
    match state {
        ValidationState::None => None,
        ValidationState::Error(message) => Some((
            message.clone(),
            c.status_danger_foreground_1,
            IconName::ErrorCircle,
        )),
        ValidationState::Warning(message) => {
            Some((message.clone(), c.status_warning_foreground_1, IconName::Warning))
        }
        ValidationState::Success(message) => Some((
            message.clone(),
            c.status_success_foreground_1,
            IconName::Checkmark,
        )),
    }
}

fn paint_line(
    ui: &mut Ui,
    _theme: &Theme,
    text: &str,
    color: fluent2_tokens::Rgba,
    ramp: TypeRamp,
    icon: Option<IconName>,
) {
    ui.horizontal(|ui| {
        if let Some(icon) = icon {
            paint_icon(ui, icon, theme::c(color));
        }
        let galley = WidgetText::from(text).color(theme::c(color)).into_galley(
            ui,
            Some(egui::TextWrapMode::Wrap),
            ui.available_width().max(1.0),
            TextStyle::Name(ramp.name().into()),
        );
        let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
        if ui.is_rect_visible(rect) {
            ui.painter().galley(rect.left_top(), galley, theme::c(color));
        }
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text.to_owned()));
    });
}

fn paint_icon(ui: &mut Ui, icon: IconName, color: egui::Color32) {
    let px = 16.0;
    let (rect, _) = ui.allocate_exact_size(egui::Vec2::splat(px), Sense::hover());
    if ui.is_rect_visible(rect) {
        icon::paint_icon(
            ui,
            rect,
            icon,
            IconSize::S16,
            icon::IconStyle::Regular,
            Some(color),
        );
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
