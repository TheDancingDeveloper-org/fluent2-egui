//! Dropdown and Combobox.
//!
//! Both open a listbox popup under an Input-shaped field. Dropdown is a button: the selected
//! option's text, a chevron, and keyboard type-to-jump. Combobox is an editable Input that
//! filters the same list. Metrics follow the WI-1549 brief: option rows are 32 px, the selected
//! row carries a check, and more than 50 options are laid out with [`egui::ScrollArea::show_rows`].
//!
//! Colours and radii come from the installed theme. The field reuses the Input outline frame
//! (rest stroke, accessible bottom border, focus accent).

use egui::{
    Align, CornerRadius, Id, Layout, Rect, Response, ScrollArea, Sense, Stroke, StrokeKind, Ui, Vec2, Widget,
    WidgetInfo, WidgetText, WidgetType, epaint::RectShape,
};

use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::motion::{Curve, Duration, animate};
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use super::ButtonSize;

pub(super) const VIRTUAL_THRESHOLD: usize = 50;
/// Option row height at Medium density.
pub(super) const OPTION_HEIGHT: f32 = 32.0;

/// Option row height at `density`.
pub(super) fn option_height(density: crate::theme::Density) -> f32 {
    density.row(OPTION_HEIGHT)
}
const MAX_VISIBLE_ROWS: f32 = 8.0;

pub(super) type PreviewFn<'a> = Box<dyn Fn(&mut Ui, &str) + 'a>;

/// A closed list of options. The value is the selected index, or `None`.
///
/// ```
/// # use fluent2_egui::controls::Dropdown;
/// # fn demo(ui: &mut egui::Ui, selected: &mut Option<usize>) {
/// let _ = ui.add(Dropdown::new("font", selected, &["Segoe", "Calibri"]).placeholder("Font"));
/// # }
/// ```
pub struct Dropdown<'a> {
    id_salt: Id,
    selected: &'a mut Option<usize>,
    options: Vec<String>,
    placeholder: Option<String>,
    size: ButtonSize,
    width: Option<f32>,
    preview: Option<PreviewFn<'a>>,
}

impl<'a> Dropdown<'a> {
    /// A dropdown. `id_salt` names the widget; the id is never taken from an option label.
    pub fn new(
        id_salt: impl std::hash::Hash + std::fmt::Debug,
        selected: &'a mut Option<usize>,
        options: &[impl AsRef<str>],
    ) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            selected,
            options: options.iter().map(|option| option.as_ref().to_owned()).collect(),
            placeholder: None,
            size: ButtonSize::Medium,
            width: None,
            preview: None,
        }
    }

    /// Text shown while nothing is selected. Also the accessible name.
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.placeholder = Some(placeholder.into());
        self
    }

    /// Field height. Defaults to [`ButtonSize::Medium`] (32 px). Option rows stay 32 px.
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

    /// Draw each option. The default is plain text. The closure sees the option string.
    #[must_use]
    pub fn option_preview(mut self, preview: impl Fn(&mut Ui, &str) + 'a) -> Self {
        self.preview = Some(Box::new(preview));
        self
    }
}

