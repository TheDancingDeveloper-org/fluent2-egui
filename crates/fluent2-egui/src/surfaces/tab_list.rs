//! Fluent TabList.
//!
//! One tab stop for the whole list. Left/Right (Up/Down when vertical) move and select;
//! Home/End jump to the first or last enabled tab. Disabled tabs are skipped.
//!
//! The selected indicator is a 3 px bar (2 px at [`TabSize::Small`]) under the tab, or to
//! its left when the list is vertical. It slides with [`Duration::Normal`] and
//! [`Curve::DecelerateMax`].
//!
//! [`tab_layout`] and [`paint_indicator`] are crate-visible so the ribbon tab strip can
//! reuse them. Metrics follow Fluent UI React TabList (the revision named on WI-1554).
//! Nothing is copied from the accessible-tabs reference.

use egui::epaint::RectShape;
use egui::{
    CornerRadius, Id, Key, Modifiers, Rect, Response, Sense, TextStyle, Ui, Vec2, WidgetInfo, WidgetText,
    WidgetType, accesskit,
};

use fluent2_icons::{Icon, Size as IconSize};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::controls::button;
use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::icon::paint_icon;
use crate::foundation::motion::{self, Curve, Duration};
use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

/// Height of a tab. [`Medium`](Self::Medium) is the Fluent default.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabSize {
    /// 24 px, 2 px indicator.
    Small,
    /// 32 px, 3 px indicator. The default.
    #[default]
    Medium,
    /// 40 px, 3 px indicator.
    Large,
}

/// Fill of an unselected tab.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TabAppearance {
    /// No fill. The default.
    #[default]
    Transparent,
    /// Subtle neutral fill on hover and selection.
    Subtle,
}

/// One tab in a [`TabList`].
struct TabSpec {
    label: String,
    icon: Option<Icon>,
    disabled: bool,
}

/// A horizontal or vertical list of tabs with a single selection.
///
/// ```
/// # use fluent2_egui::surfaces::TabList;
/// # fn demo(ui: &mut egui::Ui, selected: &mut usize) {
/// TabList::new("pages", selected).tab("Home").tab("Files").show(ui);
/// # }
/// ```
pub struct TabList<'a> {
    id_salt: Id,
    selected: &'a mut usize,
    size: TabSize,
    vertical: bool,
    appearance: TabAppearance,
    tabs: Vec<TabSpec>,
}

