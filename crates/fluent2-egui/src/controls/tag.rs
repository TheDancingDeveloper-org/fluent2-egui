//! Tag, InteractionTag and TagGroup (WI-1934).
//!
//! A [`Tag`] is a compact label for a filter, a category or a picked item: filled, outline or
//! brand, in extra-small to medium, with an optional icon or [`Avatar`], secondary text and a
//! dismiss button. An [`InteractionTag`] adds a primary action (a button) beside its separate
//! dismiss target. A [`TagGroup`] lays tags out in a row, as an AccessKit List, with one Tab stop:
//! the arrow keys move between tags, and dismissing a tag moves focus to the next one.
//!
//! Keyboard: Enter or Space activates the focused target; Delete or Backspace on a focused tag
//! dismisses it. Dismiss buttons are named `"Remove <text>"`.
//!
//! Tags are app data: [`TagResponse::dismissed`] tells the app to drop the tag.
//!
//! ```
//! # use fluent2_egui::controls::{Tag, TagAppearance, TagSize};
//! # fn demo(ui: &mut egui::Ui) {
//! let out = Tag::new("Bug").appearance(TagAppearance::Brand).size(TagSize::Small).dismissible(true).show(ui);
//! if out.dismissed {
//!     // drop the "Bug" filter
//! }
//! # }
//! ```

use egui::{
    CornerRadius, EventFilter, Id, Key, Pos2, Rect, Response, Sense, Stroke, StrokeKind, TextStyle, Ui,
    UiBuilder, Vec2, Widget, WidgetInfo, WidgetText, WidgetType,
};
use fluent2_tokens::Rgba;
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use super::avatar::Avatar;
use crate::foundation::focus::{focus_visible, paint_focus_ring};
use crate::foundation::icon::{IconName, IconSize, IconStyle, paint_icon};
use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, ThemeKind, c};

/// How a tag is filled.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TagAppearance {
    /// `neutral_background_3`. The default.
    #[default]
    Filled,
    /// No fill, a `neutral_stroke_1` outline.
    Outline,
    /// `brand_background_2` with `brand_foreground_2`.
    Brand,
}

/// A tag's height: 20, 24 or 32 pt.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TagSize {
    /// 20 pt, Caption1 text, 12 pt media.
    ExtraSmall,
    /// 24 pt, Caption1 text, 16 pt media.
    Small,
    /// 32 pt, Body1 text, 20 pt media; secondary text sits under the primary. The default.
    #[default]
    Medium,
}

/// What [`Tag::show`], [`InteractionTag::show`] and [`TagGroup::show`] report.
#[derive(Debug)]
pub struct TagResponse {
    /// The tag body (for an [`InteractionTag`], its primary button).
    pub response: Response,
    /// The dismiss button, when the tag has one.
    pub dismiss: Option<Response>,
    /// The primary action was activated (only an [`InteractionTag`]).
    pub clicked: bool,
    /// The dismiss button was activated, or Delete or Backspace was pressed on the tag.
    pub dismissed: bool,
}

#[derive(Clone, Debug)]
enum Media {
    Icon(IconName),
    Avatar(Avatar),
}

/// A Fluent Tag. See the module docs.
///
/// ```
/// # use fluent2_egui::controls::Tag;
/// # use fluent2_egui::foundation::icon::IconName;
/// # fn demo(ui: &mut egui::Ui) {
/// ui.add(Tag::new("Design").icon(IconName::Person).secondary("12 items"));
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Tag {
    text: String,
    secondary: Option<String>,
    appearance: TagAppearance,
    size: TagSize,
    media: Option<Media>,
    dismissible: bool,
}