impl Widget for Dropdown<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let enabled = ui.is_enabled();
        let height = field_height(self.size, theme.density);
        let width = finite(self.width.unwrap_or_else(|| ui.available_width())).max(0.0);
        let id = ui.make_persistent_id(self.id_salt);
        let open_id = id.with("open");
        let active_id = id.with("active");
        let typed_id = id.with("typed");

        let (rect, mut response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
        let mut open = ui
            .ctx()
            .data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
        let mut active = ui.ctx().data(|data| data.get_temp::<usize>(active_id));

        let focused = response.has_focus() && enabled;
        if enabled && focused {
            let input = ui.input(|input| input.clone());
            if let Some(ch) = typed_letter(&input) {
                let previous = ui.ctx().data(|data| data.get_temp::<char>(typed_id));
                let start = if previous == Some(ch) {
                    active.or(*self.selected).map(|index| index + 1).unwrap_or(0)
                } else {
                    0
                };
                if let Some(found) = jump_to(&self.options, ch, start) {
                    active = Some(found);
                    open = true;
                }
                ui.ctx().data_mut(|data| data.insert_temp(typed_id, ch));
            }
            let typed = typed_letter(&input).is_some();
            if !typed
                && (input.key_pressed(egui::Key::Enter)
                    || input.key_pressed(egui::Key::Space)
                    || (input.modifiers.alt && input.key_pressed(egui::Key::ArrowDown)))
            {
                let was_open = ui
                    .ctx()
                    .data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
                if was_open && input.key_pressed(egui::Key::Enter) {
                    // Select below, once `active` is known. Opening is skipped.
                } else {
                    open = !open || input.modifiers.alt;
                }
                response.request_focus();
            }
            if open {
                // Egui clears focus on Escape unless the focused widget asks for it, and an
                // open popup needs Escape to close itself before any enclosing Dialog sees it.
                ui.memory_mut(|memory| {
                    memory.set_focus_lock_filter(
                        response.id,
                        egui::EventFilter {
                            escape: true,
                            ..Default::default()
                        },
                    );
                });
                let count = self.options.len();
                if input.key_pressed(egui::Key::ArrowDown) {
                    active = Some(step(active.or(*self.selected), count, 1));
                }
                if input.key_pressed(egui::Key::ArrowUp) {
                    active = Some(step(active.or(*self.selected), count, -1));
                }
                if input.key_pressed(egui::Key::Escape) {
                    open = false;
                    // Consume the press so an enclosing Dialog does not also read it as its own Escape.
                    ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape));
                }
                let was_open = ui
                    .ctx()
                    .data(|data| data.get_temp::<bool>(open_id).unwrap_or(false));
                if was_open && input.key_pressed(egui::Key::Enter) {
                    if let Some(index) = active.or(*self.selected).filter(|index| *index < count) {
                        *self.selected = Some(index);
                        response.mark_changed();
                    }
                    open = false;
                }
            }
        }

        if response.clicked() && enabled {
            open = !open;
            response.request_focus();
        }

        let selected_index = *self.selected;
        let chosen = match selected_index {
            Some(index) => self.options.get(index).cloned(),
            None => None,
        };
        let label = chosen.or_else(|| self.placeholder.clone()).unwrap_or_default();
        paint_field(
            ui,
            rect,
            &theme,
            (enabled, focused, response.hovered(), selected_index.is_none()),
            &label,
        );

        if focus_visible(ui, &response) {
            paint_focus_ring(ui, rect, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
        }
        let accessible_name = self.placeholder.clone().unwrap_or_else(|| "Dropdown".to_owned());
        let value_text = selected_index
            .and_then(|index| self.options.get(index).cloned())
            .unwrap_or_default();
        response.widget_info(|| {
            let mut info = WidgetInfo::labeled(WidgetType::ComboBox, enabled, accessible_name.clone());
            info.current_text_value = Some(value_text.clone());
            info
        });

        if open && enabled {
            let (picked, outside) =
                show_listbox(ui, id, rect, &self.options, selected_index, active, &self.preview);
            if let Some(index) = picked {
                *self.selected = Some(index);
                response.mark_changed();
                open = false;
            } else if outside {
                open = false;
            }
        }

        ui.ctx().data_mut(|data| {
            data.insert_temp(open_id, open);
            if let Some(index) = active {
                data.insert_temp(active_id, index);
            }
        });
        response
    }
}

