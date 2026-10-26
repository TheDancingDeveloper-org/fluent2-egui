//! Combobox: an editable Input that filters a list of options.
//!
//! The listbox, option rows and field frame live in [`super::dropdown`] and are shared with
//! [`super::Dropdown`]. Metrics are the WI-1549 brief: 32 px rows, a check on the selected
//! option, and [`egui::ScrollArea::show_rows`] past 50 options.

use std::sync::atomic::{AtomicU64, Ordering};

use egui::{Id, Rect, Response, Sense, TextEdit, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType};

use super::ButtonSize;
use super::dropdown::{
    PreviewFn, field_height, finite, icon_points, icon_size, paint_outline, rounded_u8, show_listbox, step,
};
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconStyle, paint_icon};
use crate::theme::{Theme, c};
use egui::CornerRadius;

use fluent2_tokens::generated::global::spacing_horizontal;

/// Options the combobox filters without copying the list.
///
/// `revision` changes when the list changes. The filtered result is cached against it and the
/// query, so a stable list is not scanned again while the popup stays open on the same text.
/// A plain slice has no revision of its own; its cache key is a fingerprint of the labels.
pub trait ComboboxOptions {
    /// How many options the list holds.
    fn len(&self) -> usize;

    /// Whether the list holds no options.
    fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The option at `index`.
    fn get(&self, index: usize) -> &str;

    /// A value that changes when the list changes.
    fn revision(&self) -> u64;
}

/// A borrowed option list with an explicit revision.
///
/// Bump `revision` when the list changes so the filter cache rebuilds. [`Combobox::new`] borrows
/// a slice at revision 0 and does not copy the strings. [`ModelCombobox`] takes a model whose
/// revision can change.
pub struct SliceOptions<'a, T: AsRef<str>> {
    options: &'a [T],
    revision: u64,
}

impl<'a, T: AsRef<str>> SliceOptions<'a, T> {
    /// Borrow `options`. `revision` is the cache key for this list.
    pub fn new(options: &'a [T], revision: u64) -> Self {
        Self { options, revision }
    }
}

impl<T: AsRef<str>> ComboboxOptions for SliceOptions<'_, T> {
    fn len(&self) -> usize {
        self.options.len()
    }

    fn get(&self, index: usize) -> &str {
        self.options[index].as_ref()
    }

    fn revision(&self) -> u64 {
        self.revision
    }
}

/// Counts how often the combobox actually filters. Tests read it; production code leaves it
/// unset and pays nothing.
#[derive(Clone, Copy)]
pub struct FilterProbe(pub &'static AtomicU64);

/// Filtered indexes stored on the widget id. Rebuilt only while the popup is open and the
/// query or model revision changed.
#[derive(Clone)]
struct FilterCache {
    query: String,
    revision: u64,
    indexes: Vec<usize>,
}

/// An editable field that filters a list of options.
///
/// ```
/// # use fluent2_egui::controls::Combobox;
/// # fn demo(ui: &mut egui::Ui, text: &mut String) {
/// let _ = ui.add(Combobox::new("city", text, &["Perth", "Paris"]).freeform(true));
/// # }
/// ```
pub struct Combobox<'a, T: AsRef<str> = &'a str> {
    id_salt: Id,
    text: &'a mut String,
    options: &'a [T],
    model: Option<&'a dyn ComboboxOptions>,
    placeholder: Option<String>,
    label: Option<String>,
    size: ButtonSize,
    width: Option<f32>,
    freeform: bool,
    preview: Option<PreviewFn<'a>>,
    probe: Option<&'static AtomicU64>,
}

impl<'a, T: AsRef<str>> Combobox<'a, T> {
    /// A combobox bound to `text`. `id_salt` names the widget.
    ///
    /// Borrows `options` for this frame. The strings are not copied into the widget. A changed
    /// slice invalidates the filter cache even when the query stays the same.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        text: &'a mut String,
        options: &'a [T],
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            text,
            options,
            model: None,
            placeholder: None,
            label: None,
            size: ButtonSize::Medium,
            width: None,
            freeform: false,
            preview: None,
            probe: None,
        }
    }

    /// Count real filter passes. For tests, not application code.
    #[must_use]
    pub fn filter_probe(mut self, probe: FilterProbe) -> Self {
        self.probe = Some(probe.0);
        self
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

/// Combobox over a reusable [`ComboboxOptions`] model.
pub struct ModelCombobox<'a> {
    id_salt: Id,
    text: &'a mut String,
    model: &'a dyn ComboboxOptions,
    placeholder: Option<String>,
    label: Option<String>,
    size: ButtonSize,
    width: Option<f32>,
    freeform: bool,
    preview: Option<PreviewFn<'a>>,
    probe: Option<&'static AtomicU64>,
}