impl Tag {
    /// A filled, medium tag showing `text`.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            secondary: None,
            appearance: TagAppearance::default(),
            size: TagSize::default(),
            media: None,
            dismissible: false,
        }
    }

    /// Filled (the default), outline or brand.
    #[must_use]
    pub fn appearance(mut self, appearance: TagAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Extra-small, small or medium (the default).
    #[must_use]
    pub fn size(mut self, size: TagSize) -> Self {
        self.size = size;
        self
    }

    /// A leading icon. Replaces an avatar.
    #[must_use]
    pub fn icon(mut self, icon: IconName) -> Self {
        self.media = Some(Media::Icon(icon));
        self
    }

    /// A leading avatar, resized to the tag. Replaces an icon.
    #[must_use]
    pub fn avatar(mut self, avatar: Avatar) -> Self {
        self.media = Some(Media::Avatar(avatar));
        self
    }

    /// Dimmer second text: under the primary at medium size, after it otherwise.
    #[must_use]
    pub fn secondary(mut self, text: impl Into<String>) -> Self {
        self.secondary = Some(text.into());
        self
    }

    /// Add a dismiss button (`"Remove <text>"`).
    #[must_use]
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = dismissible;
        self
    }

    /// Show the tag.
    pub fn show(self, ui: &mut Ui) -> TagResponse {
        show_tag(ui, &self, Options::default())
    }

    fn accessible_text(&self) -> String {
        match &self.secondary {
            Some(secondary) if !secondary.is_empty() => format!("{}, {secondary}", self.text),
            _ => self.text.clone(),
        }
    }
}

impl Widget for Tag {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// A Fluent InteractionTag: a tag whose body is a button, with a separate dismiss target.
///
/// ```
/// # use fluent2_egui::controls::{InteractionTag, Tag};
/// # fn demo(ui: &mut egui::Ui) {
/// let out = InteractionTag::new(Tag::new("Status: open").dismissible(true)).show(ui);
/// if out.clicked { /* edit the filter */ }
/// if out.dismissed { /* drop it */ }
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct InteractionTag {
    tag: Tag,
    selected: bool,
}

impl InteractionTag {
    /// Make `tag` interactive. Its `dismissible` setting adds the dismiss target.
    #[must_use]
    pub fn new(tag: Tag) -> Self {
        Self { tag, selected: false }
    }