pub(super) fn show_listbox(
    ui: &mut Ui,
    id: Id,
    field: Rect,
    options: &[impl AsRef<str>],
    selected: Option<usize>,
    active: Option<usize>,
    preview: &Option<PreviewFn<'_>>,
) -> (Option<usize>, bool) {
    let count = options.len();
    let rows = (count as f32)
        .min(MAX_VISIBLE_ROWS)
        .max(if count == 0 { 0.0 } else { 1.0 });
    if count == 0 {
        return (None, false);
    }
    let option_h = option_height(crate::theme::density_of(ui));
    let popup_height = rows * option_h;
    let popup_id = id.with("popup");
    let mut picked = None;
    // `Popup` is the layer egui hit-tests. A bare `Area` receives no pointer events, so a click
    // on an option never reached the row.
    // WI-1545 menu frame: neutral fill, medium radius and the elevation shadow.
    let frame = crate::surfaces::menu_frame(ui);
    let shown = egui::Popup::from_response(&ui.interact(field, id.with("anchor"), Sense::hover()))
        .id(popup_id)
        .layout(Layout::top_down(Align::Min))
        .frame(frame)
        // IgnoreClicks keeps a scrollbar drag from closing the list. Dismissal of a press that
        // is outside both the field and this popup is handled below.
        .close_behavior(egui::PopupCloseBehavior::IgnoreClicks)
        .open(true)
        .show(|ui| {
            ui.set_min_width(field.width());
            ui.set_max_width(field.width());
            ui.set_max_height(popup_height);
            let mut row = |ui: &mut Ui, index: usize| {
                let label = options.get(index).map(|option| option.as_ref()).unwrap_or("");
                let response = option_row(
                    ui,
                    id,
                    index,
                    label,
                    selected == Some(index),
                    active == Some(index),
                    preview,
                );
                // kittest delivers the pointer press on the frame the hover arrives, and the
                // release on the next. `clicked()` needs both on one widget, so a press over
                // the row is the selection.
                let pressed = response.contains_pointer() && ui.input(|input| input.pointer.any_pressed());
                if response.clicked() || pressed {
                    picked = Some(index);
                }
            };
            if count > VIRTUAL_THRESHOLD {
                ScrollArea::vertical()
                    .id_salt(popup_id.with("scroll"))
                    .max_height(popup_height)
                    .show_rows(ui, option_h, count, |ui, range| {
                        for index in range {
                            row(ui, index);
                        }
                    });
            } else {
                for index in 0..count {
                    row(ui, index);
                }
            }
        });
    // A press on the scrollbar is inside the popup rect, so it does not dismiss. A press on the
    // field is handled by the field. Anything else closes.
    let popup_rect = shown.as_ref().map(|inner| inner.response.rect);
    let outside = ui.input(|input| {
        input.pointer.any_pressed()
            && input
                .pointer
                .interact_pos()
                .is_some_and(|pos| !field.contains(pos) && popup_rect.is_none_or(|rect| !rect.contains(pos)))
    });
    (picked, outside)
}

pub(super) fn option_row(
    ui: &mut Ui,
    _id: Id,
    _index: usize,
    label: &str,
    selected: bool,
    active: bool,
    preview: &Option<PreviewFn<'_>>,
) -> Response {
    let theme = Theme::get(ui.ctx());
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(width.max(0.0), option_height(theme.density)),
        Sense::click(),
    );
    let fill = if response.is_pointer_button_down_on() {
        Some(theme.color.subtle_background_pressed)
    } else if response.hovered() || active {
        Some(theme.color.subtle_background_hover)
    } else {
        None
    };
    if let Some(color) = fill {
        ui.painter().rect_filled(rect, CornerRadius::ZERO, c(color));
    }
    // High contrast fills hover and press with the highlight colour; the text follows it.
    let ink = if response.is_pointer_button_down_on() {
        theme.color.neutral_foreground_1_pressed
    } else if response.hovered() || active {
        theme.color.neutral_foreground_1_hover
    } else {
        theme.color.neutral_foreground_1
    };
    let check = 16.0;
    let pad = spacing_horizontal::S;
    if selected {
        let icon_rect = Rect::from_center_size(
            egui::pos2(rect.left() + pad + check / 2.0, rect.center().y),
            Vec2::splat(check),
        );
        paint_icon(
            ui,
            icon_rect,
            IconName::Checkmark,
            IconSize::S16,
            IconStyle::Regular,
            Some(c(ink)),
        );
    }
    let text_rect = Rect::from_min_max(
        egui::pos2(rect.left() + pad + check + spacing_horizontal::XS, rect.top()),
        egui::pos2(rect.right() - pad, rect.bottom()),
    );
    if let Some(preview) = preview {
        ui.scope_builder(egui::UiBuilder::new().max_rect(text_rect), |ui| {
            ui.set_min_size(text_rect.size());
            preview(ui, label);
        });
    } else {
        let galley = WidgetText::from(label.to_owned()).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            text_rect.width().max(0.0),
            egui::TextStyle::Body,
        );
        let pos = egui::pos2(text_rect.left(), rect.center().y - galley.size().y / 2.0);
        ui.painter().galley(pos, galley, c(ink));
    }
    response.widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, selected, label));
    response
}