impl<'a> ModelCombobox<'a> {
    /// A combobox over a reusable model. The model is not copied.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        text: &'a mut String,
        model: &'a dyn ComboboxOptions,
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            text,
            model,
            placeholder: None,
            label: None,
            size: ButtonSize::Medium,
            width: None,
            freeform: false,
            preview: None,
            probe: None,
        }
    }

    /// Count real filter passes. For tests, not application code.
    #[must_use]
    pub fn filter_probe(mut self, probe: FilterProbe) -> Self {
        self.probe = Some(probe.0);
        self
    }

    /// Grey hint while `text` is empty. Not the accessible name.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }
}

impl Widget for ModelCombobox<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let combobox = Combobox {
            id_salt: self.id_salt,
            text: self.text,
            options: &[] as &[&str],
            model: Some(self.model),
            placeholder: self.placeholder,
            label: self.label,
            size: self.size,
            width: self.width,
            freeform: self.freeform,
            preview: self.preview,
            probe: self.probe,
        };
        combobox.ui(ui)
    }
}

impl<T: AsRef<str>> Widget for Combobox<'_, T> {
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
        let filter_id = id.with("filter");
        let view = OptionView {
            slice: self.options,
            model: self.model,
        };
        let filtered = if open {
            cached_filter(ui, filter_id, &view, self.text, self.probe)
        } else {
            Vec::new()
        };
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
            if !open {
                data.remove::<FilterCache>(filter_id);
            }
        });
        response
    }
}

struct OptionView<'a, T: AsRef<str>> {
    slice: &'a [T],
    model: Option<&'a dyn ComboboxOptions>,
}

impl<T: AsRef<str>> ComboboxOptions for OptionView<'_, T> {
    fn len(&self) -> usize {
        if let Some(model) = self.model {
            model.len()
        } else {
            self.slice.len()
        }
    }

    fn get(&self, index: usize) -> &str {
        if let Some(model) = self.model {
            model.get(index)
        } else {
            self.slice[index].as_ref()
        }
    }

    fn revision(&self) -> u64 {
        self.model
            .map(|model| model.revision())
            .unwrap_or_else(|| slice_fingerprint(self.slice))
    }
}

/// Cheap identity for a borrowed slice. Length plus a FNV-1a of every label, so a shorter list
/// or a same-length list with different text does not reuse another slice's filtered indexes.
fn slice_fingerprint<T: AsRef<str>>(slice: &[T]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    let mut hash = OFFSET;
    for label in slice {
        for byte in label.as_ref().as_bytes() {
            hash ^= u64::from(*byte);
            hash = hash.wrapping_mul(PRIME);
        }
        hash ^= 0xff;
        hash = hash.wrapping_mul(PRIME);
    }
    hash ^ (slice.len() as u64)
}

fn filter_matches(cache: &FilterCache, query: &str, options: &dyn ComboboxOptions) -> bool {
    cache.query == query
        && cache.revision == options.revision()
        && cache.indexes.iter().all(|index| *index < options.len())
}