impl<'a> TabList<'a> {
    /// A tab list whose selection lives in `selected`.
    ///
    /// `id_salt` is the widget id. It is never derived from a tab label.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, selected: &'a mut usize) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            selected,
            size: TabSize::Medium,
            vertical: false,
            appearance: TabAppearance::Transparent,
            tabs: Vec::new(),
        }
    }

    /// Size. Defaults to [`TabSize::Medium`].
    #[must_use]
    pub fn size(mut self, size: TabSize) -> Self {
        self.size = size;
        self
    }

    /// Stack tabs vertically. Arrow keys become Up/Down.
    #[must_use]
    pub fn vertical(mut self) -> Self {
        self.vertical = true;
        self
    }

    /// Unselected fill. Defaults to [`TabAppearance::Transparent`].
    #[must_use]
    pub fn appearance(mut self, appearance: TabAppearance) -> Self {
        self.appearance = appearance;
        self
    }

    /// Append an enabled text tab.
    #[must_use]
    pub fn tab(mut self, label: impl Into<String>) -> Self {
        self.tabs.push(TabSpec {
            label: label.into(),
            icon: None,
            disabled: false,
        });
        self
    }

    /// Append an enabled tab with a leading icon.
    #[must_use]
    pub fn tab_icon(mut self, icon: Icon, label: impl Into<String>) -> Self {
        self.tabs.push(TabSpec {
            label: label.into(),
            icon: Some(icon),
            disabled: false,
        });
        self
    }

    /// Append a disabled text tab. Arrow keys and clicks skip it.
    #[must_use]
    pub fn tab_disabled(mut self, label: impl Into<String>) -> Self {
        self.tabs.push(TabSpec {
            label: label.into(),
            icon: None,
            disabled: true,
        });
        self
    }

    /// Show the list. `response.changed()` is true when the selection changed this frame.
    pub fn show(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        button::ensure_type_ramp(ui);
        let id = ui.make_persistent_id(self.id_salt);
        let enabled_parent = ui.is_enabled();
        let metrics = metrics_of(self.size);
        let count = self.tabs.len();

        // The selected tab is drawn in the strong step. The step is chosen from the selection
        // at the start of the frame, so a click or key move restyles on the next frame.
        let shown = self.selected.clamp_index(count);
        let mut galleys = Vec::with_capacity(count);
        let mut sizes = Vec::with_capacity(count);
        for (index, tab) in self.tabs.iter().enumerate() {
            let galley = WidgetText::from(tab.label.as_str()).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                metrics.text_style(index == shown),
            );
            let icon_w = if tab.icon.is_some() {
                metrics.icon + spacing_horizontal::S
            } else {
                0.0
            };
            let cross = metrics.height;
            let main = finite(metrics.padding * 2.0 + icon_w + galley.size().x);
            sizes.push(if self.vertical {
                Vec2::new(main.max(metrics.min_main), cross)
            } else {
                Vec2::new(main, cross)
            });
            galleys.push(galley);
        }

        let gap = spacing_horizontal::NONE;
        let (total_main, max_cross) = stack(&sizes, self.vertical, gap);
        let desired = if self.vertical {
            Vec2::new(finite(max_cross), finite(total_main))
        } else {
            Vec2::new(finite(total_main), finite(max_cross))
        };
        let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
        let layout = tab_layout(rect, &sizes, self.vertical, gap);

        // The selection is only written back when the user moves it, so an out-of-range
        // value is shown clamped without reporting a change.
        let before = *self.selected;
        let selected = shown;
        let mut next = selected;
        if enabled_parent {
            if let Some(clicked) = click_index(&layout, &self.tabs, &response) {
                next = clicked;
            }
            if response.has_focus()
                && let Some(moved) = keyboard_move(ui, next, self.vertical, &self.tabs)
            {
                next = moved;
            }
        }
        if count > 0 && next != before {
            *self.selected = next;
            response.mark_changed();
        }

        if ui.is_rect_visible(rect) {
            for (index, tab) in self.tabs.iter().enumerate() {
                let tab_rect = layout[index];
                let hovered = ui.rect_contains_pointer(tab_rect);
                let state = TabState {
                    enabled: enabled_parent && !tab.disabled,
                    hovered,
                    pressed: hovered && response.is_pointer_button_down_on(),
                    selected: index == selected && !tab.disabled,
                    subtle: self.appearance == TabAppearance::Subtle,
                };
                paint_tab_face(ui, tab_rect, self.appearance, &theme, state);
                paint_tab_content(ui, tab_rect, tab, &galleys[index], metrics, &theme, state);
            }
            if count > 0 && !self.tabs[selected].disabled {
                let indicator = indicator_rect(layout[selected], self.vertical, metrics.indicator);
                let animated = animate_indicator(ui, id, indicator, self.vertical);
                paint_indicator(ui, animated, &theme);
            }
            if focus::focus_visible(ui, &response) {
                let ring = layout.get(selected).copied().unwrap_or(rect);
                paint_focus_ring(ui, ring, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
            }
        }

        let label = self
            .tabs
            .get(selected)
            .map(|tab| tab.label.as_str())
            .unwrap_or("");
        response.widget_info(|| WidgetInfo::labeled(WidgetType::SelectableLabel, enabled_parent, label));
        let tab_labels: Vec<(String, bool, bool)> = self
            .tabs
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                (
                    tab.label.clone(),
                    index == selected,
                    tab.disabled || !enabled_parent,
                )
            })
            .collect();
        // The focusable strip is `response`, so it carries the TabList role; on `id`, which no
        // widget owns, the role sat on an empty node and Tab landed on an unnamed "button"
        // (WI-1586). The active descendant points screen readers at the selected tab.
        let active = id.with(("tab", selected)).accesskit_id();
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(accesskit::Role::TabList);
            if count > 0 {
                node.set_active_descendant(active);
            }
        });
        for (index, (text, is_selected, disabled)) in tab_labels.into_iter().enumerate() {
            let tab_id = id.with(("tab", index));
            ui.ctx().accesskit_node_builder(tab_id, |node| {
                node.set_role(accesskit::Role::Tab);
                node.set_label(text);
                node.set_selected(is_selected);
                if disabled {
                    node.set_disabled();
                }
            });
        }
        response
    }
}

