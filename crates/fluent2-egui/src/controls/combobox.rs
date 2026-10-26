//! Combobox: an editable Input that filters a list of options.
//!
//! The listbox, option rows and field frame live in [`super::dropdown`] and are shared with
//! [`super::Dropdown`]. Metrics are the WI-1549 brief: 32 px rows, a check on the selected
//! option, and [`egui::ScrollArea::show_rows`] past 50 options.

use egui::{Id, Rect, Response, Sense, TextEdit, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType};

use super::ButtonSize;
use super::dropdown::{
    PreviewFn, field_height, filter_options, finite, icon_points, icon_size, paint_outline, rounded_u8,
    show_listbox, step,
};
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconStyle, paint_icon};
use crate::theme::{Theme, c};
use egui::CornerRadius;

use fluent2_tokens::generated::global::spacing_horizontal;

/// An editable field that filters a list of options.
///
/// ```
/// # use fluent2_egui::controls::Combobox;
/// # fn demo(ui: &mut egui::Ui, text: &mut String) {
/// let _ = ui.add(Combobox::new("city", text, &["Perth", "Paris"]).freeform(true));
/// # }
/// ```
pub struct Combobox<'a> {
    id_salt: Id,
    text: &'a mut String,
    options: Vec<String>,
    placeholder: Option<String>,
    label: Option<String>,
    size: ButtonSize,
    width: Option<f32>,
    freeform: bool,
    preview: Option<PreviewFn<'a>>,
}

impl<'a> Combobox<'a> {
    /// A combobox bound to `text`. `id_salt` names the widget.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        text: &'a mut String,
        options: &[impl AsRef<str>],
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            text,
            options: options.iter().map(|option| option.as_ref().to_owned()).collect(),
            placeholder: None,
            label: None,
            size: ButtonSize::Medium,
            width: None,
            freeform: false,
            preview: None,
        }
    }

    /// Grey hint while `text` is empty. Not the accessible name.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Accessible name, read by screen readers. Not drawn. Inside a [`super::Field`] the
    /// Field's label is the default; otherwise the placeholder, then "Combobox".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Field height. Defaults to [`ButtonSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// Field width. Defaults to [`Ui::available_width`].
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// When set, Enter keeps the typed text unless the user has moved the highlight with the
    /// arrow keys since the text last changed. Otherwise Enter only accepts an option.
    #[must_use]
    pub fn freeform(mut self, freeform: bool) -> Self {
        self.freeform = freeform;
        self
    }

    /// Draw each option. The default is plain text.
    #[must_use]
    pub fn option_preview(mut self, preview: impl Fn(&mut Ui, &str) + 'a) -> Self {
        self.preview = Some(Box::new(preview));
        self
    }
}