fn paint_field(ui: &Ui, rect: Rect, theme: &Theme, state: (bool, bool, bool, bool), label: &str) {
    let (enabled, focused, hovered, placeholder) = state;
    paint_outline(ui, rect, theme, enabled, focused, hovered);
    let color = if !enabled {
        c(theme.color.neutral_foreground_disabled)
    } else if placeholder {
        c(theme.color.neutral_foreground_4)
    } else {
        c(theme.color.neutral_foreground_1)
    };
    let pad = spacing_horizontal::S;
    let icon_px = 16.0;
    let text_rect = Rect::from_min_max(
        egui::pos2(rect.left() + pad, rect.top()),
        egui::pos2(
            rect.right() - pad - icon_px - spacing_horizontal::XXS,
            rect.bottom(),
        ),
    );
    let galley = WidgetText::from(label.to_owned()).color(color).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        text_rect.width().max(0.0),
        egui::TextStyle::Body,
    );
    let pos = egui::pos2(text_rect.left(), rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(pos, galley, color);
    let icon_rect = Rect::from_center_size(
        egui::pos2(rect.right() - pad - icon_px / 2.0, rect.center().y),
        Vec2::splat(icon_px),
    );
    let icon_color = if enabled {
        c(theme.color.neutral_foreground_3)
    } else {
        c(theme.color.neutral_foreground_disabled)
    };
    paint_icon(
        ui,
        icon_rect,
        IconName::ChevronDown,
        IconSize::S16,
        IconStyle::Regular,
        Some(icon_color),
    );
}

pub(super) fn paint_outline(ui: &Ui, rect: Rect, theme: &Theme, enabled: bool, focused: bool, hovered: bool) {
    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    let stroke = if !enabled {
        Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_disabled))
    } else if hovered {
        Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_1_hover))
    } else {
        Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_1))
    };
    ui.painter().add(RectShape::new(
        rect,
        radius,
        c(theme.color.neutral_background_1),
        stroke,
        StrokeKind::Inside,
    ));
    let target = if focused { 1.0 } else { 0.0 };
    let t = animate(
        ui.ctx(),
        accent_id(rect),
        target,
        Duration::Normal,
        Curve::DecelerateMid,
    );
    if t > 0.0 {
        let half = rect.width() * 0.5 * t;
        let bar = Rect::from_min_max(
            egui::pos2(rect.center().x - half, rect.bottom() - stroke_width::THICK),
            egui::pos2(rect.center().x + half, rect.bottom()),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::ZERO, c(theme.color.compound_brand_stroke));
    }
    if t < 1.0 {
        let color = if !enabled {
            theme.color.neutral_stroke_disabled
        } else if hovered {
            theme.color.neutral_stroke_accessible_hover
        } else {
            theme.color.neutral_stroke_accessible
        };
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(stroke_width::THIN, c(color)),
        );
    }
}