/// Size numbers the ribbon strip needs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct TabMetrics {
    /// Tab cross-axis size in points.
    pub height: f32,
    /// Horizontal (or vertical) content padding.
    pub padding: f32,
    /// Indicator thickness.
    pub indicator: f32,
    /// Leading icon box.
    pub icon: f32,
    /// Minimum length along the list axis when vertical.
    pub min_main: f32,
    /// Text step of an unselected tab.
    text: TypeRamp,
    /// Text step of the selected tab.
    text_selected: TypeRamp,
}

impl TabMetrics {
    fn text_style(self, selected: bool) -> TextStyle {
        let step = if selected { self.text_selected } else { self.text };
        TextStyle::Name(step.name().into())
    }
}

fn metrics_of(size: TabSize) -> TabMetrics {
    match size {
        TabSize::Small => TabMetrics {
            height: 24.0,
            padding: spacing_horizontal::S,
            indicator: stroke_width::THICK,
            icon: 16.0,
            min_main: 24.0,
            text: TypeRamp::Caption1,
            text_selected: TypeRamp::Caption1,
        },
        TabSize::Medium => TabMetrics {
            height: 32.0,
            padding: spacing_horizontal::M,
            indicator: stroke_width::THICKER,
            icon: 20.0,
            min_main: 32.0,
            text: TypeRamp::Body1,
            text_selected: TypeRamp::Body1Strong,
        },
        TabSize::Large => TabMetrics {
            height: 40.0,
            padding: spacing_horizontal::L,
            indicator: stroke_width::THICKER,
            icon: 20.0,
            min_main: 40.0,
            text: TypeRamp::Body1,
            text_selected: TypeRamp::Body1Strong,
        },
    }
}

/// Main-axis width of each tab in a horizontal list, as [`TabList::show`] lays them out with
/// no icons and no gap. `selected` is the tab drawn in the strong step. The ribbon uses this
/// to place a contextual header over its tabs.
pub(crate) fn tab_widths(ui: &mut Ui, labels: &[&str], selected: usize, size: TabSize) -> Vec<f32> {
    button::ensure_type_ramp(ui);
    let metrics = metrics_of(size);
    labels
        .iter()
        .enumerate()
        .map(|(index, label)| {
            let galley = WidgetText::from(*label).into_galley(
                ui,
                Some(egui::TextWrapMode::Extend),
                f32::INFINITY,
                metrics.text_style(index == selected),
            );
            finite(metrics.padding * 2.0 + galley.size().x)
        })
        .collect()
}

/// Place each tab inside `rect`.
///
/// `sizes` are each tab's desired size. `vertical` stacks on Y. `gap` is the space between
/// tabs. Non-finite sizes are treated as zero. The ribbon tab strip calls this.
pub(crate) fn tab_layout(rect: Rect, sizes: &[Vec2], vertical: bool, gap: f32) -> Vec<Rect> {
    let gap = finite(gap);
    let mut cursor = if vertical { rect.min.y } else { rect.min.x };
    let mut out = Vec::with_capacity(sizes.len());
    for size in sizes {
        let w = finite(size.x);
        let h = finite(size.y);
        let tab = if vertical {
            Rect::from_min_size(egui::pos2(rect.min.x, cursor), Vec2::new(w.max(rect.width()), h))
        } else {
            Rect::from_min_size(egui::pos2(cursor, rect.min.y), Vec2::new(w, h.max(rect.height())))
        };
        out.push(tab);
        cursor += if vertical { h } else { w };
        cursor += gap;
    }
    out
}

