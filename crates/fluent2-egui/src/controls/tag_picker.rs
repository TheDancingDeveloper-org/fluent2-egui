//! TagPicker: a token input with removable tags, free-text entry and suggestions (WI-1941).
//!
//! The app owns the tags, the typed query and the suggestions. When the query changes,
//! [`TagPickerResponse::query_changed`] tells the app to fetch suggestions (for example
//! `/contacts/suggest`), which it passes in on later frames with [`TagPicker::loading`] while
//! the request runs. Nothing here blocks.
//!
//! Keyboard: typing filters, Down and Up move through the suggestions, Enter adds the
//! highlighted suggestion (or the typed text when free text is allowed), Escape closes the
//! list, and Backspace in an empty field removes the last tag. A `,`, `;` or line break, typed
//! or pasted, adds every piece before it, so a pasted "a@x.com; b@y.com" becomes two tags. Each
//! tag has a remove button, which is a Tab stop named "Remove …".

use egui::{
    CornerRadius, Id, Key, Modifiers, Rect, Response, Sense, Stroke, StrokeKind, TextEdit, Ui, Vec2,
    WidgetInfo, WidgetText, Role,
};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use super::dropdown::{finite, paint_outline, rounded_u8, show_listbox, step};
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::theme::{Theme, c};

/// One suggestion offered under the field.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TagSuggestion {
    /// The tag text added when it is picked (for example an address).
    pub value: String,
    /// What the list shows (for example a name). Empty shows `value`.
    pub label: String,
    /// A second, dimmer part shown after the label (for example the address).
    pub secondary: Option<String>,
}

impl TagSuggestion {
    /// A suggestion whose label is its value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            ..Self::default()
        }
    }

    /// Show `label` (and the value as secondary text) instead of the value.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        if self.secondary.is_none() {
            self.secondary = Some(self.value.clone());
        }
        self
    }

    fn display(&self) -> String {
        let label = if self.label.is_empty() {
            &self.value
        } else {
            &self.label
        };
        match &self.secondary {
            Some(secondary) if secondary != label => format!("{label} <{secondary}>"),
            _ => label.clone(),
        }
    }
}

/// What [`TagPicker::show`] did this frame.
#[derive(Debug)]
pub struct TagPickerResponse {
    /// The field.
    pub response: Response,
    /// The typed query changed: fetch suggestions for it.
    pub query_changed: bool,
    /// Tags added this frame.
    pub added: Vec<String>,
    /// Tags removed this frame.
    pub removed: Vec<String>,
}

type Validator<'a> = &'a dyn Fn(&str) -> bool;

/// A Fluent TagPicker. See the module docs.
///
/// ```
/// # use fluent2_egui::controls::{TagPicker, TagSuggestion};
/// # fn demo(ui: &mut egui::Ui, to: &mut Vec<String>, query: &mut String, found: &[TagSuggestion]) {
/// let valid = |tag: &str| tag.contains('@');
/// let out = TagPicker::new("to", to, query)
///     .label("To")
///     .suggestions(found)
///     .validate(&valid)
///     .show(ui);
/// if out.query_changed {
///     // start fetching suggestions for `query`
/// }
/// # }
/// ```
pub struct TagPicker<'a> {
    id_salt: Id,
    tags: &'a mut Vec<String>,
    query: &'a mut String,
    suggestions: &'a [TagSuggestion],
    /// The query the suggestions were fetched for, when the app says so.
    suggestions_query: Option<String>,
    loading: bool,
    free_text: bool,
    validate: Option<Validator<'a>>,
    placeholder: Option<String>,
    label: Option<String>,
    width: Option<f32>,
}