    /// Show the primary part in the brand selected style (for a toggled filter).
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Show the tag.
    pub fn show(self, ui: &mut Ui) -> TagResponse {
        show_tag(
            ui,
            &self.tag,
            Options {
                interactive: true,
                selected: self.selected,
                ..Options::default()
            },
        )
    }
}

impl Widget for InteractionTag {
    fn ui(self, ui: &mut Ui) -> Response {
        self.show(ui).response
    }
}

/// What [`TagGroup::show`] did this frame.
#[derive(Debug)]
pub struct TagGroupResponse {
    /// The group (the AccessKit List).
    pub response: Response,
    /// The index of the tag whose primary action was activated.
    pub clicked: Option<usize>,
    /// The index of the tag that was dismissed. The app removes it.
    pub dismissed: Option<usize>,
}

/// A Fluent TagGroup: a wrapping row of tags with roving focus. See the module docs.
///
/// ```
/// # use fluent2_egui::controls::{Tag, TagGroup};
/// # fn demo(ui: &mut egui::Ui, filters: &mut Vec<String>) {
/// let tags = filters.iter().map(|f| Tag::new(f.clone())).collect();
/// let out = TagGroup::new("filters", tags).label("Filters").dismissible(true).show(ui);
/// if let Some(index) = out.dismissed {
///     filters.remove(index);
/// }
/// # }
/// ```
#[derive(Debug)]
pub struct TagGroup {
    id_salt: Id,
    tags: Vec<Tag>,
    label: Option<String>,
    interactive: bool,
    dismissible: Option<bool>,
    appearance: Option<TagAppearance>,
    size: Option<TagSize>,
}

impl TagGroup {
    /// A group of `tags`, keyed by `id_salt`.
    #[must_use]
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, tags: Vec<Tag>) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            tags,
            label: None,
            interactive: false,
            dismissible: None,
            appearance: None,
            size: None,
        }
    }

    /// The group's accessible name.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Show every tag as an [`InteractionTag`].
    #[must_use]
    pub fn interactive(mut self, interactive: bool) -> Self {
        self.interactive = interactive;
        self
    }

    /// Give every tag a dismiss button (or none), overriding each tag's own setting.
    #[must_use]
    pub fn dismissible(mut self, dismissible: bool) -> Self {
        self.dismissible = Some(dismissible);
        self
    }

    /// One appearance for every tag.
    #[must_use]
    pub fn appearance(mut self, appearance: TagAppearance) -> Self {
        self.appearance = Some(appearance);
        self
    }

    /// One size for every tag.
    #[must_use]
    pub fn size(mut self, size: TagSize) -> Self {
        self.size = Some(size);
        self
    }

    /// Show the group.
    pub fn show(mut self, ui: &mut Ui) -> TagGroupResponse {
        let id = ui.make_persistent_id(self.id_salt);
        let current_id = id.with("current");
        let pending_id = id.with("pending_focus");
        let count = self.tags.len();
        for tag in &mut self.tags {
            if let Some(dismissible) = self.dismissible {
                tag.dismissible = dismissible;
            }
            if let Some(appearance) = self.appearance {
                tag.appearance = appearance;
            }
            if let Some(size) = self.size {
                tag.size = size;
            }
        }
        let mut current = ui
            .ctx()
            .data(|d| d.get_temp::<usize>(current_id))
            .unwrap_or(0)
            .min(count.saturating_sub(1));
        let pending = ui.ctx().data_mut(|d| d.remove_temp::<usize>(pending_id));
        if let Some(index) = pending {
            current = index.min(count.saturating_sub(1));
        }

        let gap = spacing_horizontal::S;
        let available = ui.available_rect_before_wrap();
        let response = ui.interact(Rect::from_min_size(available.min, Vec2::ZERO), id, Sense::hover());
        // The List node must exist before the tags, so they hang under it.
        ui.ctx()
            .accesskit_node_builder(response.id, |node| node.set_role(egui::accesskit::Role::List));
        let mut child = ui.new_child(
            UiBuilder::new()
                .id_salt(id.with("tags"))
                .max_rect(available)
                .layout(egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true))
                .accessibility_parent(response.id),
        );
        child.spacing_mut().item_spacing = Vec2::new(gap, spacing_vertical::S);
        let mut clicked = None;
        let mut dismissed = None;
        let mut focused = None;
        let mut targets = Vec::with_capacity(count);
        for (index, tag) in self.tags.iter().enumerate() {
            let out = show_tag(
                &mut child,
                tag,
                Options {
                    interactive: self.interactive,
                    in_group: true,
                    tab_stop: index == current,
                    id: Some(id.with(("tag", index))),
                    ..Options::default()
                },
            );
            if out.clicked {
                clicked = Some(index);
            }
            if out.dismissed && dismissed.is_none() {
                dismissed = Some(index);
            }
            let primary = if self.interactive {
                Some(out.response.id)
            } else {
                out.dismiss.as_ref().map(|d| d.id)
            };
            if out.response.has_focus() || out.dismiss.as_ref().is_some_and(Response::has_focus) {
                focused = Some(index);
            }
            if pending == Some(index)
                && let Some(target) = primary
            {
                ui.memory_mut(|m| m.request_focus(target));
            }
            targets.push(primary);
        }
        let used = child.min_rect();
        ui.advance_cursor_after_rect(used);
        let response = response.with_new_rect(used);

        if let Some(index) = focused {
            current = index;
            // The arrow keys move between tags, not egui's spatial focus.
            if let Some(target) = targets.get(index).copied().flatten() {
                ui.memory_mut(|m| {
                    m.set_focus_lock_filter(
                        target,
                        EventFilter {
                            horizontal_arrows: true,
                            vertical_arrows: true,
                            ..EventFilter::default()
                        },
                    );
                });
            }
            let (next, prev, home, end) = ui.input(|i| {
                (
                    i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::ArrowDown),
                    i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::ArrowUp),
                    i.key_pressed(Key::Home),
                    i.key_pressed(Key::End),
                )
            });
            let to = if next {
                Some((index + 1).min(count.saturating_sub(1)))
            } else if prev {
                Some(index.saturating_sub(1))
            } else if home {
                Some(0)
            } else if end {
                Some(count.saturating_sub(1))
            } else {
                None
            };
            if let Some(to) = to
                && to != index
                && let Some(target) = targets.get(to).copied().flatten()
            {
                ui.memory_mut(|m| m.request_focus(target));
                current = to;
            }
        }
        if let Some(index) = dismissed {
            // After the app removes the tag, the next one (now at `index`) takes focus; the
            // last tag hands it to the one before.
            let next = if index + 1 < count {
                index
            } else {
                index.saturating_sub(1)
            };
            if count > 1 {
                ui.ctx().data_mut(|d| d.insert_temp(pending_id, next));
            }
            current = next;
        }
        ui.ctx().data_mut(|d| d.insert_temp(current_id, current));

        let label = self.label.clone();
        let enabled = ui.is_enabled();
        response.widget_info(|| match &label {
            Some(label) => WidgetInfo::labeled(WidgetType::Other, enabled, label.as_str()),
            None => WidgetInfo::new(WidgetType::Other),
        });
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::List);
            if let Some(label) = &label {
                node.set_label(label.as_str());
            }
        });
        TagGroupResponse {
            response,
            clicked,
            dismissed,
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct Options {
    interactive: bool,
    selected: bool,
    in_group: bool,
    tab_stop: bool,
    id: Option<Id>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            interactive: false,
            selected: false,
            in_group: false,
            tab_stop: true,
            id: None,
        }
    }
}