/// Paint the selected indicator: a circular-radius bar in `compound_brand_stroke`.
pub(crate) fn paint_indicator(ui: &Ui, rect: Rect, theme: &Theme) {
    if !rect.is_positive() {
        return;
    }
    let radius = CornerRadius::same(rounded_u8(border_radius::CIRCULAR));
    ui.painter().add(RectShape::filled(
        rect,
        radius,
        c(theme.color.compound_brand_stroke),
    ));
}

fn indicator_rect(tab: Rect, vertical: bool, thickness: f32) -> Rect {
    let thickness = finite(thickness);
    if vertical {
        Rect::from_min_size(tab.min, Vec2::new(thickness, tab.height()))
    } else {
        Rect::from_min_size(
            egui::pos2(tab.min.x, tab.max.y - thickness),
            Vec2::new(tab.width(), thickness),
        )
    }
}

fn animate_indicator(ui: &Ui, id: Id, target: Rect, vertical: bool) -> Rect {
    let origin = if vertical { target.min.y } else { target.min.x };
    let length = if vertical { target.height() } else { target.width() };
    let thickness = if vertical { target.width() } else { target.height() };
    let animated_origin = motion::animate(
        ui.ctx(),
        id.with("indicator_origin"),
        origin,
        Duration::Normal,
        Curve::DecelerateMax,
    );
    let animated_length = motion::animate(
        ui.ctx(),
        id.with("indicator_length"),
        length,
        Duration::Normal,
        Curve::DecelerateMax,
    );
    if vertical {
        Rect::from_min_size(
            egui::pos2(target.min.x, animated_origin),
            Vec2::new(thickness, finite(animated_length)),
        )
    } else {
        Rect::from_min_size(
            egui::pos2(animated_origin, target.min.y),
            Vec2::new(finite(animated_length), thickness),
        )
    }
}

/// Per-frame state of one tab, as the painters need it.
#[derive(Clone, Copy)]
struct TabState {
    enabled: bool,
    hovered: bool,
    pressed: bool,
    selected: bool,
    /// Subtle appearance: selected and pressed tabs get a fill.
    subtle: bool,
}

fn paint_tab_face(ui: &Ui, rect: Rect, appearance: TabAppearance, theme: &Theme, state: TabState) {
    if appearance != TabAppearance::Subtle || !state.enabled {
        return;
    }
    let color = &theme.color;
    let background = if state.selected || state.pressed {
        color.subtle_background_selected
    } else if state.hovered {
        color.subtle_background_hover
    } else {
        return;
    };
    ui.painter().add(RectShape::filled(
        rect,
        CornerRadius::same(rounded_u8(border_radius::MEDIUM)),
        c(background),
    ));
}