impl Widget for Combobox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let height = field_height(self.size, crate::theme::density_of(ui));
        let width = finite(self.width.unwrap_or_else(|| ui.available_width())).max(0.0);
        let id = ui.make_persistent_id(self.id_salt);
        let open_id = id.with("open");
        let active_id = id.with("active");
        let navigated_id = id.with("navigated");

        let rect = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover()).0;
        let icon_px = icon_points(self.size);
        let pad_x = spacing_horizontal::S;
        let text_rect = Rect::from_min_max(
            egui::pos2(rect.left() + pad_x, rect.top()),
            egui::pos2(
                rect.right() - pad_x - icon_px - spacing_horizontal::XXS,
                rect.bottom(),
            ),
        );
        let hint = self.placeholder.clone().unwrap_or_default();
        let text_color = if enabled {
            c(theme.color.neutral_foreground_1)
        } else {
            c(theme.color.neutral_foreground_disabled)
        };
        let had_focus = ui.memory(|memory| memory.had_focus_last_frame(id));
        let enter = enabled
            && had_focus
            && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
        // Reserved before the text and chevron paint so the frame fill lands beneath them
        // (WI-1913).
        let background = ui.painter().add(egui::Shape::Noop);
        let mut response = ui.put(
            text_rect,
            TextEdit::singleline(self.text)
                .id(id)
                .frame(egui::Frame::NONE)
                .interactive(enabled)
                .desired_width(text_rect.width().max(0.0))
                .text_color(text_color)
                .hint_text(WidgetText::from(hint.clone()).color(c(theme.color.neutral_foreground_4))),
        );

        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
        let filtered = filter_options(&self.options, self.text);
        let mut active = ui
            .ctx()
            .data(|data| data.get_temp::<usize>(active_id))
            .unwrap_or(0)
            .min(filtered.len().saturating_sub(1));
        // Set only by arrow keys, and cleared when the text changes, so a freeform Enter can
        // tell a keyboard selection from the default highlight on the first filtered option.
        let mut navigated = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(navigated_id))
            .unwrap_or(false)
            && !filtered.is_empty();

        let focused = response.has_focus() && enabled;
        if response.changed() && enabled && !self.text.is_empty() {
            open = true;
            active = 0;
            navigated = false;
        }
        if enabled && focused {
            let input = ui.input(|i| i.clone());
            if input.modifiers.alt && input.key_pressed(egui::Key::ArrowDown) {
                open = true;
            }
            if open && input.key_pressed(egui::Key::ArrowDown) {
                // The first arrow selects the highlighted option; later arrows move it.
                if navigated {
                    active = step(Some(active), filtered.len(), 1);
                }
                navigated = !filtered.is_empty();
            }
            if open && input.key_pressed(egui::Key::ArrowUp) {
                if navigated {
                    active = step(Some(active), filtered.len(), -1);
                }
                navigated = !filtered.is_empty();
            }
            if input.key_pressed(egui::Key::Escape) {
                open = false;
            }
            if enter {
                if open {
                    let commit_active = if self.freeform { navigated } else { true };
                    if commit_active {
                        if let Some((_, label)) = filtered.get(active) {
                            *self.text = (*label).to_owned();
                            response.mark_changed();
                        } else if !self.freeform {
                            self.text.clear();
                            response.mark_changed();
                        }
                    } else {
                        // The typed text is already in `self.text`. Keep it.
                        response.mark_changed();
                    }
                }
                open = false;
                navigated = false;
            }
        }

        let chevron = Rect::from_center_size(
            egui::pos2(rect.right() - pad_x - icon_px / 2.0, rect.center().y),
            Vec2::splat(icon_px),
        );
        // Pointer only: the field is the Tab stop and Alt+Down opens the list. A focusable
        // chevron was a silent extra Tab stop (WI-1586 Orca walk).
        let chevron_response = ui.interact(chevron, id.with("chevron"), Sense::CLICK);
        if chevron_response.clicked() && enabled {
            open = !open;
            response.request_focus();
        }
        if open && !response.has_focus() && !chevron_response.clicked() {
            // A key press moves the pointer's interact position off the field. Close only on
            // a press that is not over the popup (WI-145 / #145).
            let pressed_outside = ui.input(|input| {
                let pointer = &input.pointer;
                let pressed = pointer.any_pressed() || pointer.any_click();
                let over_popup = pointer.interact_pos().is_some_and(|pos| pos.y >= rect.bottom());
                pressed && !over_popup
            });
            if pressed_outside {
                open = false;
            }
        }
        let icon_color = if enabled {
            c(theme.color.neutral_foreground_3)
        } else {
            c(theme.color.neutral_foreground_disabled)
        };
        paint_icon(
            ui,
            chevron,
            IconName::ChevronDown,
            icon_size(self.size),
            IconStyle::Regular,
            Some(icon_color),
        );

        paint_outline(
            ui,
            rect,
            &theme,
            background,
            (enabled, focused, response.hovered() || chevron_response.hovered()),
        );
        if focus_visible(ui, &response) {
            paint_focus_ring(
                ui,
                rect,
                CornerRadius::same(rounded_u8(
                    fluent2_tokens::generated::global::border_radius::MEDIUM,
                )),
            );
        }

        // Name, placeholder and value stay separate. TextEdit already wrote the AccessKit
        // node (the placeholder as hint_text); a later widget_info does not replace it, so
        // the name is set on that node. The value is the typed text, not the name.
        let accessible_name = self
            .label
            .clone()
            .filter(|label| !label.is_empty())
            .or_else(|| super::field::enclosing_field_label(ui.ctx()).filter(|label| !label.is_empty()))
            .or_else(|| {
                let placeholder = hint.trim();
                if placeholder.is_empty() {
                    None
                } else {
                    Some(placeholder.to_owned())
                }
            })
            .unwrap_or_else(|| "Combobox".to_owned());
        let value_text = self.text.clone();
        ui.ctx().accesskit_node_builder(response.id, |builder| {
            builder.set_label(accessible_name.clone());
            builder.set_value(value_text.clone());
        });
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ComboBox, enabled, accessible_name.clone());
            info.current_text_value = Some(value_text.clone());
            info
        });

        if open && enabled {
            let labels: Vec<&str> = filtered.iter().map(|(_, label)| *label).collect();
            let selected_in_list = labels
                .iter()
                .position(|label| label.eq_ignore_ascii_case(self.text));
            let (picked, outside) = show_listbox(
                ui,
                id,
                rect,
                &labels,
                selected_in_list,
                Some(active),
                &self.preview,
            );
            if outside {
                open = false;
            }
            if let Some(index) = picked {
                if let Some((_, label)) = filtered.get(index) {
                    *self.text = (*label).to_owned();
                    response.mark_changed();
                }
                open = false;
            }
        }

        ui.ctx().data_mut(|data| {
            data.insert_temp(open_id, open);
            data.insert_temp(active_id, active);
            data.insert_temp(navigated_id, navigated);
        });
        response
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::rc::Rc;

    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;

    use super::Combobox;
    use crate::controls::Field;
    use crate::theme::{self, Theme};
    use fluent2_tokens::FLUENT;

    fn install_egui_theme(ui: &egui::Ui) {
        let theme = match ui.ctx().theme() {
            egui::Theme::Light => Theme::light(FLUENT),
            egui::Theme::Dark => Theme::dark(FLUENT),
        };
        theme::install(ui.ctx(), &theme);
    }

    fn freeform_city() -> (Rc<RefCell<String>>, Harness<'static, ()>) {
        let text = Rc::new(RefCell::new(String::new()));
        let shown = Rc::clone(&text);
        let harness = Harness::new_ui(move |ui| {
            install_egui_theme(ui);
            let mut value = shown.borrow().clone();
            let response = ui.add(
                Combobox::new("city", &mut value, &["Perth", "Paris", "Padua"])
                    .placeholder("City")
                    .freeform(true)
                    .width(160.0),
            );
            response.request_focus();
            *shown.borrow_mut() = value;
        });
        (text, harness)
    }

    #[test]
    fn freeform_enter_commits_the_arrow_key_option() {
        let (text, mut harness) = freeform_city();
        harness.run();
        harness.run();
        harness.get_by_label("City").type_text("pa");
        harness.run();
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(text.borrow().as_str(), "Paris");
    }

    #[test]
    fn freeform_second_arrow_commits_the_next_option() {
        let (text, mut harness) = freeform_city();
        harness.run();
        harness.run();
        harness.get_by_label("City").type_text("pa");
        harness.run();
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(text.borrow().as_str(), "Padua");
    }

    #[test]
    fn freeform_typing_after_navigation_keeps_the_text() {
        let (text, mut harness) = freeform_city();
        harness.run();
        harness.run();
        harness.get_by_label("City").type_text("pa");
        harness.run();
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        harness.get_by_label("City").type_text("d");
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(text.borrow().as_str(), "pad");
    }

    #[test]
    fn field_label_names_a_combobox_without_a_placeholder() {
        let mut harness = Harness::new_ui(|ui| {
            install_egui_theme(ui);
            let mut value = String::new();
            Field::new("Home city").show(ui, |ui| {
                ui.add(Combobox::new("city", &mut value, &["Perth", "Paris"]).width(160.0));
            });
        });
        harness.run();
        harness.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Home city");
    }

    #[test]
    fn explicit_label_wins_over_the_field() {
        let mut harness = Harness::new_ui(|ui| {
            install_egui_theme(ui);
            let mut value = String::new();
            Field::new("Home city").show(ui, |ui| {
                ui.add(
                    Combobox::new("city", &mut value, &["Perth", "Paris"])
                        .label("Destination")
                        .width(160.0),
                );
            });
        });
        harness.run();
        harness.get_by_role_and_label(egui::accesskit::Role::ComboBox, "Destination");
        harness.get_by_role_and_label(egui::accesskit::Role::Label, "Home city");
    }
}