struct Metrics {
    height: f32,
    pad: f32,
    media: f32,
    ramp: TypeRamp,
}

fn metrics(size: TagSize) -> Metrics {
    match size {
        TagSize::ExtraSmall => Metrics {
            height: 20.0,
            pad: spacing_horizontal::S_NUDGE,
            media: 12.0,
            ramp: TypeRamp::Caption1,
        },
        TagSize::Small => Metrics {
            height: 24.0,
            pad: spacing_horizontal::S_NUDGE,
            media: 16.0,
            ramp: TypeRamp::Caption1,
        },
        TagSize::Medium => Metrics {
            height: 32.0,
            pad: spacing_horizontal::S,
            media: 20.0,
            ramp: TypeRamp::Body1,
        },
    }
}

/// Fill, ink, secondary ink and border for one part of a tag.
fn colors(theme: &Theme, appearance: TagAppearance, selected: bool, state: (bool, bool)) -> [Rgba; 4] {
    let color = &theme.color;
    let (hovered, pressed) = state;
    if selected {
        let fill = if pressed {
            color.brand_background_pressed
        } else if hovered {
            color.brand_background_hover
        } else {
            color.brand_background
        };
        let ink = color.neutral_foreground_on_brand;
        return [fill, ink, ink, color.transparent_stroke];
    }
    match appearance {
        TagAppearance::Filled => [
            if pressed {
                color.neutral_background_3_pressed
            } else if hovered {
                color.neutral_background_3_hover
            } else {
                color.neutral_background_3
            },
            color.neutral_foreground_1,
            color.neutral_foreground_2,
            color.transparent_stroke,
        ],
        TagAppearance::Outline => [
            if pressed {
                color.subtle_background_pressed
            } else if hovered {
                color.subtle_background_hover
            } else {
                color.neutral_background_1
            },
            color.neutral_foreground_1,
            color.neutral_foreground_2,
            if hovered {
                color.neutral_stroke_1_hover
            } else {
                color.neutral_stroke_1
            },
        ],
        TagAppearance::Brand => [
            if pressed {
                color.brand_background_2_pressed
            } else if hovered {
                color.brand_background_2_hover
            } else {
                color.brand_background_2
            },
            color.brand_foreground_2,
            color.brand_foreground_2,
            color.transparent_stroke,
        ],
    }
}