fn paint_tab_content(
    ui: &Ui,
    rect: Rect,
    tab: &TabSpec,
    galley: &std::sync::Arc<egui::Galley>,
    metrics: TabMetrics,
    theme: &Theme,
    state: TabState,
) {
    let color = &theme.color;
    let foreground = if !state.enabled {
        c(color.neutral_foreground_disabled)
    } else if state.subtle && (state.selected || state.pressed) {
        // `paint_tab_face` fills these with the highlight colour in high contrast.
        c(color.neutral_foreground_1_selected)
    } else if state.selected {
        c(color.neutral_foreground_1)
    } else if state.hovered {
        c(color.neutral_foreground_2_hover)
    } else {
        c(color.neutral_foreground_2)
    };
    let icon_w = if tab.icon.is_some() {
        metrics.icon + spacing_horizontal::S
    } else {
        0.0
    };
    let total = icon_w + galley.size().x;
    let mut x = rect.center().x - total / 2.0;
    if let Some(icon) = tab.icon {
        let icon_size = if metrics.icon <= 16.0 {
            IconSize::S16
        } else {
            IconSize::S20
        };
        let icon_rect = Rect::from_min_size(
            egui::pos2(x, rect.center().y - metrics.icon / 2.0),
            Vec2::splat(metrics.icon),
        );
        paint_icon(
            ui,
            icon_rect,
            icon,
            icon_size,
            fluent2_icons::Style::Regular,
            Some(foreground),
        );
        x += icon_w;
    }
    let text_pos = egui::pos2(x, rect.center().y - galley.size().y / 2.0);
    ui.painter().galley(text_pos, galley.clone(), foreground);
}

fn click_index(layout: &[Rect], tabs: &[TabSpec], response: &Response) -> Option<usize> {
    if !response.clicked() {
        return None;
    }
    let pointer = response.interact_pointer_pos()?;
    let index = layout.iter().position(|rect| rect.contains(pointer))?;
    tabs.get(index).filter(|tab| !tab.disabled).map(|_| index)
}

fn keyboard_move(ui: &Ui, current: usize, vertical: bool, tabs: &[TabSpec]) -> Option<usize> {
    let (prev, next) = if vertical {
        (Key::ArrowUp, Key::ArrowDown)
    } else {
        (Key::ArrowLeft, Key::ArrowRight)
    };
    let backwards = consume(ui, prev);
    let forwards = consume(ui, next);
    let home = consume(ui, Key::Home);
    let end = consume(ui, Key::End);
    if home {
        return first_enabled(tabs, 0);
    }
    if end {
        return last_enabled(tabs);
    }
    if forwards {
        return step_enabled(tabs, current, 1);
    }
    if backwards {
        return step_enabled(tabs, current, -1);
    }
    None
}

fn consume(ui: &Ui, key: Key) -> bool {
    ui.input_mut(|input| input.consume_key(Modifiers::NONE, key))
}

fn first_enabled(tabs: &[TabSpec], from: usize) -> Option<usize> {
    (from..tabs.len()).find(|&index| !tabs[index].disabled)
}

fn last_enabled(tabs: &[TabSpec]) -> Option<usize> {
    tabs.iter().rposition(|tab| !tab.disabled)
}

fn step_enabled(tabs: &[TabSpec], current: usize, delta: isize) -> Option<usize> {
    if tabs.is_empty() {
        return None;
    }
    let mut index = current as isize;
    for _ in 0..tabs.len() {
        index += delta;
        if index < 0 || index >= tabs.len() as isize {
            return None;
        }
        if !tabs[index as usize].disabled {
            return Some(index as usize);
        }
    }
    None
}

fn stack(sizes: &[Vec2], vertical: bool, gap: f32) -> (f32, f32) {
    let mut main = 0.0;
    let mut cross: f32 = 0.0;
    for (index, size) in sizes.iter().enumerate() {
        let (m, c) = if vertical {
            (finite(size.y), finite(size.x))
        } else {
            (finite(size.x), finite(size.y))
        };
        main += m;
        if index + 1 != sizes.len() {
            main += finite(gap);
        }
        cross = cross.max(c);
    }
    (main, cross)
}

trait ClampIndex {
    fn clamp_index(self, len: usize) -> usize;
}

impl ClampIndex for usize {
    fn clamp_index(self, len: usize) -> usize {
        if len == 0 { 0 } else { self.min(len - 1) }
    }
}

fn finite(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

fn rounded_u8(value: f32) -> u8 {
    if value.is_finite() {
        value.round().clamp(0.0, f32::from(u8::MAX)) as u8
    } else {
        0
    }
}