impl<'a> TagPicker<'a> {
    /// A picker over `tags`, with the typed text in `query`. `id_salt` names the widget.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        tags: &'a mut Vec<String>,
        query: &'a mut String,
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            tags,
            query,
            suggestions: &[],
            suggestions_query: None,
            loading: false,
            free_text: true,
            validate: None,
            placeholder: None,
            label: None,
            width: None,
        }
    }

    /// The suggestions for the current query, in order. For results that arrive later (a
    /// fetch), prefer [`TagPicker::suggestions_for`], so a list for an older query is never
    /// offered.
    #[must_use]
    pub fn suggestions(mut self, suggestions: &'a [TagSuggestion]) -> Self {
        self.suggestions = suggestions;
        self
    }

    /// The suggestions fetched for `query`. They are offered only while the typed query still
    /// matches it (ignoring case and surrounding spaces); until the current query's results
    /// arrive, the list is hidden and Enter adds the typed text instead.
    #[must_use]
    pub fn suggestions_for(mut self, query: impl Into<String>, suggestions: &'a [TagSuggestion]) -> Self {
        self.suggestions = suggestions;
        self.suggestions_query = Some(query.into());
        self
    }

    /// Suggestions are being fetched: a spinner shows in the field.
    #[must_use]
    pub fn loading(mut self, loading: bool) -> Self {
        self.loading = loading;
        self
    }

    /// Whether Enter, `,` and `;` add typed text that is not a suggestion. The default is true.
    #[must_use]
    pub fn free_text(mut self, allowed: bool) -> Self {
        self.free_text = allowed;
        self
    }

    /// Marks tags for which `valid` is false in the danger style (they are still added).
    #[must_use]
    pub fn validate(mut self, valid: Validator<'a>) -> Self {
        self.validate = Some(valid);
        self
    }

    /// Grey hint while there are no tags and no query.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Accessible name. Inside a Field, the Field's label is the default.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Width. Defaults to [`Ui::available_width`].
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Show the picker.
    pub fn show(self, ui: &mut Ui) -> TagPickerResponse {
        crate::controls::ensure_type_ramp(ui);
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let id = ui.make_persistent_id(self.id_salt);
        let edit_id = id.with("edit");
        let open_id = id.with("open");
        let active_id = id.with("active");
        let label = self
            .label
            .clone()
            .or_else(|| super::field::enclosing_field_label(ui.ctx()))
            .unwrap_or_else(|| "Tags".to_owned());
        let width = finite(self.width.unwrap_or_else(|| ui.available_width())).max(80.0);
        let pad = spacing_horizontal::S;
        let gap = spacing_horizontal::XS;
        let min_height = crate::theme::density_of(ui).height(32.0);

        let mut added = Vec::new();
        let mut removed = Vec::new();
        let had_focus = ui.memory(|memory| memory.had_focus_last_frame(edit_id));
        // Enter and Backspace are read before the TextEdit runs, which would consume them.
        let (enter, backspace) = if enabled && had_focus {
            ui.input_mut(|input| {
                (
                    input.consume_key(Modifiers::NONE, Key::Enter),
                    self.query.is_empty() && input.key_pressed(Key::Backspace),
                )
            })
        } else {
            (false, false)
        };
        // A paste is taken before the single-line TextEdit, which would turn its line breaks
        // into spaces; line breaks separate tags like `,` and `;`.
        let pasted: Vec<String> = if enabled && had_focus {
            ui.input_mut(|input| {
                let mut pasted = Vec::new();
                input.events.retain(|event| match event {
                    egui::Event::Paste(text) => {
                        pasted.push(text.clone());
                        false
                    }
                    _ => true,
                });
                pasted
            })
        } else {
            Vec::new()
        };
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
        let mut active = ui
            .ctx()
            .data(|data| data.get_temp::<usize>(active_id))
            .unwrap_or(0);

        let background = ui.painter().add(egui::Shape::Noop);
        let start = ui.cursor().min;
        let mut edit_response = None;
        let mut remove_index = None;
        let outer = ui.allocate_ui_with_layout(
            Vec2::new(width, min_height),
            egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true),
            |ui| {
                ui.set_width(width);
                ui.set_min_height(min_height);
                ui.add_space(pad);
                ui.spacing_mut().item_spacing = Vec2::new(gap, spacing_vertical::XS);
                let validate = self.validate;
                for (index, tag) in self.tags.iter().enumerate() {
                    let valid = validate.is_none_or(|valid| valid(tag));
                    if tag_chip(ui, id.with(("tag", index)), tag, valid, enabled) {
                        remove_index = Some(index);
                    }
                }
                let spinner = if self.loading { 20.0 } else { 0.0 };
                let room = (ui.available_width() - pad - spinner).max(80.0);
                let hint = if self.tags.is_empty() {
                    self.placeholder.clone().unwrap_or_default()
                } else {
                    String::new()
                };
                let response = ui.add(
                    TextEdit::singleline(self.query)
                        .id(edit_id)
                        .frame(egui::Frame::NONE)
                        .interactive(enabled)
                        .desired_width(room)
                        .text_color(c(theme.color.neutral_foreground_1))
                        .hint_text(WidgetText::from(hint).color(c(theme.color.neutral_foreground_4))),
                );
                if self.loading {
                    ui.add(crate::controls::Spinner::new().size(crate::controls::SpinnerSize::Tiny));
                }
                edit_response = Some(response);
            },
        );
        let rect = Rect::from_min_size(
            start,
            Vec2::new(width, outer.response.rect.height().max(min_height)),
        );
        let Some(response) = edit_response else {
            return TagPickerResponse {
                response: outer.response,
                query_changed: false,
                added,
                removed,
            };
        };

        if let Some(index) = remove_index
            && index < self.tags.len()
        {
            removed.push(self.tags.remove(index));
            response.request_focus();
        }

        let mut query_changed = response.changed();
        for text in &pasted {
            self.query.push_str(text);
            query_changed = true;
        }
        // Typed or pasted separators add every piece before the last.
        if query_changed && self.free_text && self.query.contains([',', ';', '\n', '\r']) {
            let text = std::mem::take(self.query);
            let mut pieces: Vec<&str> = text.split([',', ';', '\n', '\r']).collect();
            let rest = pieces.pop().unwrap_or("").trim_start().to_owned();
            for piece in pieces {
                add_tag(self.tags, piece, &mut added);
            }
            *self.query = rest;
        }
        if query_changed {
            open = true;
            active = 0;
        }
        // Suggestions are stale while loading, on the frame the query changed (they cannot be
        // for it yet), or when they were fetched for another query: then none are offered, so
        // Enter cannot add a result of an older query (WI-1941 review).
        let stale = self.loading
            || query_changed
            || self
                .suggestions_query
                .as_deref()
                .is_some_and(|q| !q.trim().eq_ignore_ascii_case(self.query.trim()));
        let current: &[TagSuggestion] = if stale { &[] } else { self.suggestions };
        let count = current.len();
        active = active.min(count.saturating_sub(1));

        let focused = response.has_focus() && enabled;
        if focused {
            let (down, up, escape) = ui.input(|i| {
                (
                    i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::Escape),
                )
            });
            if down {
                if open {
                    active = step(Some(active), count, 1);
                } else {
                    open = true;
                }
            }
            if up && open {
                active = step(Some(active), count, -1);
            }
            if escape {
                open = false;
            }
        }
        if enter {
            let picked = (open && count > 0)
                .then(|| current.get(active))
                .flatten()
                .map(|s| s.value.clone());
            if let Some(value) = picked {
                add_tag(self.tags, &value, &mut added);
                self.query.clear();
                query_changed = true;
            } else if self.free_text && !self.query.trim().is_empty() {
                let text = std::mem::take(self.query);
                add_tag(self.tags, &text, &mut added);
                query_changed = true;
            }
            open = false;
            response.request_focus();
        }
        if backspace && let Some(last) = self.tags.pop() {
            removed.push(last);
        }

        paint_outline(
            ui,
            rect,
            &theme,
            background,
            (enabled, focused, outer.response.hovered()),
        );
        if focus_visible(ui, &response) {
            paint_focus_ring(ui, rect, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
        }

        let value = self.tags.join(", ");
        let name = label.clone();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(Role::ComboBox, enabled, name.clone());
            info.current_text_value = Some(value.clone());
            info
        });

        if open && enabled && focused && count > 0 {
            let labels: Vec<String> = current.iter().map(TagSuggestion::display).collect();
            let (picked, outside) = show_listbox(ui, id, rect, &labels, None, Some(active), &None);
            if let Some(index) = picked
                && let Some(suggestion) = current.get(index)
            {
                add_tag(self.tags, &suggestion.value, &mut added);
                self.query.clear();
                query_changed = true;
                open = false;
                response.request_focus();
            }
            if outside {
                open = false;
            }
        }
        if !focused && !response.lost_focus() {
            open = false;
        }

        ui.ctx().data_mut(|data| {
            data.insert_temp(open_id, open);
            data.insert_temp(active_id, active);
        });
        TagPickerResponse {
            response,
            query_changed,
            added,
            removed,
        }
    }
}