/// Options whose text contains `query`, prefix matches first. An empty query returns every option.
pub(super) fn filter_options<'a>(options: &'a [String], query: &str) -> Vec<(usize, &'a str)> {
    if query.is_empty() {
        return options
            .iter()
            .enumerate()
            .map(|(index, label)| (index, label.as_str()))
            .collect();
    }
    let query = query.to_lowercase();
    let mut prefix = Vec::new();
    let mut rest = Vec::new();
    for (index, label) in options.iter().enumerate() {
        let lower = label.to_lowercase();
        if lower.starts_with(&query) {
            prefix.push((index, label.as_str()));
        } else if lower.contains(&query) {
            rest.push((index, label.as_str()));
        }
    }
    prefix.append(&mut rest);
    prefix
}

fn jump_to(options: &[String], letter: char, start: usize) -> Option<usize> {
    let letter = letter.to_ascii_lowercase();
    let count = options.len();
    if count == 0 {
        return None;
    }
    for offset in 0..count {
        let index = (start + offset) % count;
        if options[index]
            .chars()
            .next()
            .is_some_and(|ch| ch.to_ascii_lowercase() == letter)
        {
            return Some(index);
        }
    }
    None
}

pub(super) fn step(current: Option<usize>, count: usize, delta: isize) -> usize {
    if count == 0 {
        return 0;
    }
    let base = current.unwrap_or(0) as isize;
    let next = (base + delta).clamp(0, count as isize - 1);
    next as usize
}

fn typed_letter(input: &egui::InputState) -> Option<char> {
    input.events.iter().find_map(|event| match event {
        egui::Event::Text(text) => text.chars().next().filter(|ch| ch.is_ascii_alphabetic()),
        egui::Event::Key {
            key,
            pressed: true,
            modifiers,
            ..
        } if modifiers.is_none() && key.name().len() == 1 => {
            key.name().chars().next().filter(|ch| ch.is_ascii_alphabetic())
        }
        _ => None,
    })
}

/// Field height for `size` at `density`.
pub(super) fn field_height(size: ButtonSize, density: crate::theme::Density) -> f32 {
    density.height(match size {
        ButtonSize::Small => 24.0,
        ButtonSize::Medium => 32.0,
        ButtonSize::Large => 40.0,
    })
}

pub(super) fn icon_size(size: ButtonSize) -> IconSize {
    if matches!(size, ButtonSize::Large) {
        IconSize::S20
    } else {
        IconSize::S16
    }
}

pub(super) fn icon_points(size: ButtonSize) -> f32 {
    if matches!(size, ButtonSize::Large) {
        20.0
    } else {
        16.0
    }
}

pub(super) fn accent_id(rect: Rect) -> Id {
    Id::new((
        "fluent2_dropdown_accent",
        rect.min.x.to_bits(),
        rect.min.y.to_bits(),
        rect.width().to_bits(),
    ))
}

pub(super) fn rounded_u8(value: f32) -> u8 {
    if !value.is_finite() || value <= 0.0 {
        0
    } else if value >= f32::from(u8::MAX) {
        u8::MAX
    } else {
        value.round() as u8
    }
}

pub(super) fn finite(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_puts_prefix_before_substring() {
        let options = ["Paris", "Sparc", "Perth"].map(str::to_owned);
        let filtered = filter_options(&options[..], "p");
        let labels: Vec<&str> = filtered.iter().map(|(_, label)| *label).collect();
        assert_eq!(labels, ["Paris", "Perth", "Sparc"]);
    }

    #[test]
    fn jump_wraps() {
        let options = ["Apple", "Banana", "Apricot"].map(str::to_owned);
        assert_eq!(jump_to(&options, 'a', 1), Some(2));
        assert_eq!(jump_to(&options, 'a', 0), Some(0));
    }
}