fn show_tag(ui: &mut Ui, tag: &Tag, options: Options) -> TagResponse {
    crate::controls::ensure_type_ramp(ui);
    let theme = Theme::get(ui.ctx());
    let enabled = ui.is_enabled();
    let m = metrics(tag.size);
    let height = crate::theme::density_of(ui).height(m.height);
    let style = TextStyle::Name(m.ramp.name().into());
    let secondary_style = TextStyle::Name(TypeRamp::Caption1.name().into());
    let stacked = tag.size == TagSize::Medium && tag.secondary.is_some();
    let primary = WidgetText::from(tag.text.clone()).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        320.0,
        style.clone(),
    );
    let secondary = tag.secondary.as_ref().map(|s| {
        WidgetText::from(s.clone()).into_galley(
            ui,
            Some(egui::TextWrapMode::Truncate),
            320.0,
            if stacked { secondary_style } else { style },
        )
    });
    let text_width = match &secondary {
        Some(s) if stacked => primary.size().x.max(s.size().x),
        Some(s) => primary.size().x + spacing_horizontal::XS + s.size().x,
        None => primary.size().x,
    };
    let media_width = if tag.media.is_some() {
        m.media + spacing_horizontal::XS
    } else {
        0.0
    };
    let dismiss_width = if tag.dismissible {
        m.media + 2.0 * spacing_horizontal::XS
    } else {
        0.0
    };
    let body_width = m.pad + media_width + text_width + if tag.dismissible { 0.0 } else { m.pad };
    let width = body_width + dismiss_width;

    let (rect, alloc) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
    let id = options.id.unwrap_or_else(|| alloc.id.with("tag"));
    let body_rect = Rect::from_min_size(rect.min, Vec2::new(body_width, height));
    let dismiss_rect = Rect::from_min_max(Pos2::new(body_rect.right(), rect.top()), rect.max);
    let focus_sense = |sense: Sense| {
        if options.tab_stop {
            sense | Sense::FOCUSABLE
        } else {
            sense - Sense::FOCUSABLE
        }
    };
    let body_sense = if options.interactive {
        focus_sense(Sense::click())
    } else {
        Sense::hover()
    };
    let body = ui.interact(body_rect, id, body_sense);
    let dismiss = tag
        .dismissible
        .then(|| ui.interact(dismiss_rect, id.with("dismiss"), focus_sense(Sense::click())));

    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    let pressed = |r: &Response| r.is_pointer_button_down_on() && enabled;
    let hovered = |r: &Response| r.hovered() && enabled;
    let [fill, ink, ink_2, border] = colors(&theme, tag.appearance, false, (false, false));
    let painter = ui.painter().clone();
    if ui.is_rect_visible(rect) {
        painter.rect_filled(rect, radius, c(fill));
        if options.interactive {
            let [fill, _, _, _] = colors(
                &theme,
                tag.appearance,
                options.selected,
                (hovered(&body), pressed(&body)),
            );
            let corners = if tag.dismissible {
                CornerRadius {
                    ne: 0,
                    se: 0,
                    ..radius
                }
            } else {
                radius
            };
            painter.rect_filled(body_rect, corners, c(fill));
        }
        let (ink, ink_2) = if options.interactive && options.selected {
            let [_, a, b, _] = colors(&theme, tag.appearance, true, (false, false));
            (a, b)
        } else {
            (ink, ink_2)
        };
        let border = if theme.kind == ThemeKind::HighContrast {
            theme.color.neutral_stroke_accessible
        } else {
            border
        };
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(stroke_width::THIN, c(border)),
            StrokeKind::Inside,
        );

        let mut x = body_rect.left() + m.pad;
        match &tag.media {
            Some(Media::Icon(icon)) => {
                let icon_rect = Rect::from_center_size(
                    Pos2::new(x + m.media / 2.0, rect.center().y),
                    Vec2::splat(m.media),
                );
                let size = if m.media <= 16.0 {
                    IconSize::S16
                } else {
                    IconSize::S20
                };
                paint_icon(ui, icon_rect, *icon, size, IconStyle::Regular, Some(c(ink)));
                x += media_width;
            }
            Some(Media::Avatar(avatar)) => {
                let avatar_rect = Rect::from_center_size(
                    Pos2::new(x + m.media / 2.0, rect.center().y),
                    Vec2::splat(m.media),
                );
                ui.put(avatar_rect, avatar.clone().size(m.media));
                x += media_width;
            }
            None => {}
        }
        match &secondary {
            Some(s) if stacked => {
                let total = primary.size().y + s.size().y;
                let top = rect.center().y - total / 2.0;
                let primary_height = primary.size().y;
                painter.galley(Pos2::new(x, top), primary, c(ink));
                painter.galley(Pos2::new(x, top + primary_height), s.clone(), c(ink_2));
            }
            Some(s) => {
                let primary_width = primary.size().x;
                painter.galley(
                    Pos2::new(x, rect.center().y - primary.size().y / 2.0),
                    primary,
                    c(ink),
                );
                let x2 = x + primary_width + spacing_horizontal::XS;
                painter.galley(
                    Pos2::new(x2, rect.center().y - s.size().y / 2.0),
                    s.clone(),
                    c(ink_2),
                );
            }
            None => {
                painter.galley(
                    Pos2::new(x, rect.center().y - primary.size().y / 2.0),
                    primary,
                    c(ink),
                );
            }
        }
        if let Some(d) = &dismiss {
            if options.interactive {
                let [fill, _, _, _] = colors(&theme, tag.appearance, false, (hovered(d), pressed(d)));
                painter.rect_filled(
                    dismiss_rect,
                    CornerRadius {
                        nw: 0,
                        sw: 0,
                        ..radius
                    },
                    c(fill),
                );
                painter.vline(
                    dismiss_rect.left(),
                    rect.y_range().shrink(spacing_vertical::XS),
                    Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_1)),
                );
            } else if hovered(d) {
                painter.rect_filled(
                    dismiss_rect.shrink(2.0),
                    radius,
                    c(theme.color.subtle_background_hover),
                );
            }
            let icon_rect = Rect::from_center_size(dismiss_rect.center(), Vec2::splat(m.media));
            let size = if m.media <= 16.0 {
                IconSize::S16
            } else {
                IconSize::S20
            };
            let dismiss_ink = if options.interactive && options.selected {
                ink
            } else {
                colors(&theme, tag.appearance, false, (false, false))[1]
            };
            paint_icon(
                ui,
                icon_rect,
                IconName::Dismiss,
                size,
                IconStyle::Regular,
                Some(c(dismiss_ink)),
            );
        }
    }

    // Focus rings and keyboard.
    if focus_visible(ui, &body) {
        paint_focus_ring(ui, if tag.dismissible { body_rect } else { rect }, radius);
    }
    if let Some(d) = &dismiss
        && focus_visible(ui, d)
    {
        paint_focus_ring(ui, dismiss_rect, radius);
    }
    let delete = |r: &Response| {
        r.has_focus() && ui.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace))
    };
    let dismissed = enabled
        && tag.dismissible
        && (dismiss.as_ref().is_some_and(|d| d.clicked() || delete(d))
            || (options.interactive && delete(&body)));
    let clicked = enabled && options.interactive && body.clicked();

    // AccessKit: the body is a Button (interactive) or a ListItem in a group; the dismiss
    // target is a Button named `"Remove <text>"`.
    let text = tag.accessible_text();
    if options.interactive {
        let name = text.clone();
        body.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, name.clone()));
        if options.selected {
            ui.ctx().accesskit_node_builder(body.id, |node| {
                node.set_toggled(egui::accesskit::Toggled::True);
            });
        }
    } else {
        let name = text.clone();
        let in_group = options.in_group;
        ui.ctx().accesskit_node_builder(body.id, |node| {
            node.set_role(if in_group {
                egui::accesskit::Role::ListItem
            } else {
                egui::accesskit::Role::Label
            });
            node.set_label(name);
        });
    }
    if let Some(d) = &dismiss {
        let name = format!("Remove {}", tag.text);
        d.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, name.clone()));
    }
    TagResponse {
        response: body,
        dismiss,
        clicked,
        dismissed,
    }
}

fn rounded_u8(value: f32) -> u8 {
    if value.is_finite() {
        value.round().clamp(0.0, 255.0) as u8
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_follow_fluent() {
        assert_eq!(metrics(TagSize::ExtraSmall).height, 20.0);
        assert_eq!(metrics(TagSize::Small).height, 24.0);
        assert_eq!(metrics(TagSize::Medium).height, 32.0);
    }

    #[test]
    fn accessible_text_includes_secondary() {
        assert_eq!(Tag::new("Ada").secondary("Owner").accessible_text(), "Ada, Owner");
        assert_eq!(Tag::new("Ada").accessible_text(), "Ada");
    }
}