/// Add `text` (trimmed) unless it is empty or already a tag (ignoring case).
fn add_tag(tags: &mut Vec<String>, text: &str, added: &mut Vec<String>) {
    let tag = text.trim();
    if tag.is_empty() || tags.iter().any(|t| t.eq_ignore_ascii_case(tag)) {
        return;
    }
    tags.push(tag.to_owned());
    added.push(tag.to_owned());
}

/// One tag: its text and a remove button. Returns whether the button was activated.
fn tag_chip(ui: &mut Ui, id: Id, tag: &str, valid: bool, enabled: bool) -> bool {
    let theme = Theme::get(ui.ctx());
    let color = &theme.color;
    let (fill, ink, border) = if valid {
        (
            color.neutral_background_3,
            color.neutral_foreground_1,
            color.transparent_stroke,
        )
    } else {
        (
            color.status_danger_background_1,
            color.status_danger_foreground_1,
            color.status_danger_border_1,
        )
    };
    let galley = WidgetText::from(tag.to_owned()).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        240.0,
        egui::TextStyle::Body,
    );
    let height = 24.0;
    let button = 20.0;
    let width = spacing_horizontal::S + galley.size().x + spacing_horizontal::XXS + button + 2.0;
    let (rect, chip) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    ui.painter().rect(
        rect,
        radius,
        c(fill),
        Stroke::new(stroke_width::THIN, c(border)),
        StrokeKind::Inside,
    );
    ui.painter().galley(
        egui::pos2(
            rect.left() + spacing_horizontal::S,
            rect.center().y - galley.size().y / 2.0,
        ),
        galley,
        c(ink),
    );
    let x_rect = Rect::from_center_size(
        egui::pos2(rect.right() - 2.0 - button / 2.0, rect.center().y),
        Vec2::splat(button),
    );
    let remove = ui.interact(x_rect, id.with("remove"), Sense::click());
    if remove.hovered() && enabled {
        ui.painter()
            .rect_filled(x_rect, radius, c(color.subtle_background_hover));
    }
    paint_icon(
        ui,
        x_rect.shrink(2.0),
        IconName::Dismiss,
        IconSize::S16,
        IconStyle::Regular,
        Some(c(ink)),
    );
    if focus_visible(ui, &remove) {
        paint_focus_ring(ui, x_rect, radius);
    }
    let name = format!("Remove {tag}");
    remove.widget_info(|| WidgetInfo::labeled(Role::Button, enabled, name.clone()));
    let tag_text = tag.to_owned();
    ui.ctx().accesskit_node_builder(chip.id, |node| {
        node.set_role(egui::accesskit::Role::ListItem);
        node.set_label(tag_text);
        if !valid {
            node.set_invalid(egui::accesskit::Invalid::True);
        }
    });
    let keyboard =
        remove.has_focus() && ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace));
    enabled && (remove.clicked() || keyboard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_tag_trims_and_skips_duplicates() {
        let mut tags = vec!["a@x.com".to_owned()];
        let mut added = Vec::new();
        add_tag(&mut tags, "  b@y.com ", &mut added);
        add_tag(&mut tags, "A@X.COM", &mut added);
        add_tag(&mut tags, "   ", &mut added);
        assert_eq!(tags, ["a@x.com", "b@y.com"]);
        assert_eq!(added, ["b@y.com"]);
    }

    #[test]
    fn suggestion_display() {
        assert_eq!(TagSuggestion::new("a@x.com").display(), "a@x.com");
        assert_eq!(
            TagSuggestion::new("ada@x.com").label("Ada Lovelace").display(),
            "Ada Lovelace <ada@x.com>"
        );
    }
}