fn cached_filter<'b>(
    ui: &Ui,
    filter_id: Id,
    options: &'b dyn ComboboxOptions,
    query: &str,
    probe: Option<&'static AtomicU64>,
) -> Vec<(usize, &'b str)> {
    let cached = ui.ctx().data(|data| data.get_temp::<FilterCache>(filter_id));
    let indexes = if let Some(cache) = cached.filter(|cache| filter_matches(cache, query, options)) {
        cache.indexes
    } else {
        if let Some(probe) = probe {
            probe.fetch_add(1, Ordering::Relaxed);
        }
        let indexes = filter_indexes(options, query);
        ui.ctx().data_mut(|data| {
            data.insert_temp(
                filter_id,
                FilterCache {
                    query: query.to_owned(),
                    revision: options.revision(),
                    indexes: indexes.clone(),
                },
            );
        });
        indexes
    };
    indexes
        .into_iter()
        .filter(|index| *index < options.len())
        .map(|index| (index, options.get(index)))
        .collect()
}

fn filter_indexes(options: &dyn ComboboxOptions, query: &str) -> Vec<usize> {
    let count = options.len();
    if query.is_empty() {
        return (0..count).collect();
    }
    let query = query.to_lowercase();
    let mut prefix = Vec::new();
    let mut rest = Vec::new();
    for index in 0..count {
        let label = options.get(index);
        let lower = label.to_lowercase();
        if lower.starts_with(&query) {
            prefix.push(index);
        } else if lower.contains(&query) {
            rest.push(index);
        }
    }
    prefix.append(&mut rest);
    prefix
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

    use std::sync::atomic::AtomicU64;

    use super::{ComboboxOptions, FilterProbe, ModelCombobox, SliceOptions};

    struct Counting<'a> {
        options: &'a [&'a str],
        revision: u64,
    }

    impl ComboboxOptions for Counting<'_> {
        fn len(&self) -> usize {
            self.options.len()
        }

        fn get(&self, index: usize) -> &str {
            self.options[index]
        }

        fn revision(&self) -> u64 {
            self.revision
        }
    }

    struct ProbeState {
        text: String,
        options: &'static [&'static str],
        revision: u64,
        open: bool,
        probe: &'static AtomicU64,
    }

    #[test]
    fn closed_popup_does_not_filter() {
        static PROBE: AtomicU64 = AtomicU64::new(0);
        let mut harness = Harness::new_ui_state(
            |ui, state: &mut ProbeState| {
                install_egui_theme(ui);
                let model = Counting {
                    options: state.options,
                    revision: state.revision,
                };
                ui.add(
                    ModelCombobox::new("city", &mut state.text, &model)
                        .filter_probe(FilterProbe(state.probe)),
                );
            },
            ProbeState {
                text: "P".to_owned(),
                options: &["Perth", "Paris"],
                revision: 1,
                open: false,
                probe: &PROBE,
            },
        );
        harness.run();
        harness.run();
        assert_eq!(PROBE.load(std::sync::atomic::Ordering::Relaxed), 0);
    }

    #[test]
    fn filter_reruns_when_the_query_or_model_changes_and_stays_cached() {
        static PROBE: AtomicU64 = AtomicU64::new(0);
        let mut harness = Harness::new_ui_state(
            |ui, state: &mut ProbeState| {
                install_egui_theme(ui);
                let model = Counting {
                    options: state.options,
                    revision: state.revision,
                };
                let response = ui.add(
                    ModelCombobox::new("city", &mut state.text, &model)
                        .filter_probe(FilterProbe(state.probe))
                        .placeholder("City"),
                );
                if state.open {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(response.id.with("open"), true));
                }
            },
            ProbeState {
                text: "P".to_owned(),
                options: &["Perth", "Paris", "Rome"],
                revision: 1,
                open: true,
                probe: &PROBE,
            },
        );
        harness.run();
        assert_eq!(
            PROBE.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "opening filters once"
        );
        harness.run();
        assert_eq!(
            PROBE.load(std::sync::atomic::Ordering::Relaxed),
            1,
            "same query stays cached"
        );
        harness.state_mut().text = "Pa".to_owned();
        harness.run();
        assert_eq!(
            PROBE.load(std::sync::atomic::Ordering::Relaxed),
            2,
            "a new query filters again"
        );
        harness.state_mut().revision = 2;
        harness.state_mut().options = &["Paris", "Padua", "Rome"];
        harness.run();
        assert_eq!(
            PROBE.load(std::sync::atomic::Ordering::Relaxed),
            3,
            "a new revision filters again"
        );
        let options = harness.state().options;
        let shown = SliceOptions::new(options, harness.state().revision);
        assert_eq!(shown.get(0), "Paris");
        harness.state_mut().open = false;
        let before = PROBE.load(std::sync::atomic::Ordering::Relaxed);
        harness.run();
        assert_eq!(
            PROBE.load(std::sync::atomic::Ordering::Relaxed),
            before,
            "closed does not filter"
        );
    }

    struct SliceProbe {
        text: String,
        options: Vec<&'static str>,
        open: bool,
        shown: Vec<String>,
    }

    fn slice_harness(options: Vec<&'static str>) -> Harness<'static, SliceProbe> {
        Harness::new_ui_state(
            |ui, state: &mut SliceProbe| {
                install_egui_theme(ui);
                let response =
                    ui.add(Combobox::new("city", &mut state.text, &state.options).placeholder("City"));
                if state.open {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(response.id.with("open"), true));
                }
                let view = super::OptionView {
                    slice: state.options.as_slice(),
                    model: None,
                };
                state.shown = super::cached_filter(ui, response.id.with("filter"), &view, &state.text, None)
                    .into_iter()
                    .map(|(_, label)| label.to_owned())
                    .collect();
            },
            SliceProbe {
                text: "P".to_owned(),
                options,
                open: true,
                shown: Vec::new(),
            },
        )
    }

    #[test]
    fn shrinking_an_open_slice_with_the_same_query_shows_the_new_options() {
        let mut harness = slice_harness(vec!["Perth", "Paris", "Padua", "Portland"]);
        harness.run();
        harness.run();
        assert_eq!(harness.state().shown, ["Perth", "Paris", "Padua", "Portland"]);
        harness.state_mut().options = vec!["Perth", "Paris"];
        harness.run();
        assert_eq!(harness.state().shown, ["Perth", "Paris"]);
    }

    #[test]
    fn a_same_length_slice_with_new_contents_is_shown() {
        let mut harness = slice_harness(vec!["Perth", "Paris", "Padua"]);
        harness.run();
        harness.run();
        harness.state_mut().options = vec!["Pisa", "Prague", "Porto"];
        harness.run();
        assert_eq!(harness.state().shown, ["Pisa", "Prague", "Porto"]);
    }

    #[test]
    fn time_picker_step_change_while_open_lists_the_new_times() {
        use crate::controls::calendar::TimeOfDay;
        use crate::controls::date_picker::TimePicker;

        struct StepState {
            value: Option<TimeOfDay>,
            step: u16,
            open: bool,
            shown: Vec<String>,
        }

        let mut harness = Harness::new_ui_state(
            |ui, state: &mut StepState| {
                install_egui_theme(ui);
                let built = TimePicker::new("start", &mut state.value).step_minutes(state.step);
                let options: Vec<String> = built.options().iter().map(|time| time.format(false)).collect();
                let response = TimePicker::new("start", &mut state.value)
                    .step_minutes(state.step)
                    .show(ui)
                    .response;
                if state.open {
                    ui.ctx()
                        .data_mut(|data| data.insert_temp(response.id.with("open"), true));
                }
                let view = super::OptionView {
                    slice: options.as_slice(),
                    model: None,
                };
                state.shown = super::cached_filter(ui, response.id.with("filter"), &view, "", None)
                    .into_iter()
                    .map(|(_, label)| label.to_owned())
                    .collect();
            },
            StepState {
                value: TimeOfDay::new(9, 0),
                step: 60,
                open: true,
                shown: Vec::new(),
            },
        );
        harness.run();
        assert!(harness.state().shown.iter().any(|label| label == "09:00"));
        assert!(!harness.state().shown.iter().any(|label| label == "09:30"));
        let hourly = harness.state().shown.len();
        harness.state_mut().step = 30;
        harness.run();
        assert!(harness.state().shown.iter().any(|label| label == "09:00"));
        assert!(harness.state().shown.iter().any(|label| label == "09:30"));
        assert!(harness.state().shown.len() > hourly);
    }
}
