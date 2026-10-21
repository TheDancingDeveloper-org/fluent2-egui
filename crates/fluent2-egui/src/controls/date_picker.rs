//! DatePicker and TimePicker (WI-1943).
//!
//! DatePicker is an editable field with a calendar button. Typing a date and pressing Enter (or
//! leaving the field) sets it; the button, or Alt+Down, opens a [`Calendar`] in a popup, where a
//! pick sets the date and closes it. TimePicker is a [`Combobox`] of times at a fixed step that
//! also accepts typed times ("14:30", "2:30 pm"). Text that does not parse leaves the value as it
//! was and marks the field invalid.

use egui::{CornerRadius, Id, Key, Modifiers, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use super::calendar::{Calendar, CalendarDate, CalendarStrings, TimeOfDay};
use super::dropdown::{finite, paint_outline, rounded_u8};
use super::{ButtonSize, Combobox};
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::theme::{Theme, c};

/// What a picker did this frame.
#[derive(Debug)]
pub struct PickerResponse {
    /// The field.
    pub response: Response,
    /// The value changed this frame.
    pub changed: bool,
    /// The typed text did not parse; the value was left as it was.
    pub invalid: bool,
}

/// A date field with a popup calendar.
///
/// ```
/// # use fluent2_egui::controls::{CalendarDate, DatePicker};
/// # fn demo(ui: &mut egui::Ui, due: &mut Option<CalendarDate>) {
/// let out = DatePicker::new("due", due).label("Due date").today(CalendarDate::new(2026, 10, 20)).show(ui);
/// if out.changed {
///     // save `due`
/// }
/// # }
/// ```
pub struct DatePicker<'a> {
    id_salt: Id,
    value: &'a mut Option<CalendarDate>,
    today: Option<CalendarDate>,
    first_weekday: u8,
    min: Option<CalendarDate>,
    max: Option<CalendarDate>,
    strings: CalendarStrings,
    day_first: bool,
    placeholder: String,
    label: Option<String>,
    width: Option<f32>,
}

