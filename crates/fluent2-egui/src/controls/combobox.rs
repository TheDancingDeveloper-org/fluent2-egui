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
            size: ButtonSize::Medium,
            width: None,
            freeform: false,
            preview: None,
        }
    }

    /// Grey hint while `text` is empty.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
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

    /// When set, Enter on a typed string that matches no active option keeps the typed text.
    /// Otherwise Enter only accepts an option.
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
        let height = field_height(self.size);
        let width = finite(self.width.unwrap_or_else(|| ui.available_width())).max(0.0);
        let id = ui.make_persistent_id(self.id_salt);
        let open_id = id.with("open");
        let active_id = id.with("active");

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

        let focused = response.has_focus() && enabled;
        if response.changed() && enabled && !self.text.is_empty() {
            open = true;
            active = 0;
        }
        if enabled && focused {
            let input = ui.input(|i| i.clone());
            if input.modifiers.alt && input.key_pressed(egui::Key::ArrowDown) {
                open = true;
            }
            if open && input.key_pressed(egui::Key::ArrowDown) {
                active = step(Some(active), filtered.len(), 1);
            }
            if open && input.key_pressed(egui::Key::ArrowUp) {
                active = step(Some(active), filtered.len(), -1);
            }
            if input.key_pressed(egui::Key::Escape) {
                open = false;
            }
            if enter {
                if open {
                    if self.freeform {
                        // The typed text is already in `self.text`. Keep it.
                        response.mark_changed();
                    } else if let Some((_, label)) = filtered.get(active) {
                        *self.text = (*label).to_owned();
                        response.mark_changed();
                    } else {
                        self.text.clear();
                        response.mark_changed();
                    }
                }
                open = false;
            }
        }

        let chevron = Rect::from_center_size(
            egui::pos2(rect.right() - pad_x - icon_px / 2.0, rect.center().y),
            Vec2::splat(icon_px),
        );
        let chevron_response = ui.interact(chevron, id.with("chevron"), Sense::click());
        if chevron_response.clicked() && enabled {
            open = !open;
            response.request_focus();
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
            enabled,
            focused,
            response.hovered() || chevron_response.hovered(),
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

        let accessible_name = if hint.is_empty() {
            "Combobox".to_owned()
        } else {
            hint
        };
        let value_text = self.text.clone();
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
            let picked = show_listbox(
                ui,
                id,
                rect,
                &labels,
                selected_in_list,
                Some(active),
                &self.preview,
            );
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
        });
        response
    }
}