impl<'a> DatePicker<'a> {
    /// A picker bound to `value`.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, value: &'a mut Option<CalendarDate>) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            value,
            today: None,
            first_weekday: 0,
            min: None,
            max: None,
            strings: CalendarStrings::default(),
            day_first: true,
            placeholder: "YYYY-MM-DD".to_owned(),
            label: None,
            width: None,
        }
    }

    /// Today, marked in the calendar.
    #[must_use]
    pub fn today(mut self, today: Option<CalendarDate>) -> Self {
        self.today = today;
        self
    }

    /// The calendar's first column, 0 = Sunday (the default) … 6 = Saturday.
    #[must_use]
    pub fn first_weekday(mut self, weekday: u8) -> Self {
        self.first_weekday = weekday % 7;
        self
    }

    /// The earliest and latest dates that can be picked or typed.
    #[must_use]
    pub fn range(mut self, min: Option<CalendarDate>, max: Option<CalendarDate>) -> Self {
        self.min = min;
        self.max = max;
        self
    }

    /// Calendar names.
    #[must_use]
    pub fn strings(mut self, strings: CalendarStrings) -> Self {
        self.strings = strings;
        self
    }

    /// Whether typed `a/b/yyyy` means day/month (the default) or month/day. `yyyy-mm-dd` is
    /// always accepted, and is what the field shows.
    #[must_use]
    pub fn day_first(mut self, day_first: bool) -> Self {
        self.day_first = day_first;
        self
    }

    /// Grey hint while empty. The default is "YYYY-MM-DD".
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = placeholder.into();
        self
    }

    /// Accessible name. Inside a Field, the Field's label is the default.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Width. The default is 200.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    fn allowed(&self, date: CalendarDate) -> bool {
        self.min.is_none_or(|min| date >= min) && self.max.is_none_or(|max| date <= max)
    }

    /// Show the picker.
    pub fn show(self, ui: &mut Ui) -> PickerResponse {
        crate::controls::ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(self.id_salt);
        let edit_id = id.with("edit");
        let text_id = id.with("text");
        let open_id = id.with("open");
        let invalid_id = id.with("invalid");
        let label = self
            .label
            .clone()
            .or_else(|| super::field::enclosing_field_label(ui.ctx()))
            .unwrap_or_else(|| "Date".to_owned());
        let height = crate::theme::density_of(ui).height(32.0);
        let width = finite(self.width.unwrap_or(200.0)).max(120.0);
        let (rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let button_side = height - 8.0;
        let button = Rect::from_center_size(
            egui::pos2(rect.right() - 4.0 - button_side / 2.0, rect.center().y),
            Vec2::splat(button_side),
        );
        let text_rect = Rect::from_min_max(
            egui::pos2(rect.left() + spacing_horizontal::S, rect.top()),
            egui::pos2(button.left() - 4.0, rect.bottom()),
        );

        let focused_before = ui.memory(|m| m.had_focus_last_frame(edit_id));
        let mut text = ui
            .ctx()
            .data(|d| d.get_temp::<String>(text_id))
            .filter(|_| focused_before)
            .unwrap_or_else(|| self.value.map(CalendarDate::to_iso).unwrap_or_default());
        let mut open = ui.ctx().data(|d| d.get_temp::<bool>(open_id).unwrap_or(false));
        let mut invalid = ui.ctx().data(|d| d.get_temp::<bool>(invalid_id).unwrap_or(false));
        let mut changed = false;

        let (enter, alt_down) = if enabled && focused_before {
            ui.input_mut(|i| {
                (
                    i.consume_key(Modifiers::NONE, Key::Enter),
                    i.consume_key(Modifiers::ALT, Key::ArrowDown),
                )
            })
        } else {
            (false, false)
        };

        let background = ui.painter().add(egui::Shape::Noop);
        let response = ui.put(
            text_rect,
            TextEdit::singleline(&mut text)
                .id(edit_id)
                .frame(egui::Frame::NONE)
                .interactive(enabled)
                .desired_width(text_rect.width())
                .text_color(c(theme.color.neutral_foreground_1))
                .hint_text(
                    egui::WidgetText::from(self.placeholder.clone())
                        .color(c(theme.color.neutral_foreground_4)),
                ),
        );

        if enter || response.lost_focus() {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                if self.value.is_some() {
                    *self.value = None;
                    changed = true;
                }
                invalid = false;
            } else {
                match CalendarDate::parse(trimmed, self.day_first).filter(|d| self.allowed(*d)) {
                    Some(date) => {
                        if *self.value != Some(date) {
                            *self.value = Some(date);
                            changed = true;
                        }
                        text = date.to_iso();
                        invalid = false;
                    }
                    None => invalid = true,
                }
            }
        }

        let calendar_button = ui.interact(button, id.with("button"), Sense::click());
        if enabled && (calendar_button.clicked() || alt_down) {
            open = !open || alt_down;
        }
        if calendar_button.hovered() && enabled {
            ui.painter().rect_filled(
                button,
                CornerRadius::same(rounded_u8(border_radius::MEDIUM)),
                c(theme.color.subtle_background_hover),
            );
        }
        paint_icon(
            ui,
            button.shrink(2.0),
            IconName::Calendar,
            IconSize::S20,
            IconStyle::Regular,
            Some(c(theme.color.neutral_foreground_3)),
        );
        if focus_visible(ui, &calendar_button) {
            paint_focus_ring(ui, button, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
        }
        let choose = format!("Choose {}", label.to_lowercase());
        calendar_button
            .widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, enabled, choose.clone()));
        ui.ctx().accesskit_node_builder(calendar_button.id, |node| {
            node.set_has_popup(egui::accesskit::HasPopup::Dialog);
            if open {
                node.set_expanded(true);
            } else {
                node.set_expanded(false);
            }
        });

        paint_outline(
            ui,
            rect,
            &theme,
            background,
            (
                enabled,
                response.has_focus(),
                response.hovered() || calendar_button.hovered(),
            ),
        );
        if invalid {
            ui.painter().rect_stroke(
                rect,
                CornerRadius::same(rounded_u8(border_radius::MEDIUM)),
                Stroke::new(stroke_width::THIN, c(theme.color.status_danger_border_1)),
                StrokeKind::Inside,
            );
        }
        if focus_visible(ui, &response) {
            paint_focus_ring(ui, rect, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
        }
        let value_text = self.value.map(|d| self.strings.long_date(d)).unwrap_or_default();
        let name = label.clone();
        response.widget_info(|| {
            let mut info = egui::WidgetInfo::labeled(egui::Role::TextInput, enabled, name.clone());
            info.current_text_value = Some(value_text.clone());
            info
        });
        if invalid {
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_invalid(egui::accesskit::Invalid::True)
            });
        }

        if open && enabled {
            let mut selected = *self.value;
            let popup_id = id.with("popup");
            let anchor = ui.interact(rect, id.with("anchor"), Sense::hover());
            let shown = egui::Popup::from_response(&anchor)
                .id(popup_id)
                .frame(crate::surfaces::menu_frame(ui))
                .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
                .open(true)
                .show(|ui| {
                    Calendar::new(id.with("calendar"), &mut selected)
                        .today(self.today)
                        .first_weekday(self.first_weekday)
                        .range(self.min, self.max)
                        .strings(self.strings.clone())
                        .show(ui)
                });
            let escape = ui.input(|i| i.key_pressed(Key::Escape));
            if let Some(inner) = &shown {
                if let Some(date) = inner.inner.picked {
                    if *self.value != Some(date) {
                        *self.value = Some(date);
                        changed = true;
                    }
                    text = date.to_iso();
                    invalid = false;
                    open = false;
                    response.request_focus();
                }
                let popup_rect = inner.response.rect;
                let outside = ui.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer
                            .interact_pos()
                            .is_some_and(|p| !popup_rect.contains(p) && !rect.contains(p))
                });
                if outside {
                    open = false;
                }
            }
            if escape {
                open = false;
                response.request_focus();
            }
        }

        ui.ctx().data_mut(|d| {
            d.insert_temp(text_id, text);
            d.insert_temp(open_id, open);
            d.insert_temp(invalid_id, invalid);
        });
        PickerResponse {
            response,
            changed,
            invalid,
        }
    }
}

/// A time field: a list of times at a fixed step, or a typed time.
///
/// ```
/// # use fluent2_egui::controls::{TimeOfDay, TimePicker};
/// # fn demo(ui: &mut egui::Ui, at: &mut Option<TimeOfDay>) {
/// let out = TimePicker::new("start", at).step_minutes(15).show(ui);
/// # let _ = out;
/// # }
/// ```
pub struct TimePicker<'a> {
    id_salt: Id,
    value: &'a mut Option<TimeOfDay>,
    step: u16,
    start: TimeOfDay,
    end: TimeOfDay,
    twelve_hour: bool,
    width: Option<f32>,
    size: ButtonSize,
}

impl<'a> TimePicker<'a> {
    /// A picker bound to `value`, offering every 30 minutes of the day.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, value: &'a mut Option<TimeOfDay>) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            value,
            step: 30,
            start: TimeOfDay::MIDNIGHT,
            end: TimeOfDay::LAST_MINUTE,
            twelve_hour: false,
            width: None,
            size: ButtonSize::Medium,
        }
    }

    /// Minutes between listed times (1 to 720). The default is 30.
    #[must_use]
    pub fn step_minutes(mut self, step: u16) -> Self {
        self.step = step.clamp(1, 720);
        self
    }

    /// The first and last listed times. Typed times outside it are still accepted.
    #[must_use]
    pub fn hours(mut self, start: TimeOfDay, end: TimeOfDay) -> Self {
        self.start = start.min(end);
        self.end = end.max(start);
        self
    }

    /// Show times as "2:30 PM" instead of "14:30".
    #[must_use]
    pub fn twelve_hour(mut self, twelve_hour: bool) -> Self {
        self.twelve_hour = twelve_hour;
        self
    }

    /// Width. The default is 140.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Field height.
    #[must_use]
    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    /// The listed times.
    #[must_use]
    pub fn options(&self) -> Vec<TimeOfDay> {
        let mut out = Vec::new();
        let mut minutes = self.start.minutes();
        while minutes <= self.end.minutes() {
            if let Some(time) = TimeOfDay::new((minutes / 60) as u8, (minutes % 60) as u8) {
                out.push(time);
            }
            minutes += self.step;
        }
        out
    }

    /// Show the picker.
    pub fn show(self, ui: &mut Ui) -> PickerResponse {
        let id = ui.make_persistent_id(self.id_salt);
        let text_id = id.with("text");
        let editing_id = id.with("editing");
        // While the field has focus, the typed text is kept as typed; it is parsed only when
        // committed (WI-1943 review: reformatting every keystroke made "14:30" untypable).
        let editing = ui.ctx().data(|d| d.get_temp::<bool>(editing_id)).unwrap_or(false);
        let mut text = ui
            .ctx()
            .data(|d| d.get_temp::<String>(text_id))
            .filter(|_| editing)
            .unwrap_or_else(|| self.value.map(|t| t.format(self.twelve_hour)).unwrap_or_default());
        // Read before the Combobox, which consumes Enter.
        let enter = editing && ui.input(|i| i.key_pressed(egui::Key::Enter));
        let options: Vec<String> = self
            .options()
            .iter()
            .map(|t| t.format(self.twelve_hour))
            .collect();
        let response = ui.add(
            Combobox::new(id.with("combobox"), &mut text, &options)
                .freeform(true)
                .size(self.size)
                .width(finite(self.width.unwrap_or(140.0)))
                .placeholder(if self.twelve_hour { "h:mm AM" } else { "HH:MM" }),
        );
        let mut changed = false;
        let mut invalid = false;
        // Commit on Enter, on leaving the field, or when the text is exactly a listed time (a
        // pick from the list). Other edits stay as typed.
        let picked = response.changed() && options.contains(&text);
        if enter || response.lost_focus() || picked {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                changed = self.value.take().is_some();
            } else if let Some(time) = TimeOfDay::parse(trimmed) {
                changed = *self.value != Some(time);
                *self.value = Some(time);
                text = time.format(self.twelve_hour);
            } else {
                invalid = true;
            }
        }
        let still_editing = response.has_focus() && !response.lost_focus();
        ui.ctx().data_mut(|d| {
            d.insert_temp(text_id, text);
            d.insert_temp(editing_id, still_editing);
        });
        PickerResponse {
            response,
            changed,
            invalid,
        }
    }
}
