//! Fluent document TabStrip: closable, reorderable tabs with an overflow menu.
//!
//! Where [`TabList`](super::TabList) switches between fixed views, a TabStrip holds the
//! documents an app has open. Each tab can carry a close button and a dirty (unsaved) marker,
//! can be dragged to a new place, and tabs that do not fit sit behind an overflow button that
//! opens a menu of every tab.
//!
//! The strip never changes the app's documents. [`TabStrip::show`] writes the selection and
//! reports a close or a move in [`TabStripResponse`]; [`TabStripResponse::apply`] applies them
//! to the app's own list.
//!
//! Keyboard, with the strip focused (one tab stop): Left/Right and Home/End select, Ctrl+Shift
//! Left/Right move the selected tab, Ctrl+Tab and Ctrl+Shift+Tab (or Ctrl+PageDown/PageUp)
//! cycle, Ctrl+W (Cmd+W) closes. [`TabStrip::shortcuts`] makes the cycle and close keys work
//! wherever focus is. A middle click on a tab closes it.
//!
//! Behaviour reference: fluent-gpui `crates/fluent-layout/src/tab_strip.rs` (close and the
//! selection fix-up after a close). Reorder, overflow and the dirty marker are new here; Zed's
//! pane tabs were a design reference only. No code is copied.

use egui::epaint::RectShape;
use egui::{
    CornerRadius, Id, Key, Modifiers, PointerButton, Popup, Rect, Response, Sense, Ui, Vec2, WidgetInfo,
    WidgetText, WidgetType, accesskit,
};

use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::controls::button;
use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::icon::paint_icon;
use crate::foundation::text::TypeRamp;
use crate::theme::{Theme, c};

/// Height of the strip and of every tab.
const TAB_HEIGHT: f32 = 32.0;
/// Horizontal padding inside a tab.
const PADDING: f32 = spacing_horizontal::M;
/// Gap between icon, label and close button.
const GAP: f32 = spacing_horizontal::S;
/// Leading icon box.
const ICON: f32 = 16.0;
/// Close button box.
const CLOSE: f32 = 20.0;
/// Diameter of the dirty marker.
const DIRTY_DOT: f32 = 8.0;
/// Longest label before it is truncated.
const MAX_LABEL: f32 = 200.0;
/// Width of the overflow button.
const OVERFLOW: f32 = 32.0;
/// Text step of every tab. One step for all tabs keeps widths still while dragging.
const TEXT: TypeRamp = TypeRamp::Body1;

/// One document tab in a [`TabStrip`].
///
/// ```
/// # use fluent2_egui::surfaces::StripTab;
/// let tab = StripTab::new("notes.md").dirty(true);
/// ```
#[derive(Clone, Debug)]
pub struct StripTab {
    label: String,
    icon: Option<Icon>,
    closable: bool,
    dirty: bool,
}

impl StripTab {
    /// A closable tab labelled `label`. The label is also the accessible name.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            closable: true,
            dirty: false,
        }
    }

    /// Leading icon, drawn at 16 px.
    #[must_use]
    pub fn icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    /// Whether the tab has a close button and answers Ctrl+W and middle click. Default `true`.
    #[must_use]
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// Show the unsaved-changes dot. The close button replaces it on hover.
    #[must_use]
    pub fn dirty(mut self, dirty: bool) -> Self {
        self.dirty = dirty;
        self
    }
}

/// A tab moved from one index to another.
///
/// `to` is the tab's index after the move, as `Vec::remove(from)` then `Vec::insert(to, _)`
/// would leave it.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TabMove {
    /// Index before the move.
    pub from: usize,
    /// Index after the move.
    pub to: usize,
}

/// What a [`TabStrip`] did this frame.
#[derive(Debug)]
pub struct TabStripResponse {
    /// The strip's focusable response.
    pub response: Response,
    /// The tab selected this frame, if the selection changed. Already written to the
    /// strip's `selected`.
    pub selected: Option<usize>,
    /// The tab the user asked to close (close button, middle click or Ctrl+W).
    pub closed: Option<usize>,
    /// A drag or Ctrl+Shift+Arrow moved a tab.
    pub moved: Option<TabMove>,
    /// The tab being dragged this frame, if any. A container (such as a dock) uses this to
    /// show where the tab could go outside the strip.
    pub dragging: Option<usize>,
    /// A tab drag that was released outside the strip: the tab and the release position.
    /// The strip does nothing with it; a container may move the tab there.
    pub dropped_outside: Option<(usize, egui::Pos2)>,
    /// The visible tabs this frame, as (tab index, rect) in strip order. A container uses
    /// these to turn a drop position over the strip into an insertion index.
    pub tab_rects: Vec<(usize, Rect)>,
}

impl TabStripResponse {
    /// Apply [`Self::moved`] and [`Self::closed`] to the app's tab data and selection.
    ///
    /// The selection follows a moved tab. After a close it stays on the same document when
    /// that document is still open, and otherwise moves to the tab now at the closed index,
    /// or to the last tab. Out-of-range indices are ignored.
    ///
    /// ```
    /// # use fluent2_egui::surfaces::TabStripResponse;
    /// # fn demo(r: &TabStripResponse, docs: &mut Vec<String>, selected: &mut usize) {
    /// r.apply(docs, selected);
    /// # }
    /// ```
    pub fn apply<T>(&self, items: &mut Vec<T>, selected: &mut usize) {
        if let Some(moved) = self.moved {
            apply_move(items, selected, moved);
        }
        if let Some(closed) = self.closed {
            apply_close(items, selected, closed);
        }
    }
}

/// Move `items[from]` to `to`, keeping `selected` on the same item.
fn apply_move<T>(items: &mut Vec<T>, selected: &mut usize, moved: TabMove) {
    let TabMove { from, to } = moved;
    if from >= items.len() || to >= items.len() || from == to {
        return;
    }
    let item = items.remove(from);
    items.insert(to, item);
    let s = *selected;
    *selected = if s == from {
        to
    } else if from < s && s <= to {
        s - 1
    } else if to <= s && s < from {
        s + 1
    } else {
        s
    };
}

/// Remove `items[closed]` and fix the selection.
fn apply_close<T>(items: &mut Vec<T>, selected: &mut usize, closed: usize) {
    if closed >= items.len() {
        return;
    }
    items.remove(closed);
    if closed < *selected {
        *selected -= 1;
    }
    if *selected >= items.len() {
        *selected = items.len().saturating_sub(1);
    }
}

/// A strip of document tabs with close, drag-to-reorder, overflow and a dirty marker.
///
/// ```
/// # use fluent2_egui::surfaces::{StripTab, TabStrip};
/// # fn demo(ui: &mut egui::Ui, docs: &mut Vec<(String, bool)>, selected: &mut usize) {
/// let tabs = docs.iter().map(|(name, dirty)| StripTab::new(name.clone()).dirty(*dirty));
/// let response = TabStrip::new("documents", selected).tabs(tabs).show(ui);
/// response.apply(docs, selected);
/// # }
/// ```
pub struct TabStrip<'a> {
    id_salt: Id,
    selected: &'a mut usize,
    tabs: Vec<StripTab>,
    reorderable: bool,
    shortcuts: bool,
    label: String,
}

impl<'a> TabStrip<'a> {
    /// A strip whose selection lives in `selected`.
    ///
    /// `id_salt` is the widget id. It is never derived from a tab label.
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, selected: &'a mut usize) -> Self {
        Self {
            id_salt: Id::new(id_salt),
            selected,
            tabs: Vec::new(),
            reorderable: true,
            shortcuts: false,
            label: "Open documents".to_owned(),
        }
    }

    /// Append a tab.
    #[must_use]
    pub fn tab(mut self, tab: StripTab) -> Self {
        self.tabs.push(tab);
        self
    }

    /// Append several tabs.
    #[must_use]
    pub fn tabs(mut self, tabs: impl IntoIterator<Item = StripTab>) -> Self {
        self.tabs.extend(tabs);
        self
    }

    /// Allow drag and Ctrl+Shift+Arrow to move tabs. Default `true`.
    #[must_use]
    pub fn reorderable(mut self, reorderable: bool) -> Self {
        self.reorderable = reorderable;
        self
    }

    /// Answer Ctrl+Tab, Ctrl+Shift+Tab and Ctrl+W even when the strip is not focused.
    ///
    /// Turn this on for the one strip that owns the window's documents. Default `false`.
    #[must_use]
    pub fn shortcuts(mut self, shortcuts: bool) -> Self {
        self.shortcuts = shortcuts;
        self
    }

    /// Accessible name of the strip. Default "Open documents".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Show the strip across the available width.
    pub fn show(self, ui: &mut Ui) -> TabStripResponse {
        let theme = Theme::get(ui.ctx());
        button::ensure_type_ramp(ui);
        let id = ui.make_persistent_id(self.id_salt);
        let enabled = ui.is_enabled();
        let count = self.tabs.len();
        let shown = if count == 0 {
            0
        } else {
            (*self.selected).min(count - 1)
        };

        let galleys: Vec<_> = self
            .tabs
            .iter()
            .map(|tab| {
                WidgetText::from(tab.label.as_str()).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Truncate),
                    MAX_LABEL,
                    egui::TextStyle::Name(TEXT.name().into()),
                )
            })
            .collect();
        let widths: Vec<f32> = self
            .tabs
            .iter()
            .zip(&galleys)
            .map(|(tab, galley)| tab_width(tab, galley.size().x))
            .collect();

        let tab_height = crate::theme::density_of(ui).height(TAB_HEIGHT);
        let available = finite(ui.available_width());
        let (rect, mut response) = ui.allocate_exact_size(
            Vec2::new(available, tab_height),
            Sense::focusable_noninteractive(),
        );

        // Keyboard and shortcuts. Work out the target selection, close and move first; the
        // layout below then keeps the selected tab in view.
        let mut next = shown;
        let mut closed = None;
        let mut moved = None;
        if enabled && count > 0 {
            let focused = response.has_focus();
            let keys = read_keys(ui, focused, focused || self.shortcuts);
            if let Some(step) = keys.select {
                next = step.apply(next, count);
            }
            if let Some(step) = keys.cycle {
                next = step.apply_wrapping(next, count);
            }
            if self.reorderable
                && let Some(step) = keys.shift
            {
                let to = step.apply(next, count);
                // The selection index stays on `from`; `apply` carries it to `to`.
                if to != next {
                    moved = Some(TabMove { from: next, to });
                }
            }
            if keys.close && self.tabs.get(next).is_some_and(|tab| tab.closable) {
                closed = Some(next);
            }
        }

        let overflowing = widths.iter().sum::<f32>() > available;
        let lane = if overflowing {
            (available - OVERFLOW).max(0.0)
        } else {
            available
        };
        let first_id = id.with("first");
        let stored_first = ui.data(|data| data.get_temp::<usize>(first_id)).unwrap_or(0);
        let (first, end) = visible_range(&widths, lane, stored_first, next);
        ui.data_mut(|data| data.insert_temp(first_id, first));

        let mut layout = Vec::with_capacity(end.saturating_sub(first));
        let mut x = rect.min.x;
        for width in widths.get(first..end).unwrap_or(&[]) {
            layout.push(Rect::from_min_size(
                egui::pos2(x, rect.min.y),
                Vec2::new(*width, tab_height),
            ));
            x += width;
        }

        let tab_rects: Vec<(usize, Rect)> = layout
            .iter()
            .enumerate()
            .map(|(offset, rect)| (first + offset, *rect))
            .collect();

        // Pointer: tabs, then their close buttons, so a close button sits above its tab.
        let drag_id = id.with("drag");
        // (dragged tab, last drop index seen, last pointer position). Both are kept so a
        // release with no pointer position still lands where the pointer last was.
        let mut drag: Option<(usize, usize, Option<egui::Pos2>)> = ui.data(|data| data.get_temp(drag_id));
        let mut drop_at = None;
        let mut tab_responses = Vec::with_capacity(layout.len());
        for (offset, tab_rect) in layout.iter().enumerate() {
            let index = first + offset;
            let sense = if self.reorderable {
                Sense::CLICK | Sense::DRAG
            } else {
                Sense::CLICK
            };
            let tab_response = ui.interact(*tab_rect, id.with(("tab", index)), sense);
            if enabled {
                if tab_response.clicked() || tab_response.drag_started() {
                    next = index;
                    response.request_focus();
                }
                if tab_response.clicked_by(PointerButton::Middle)
                    && self.tabs.get(index).is_some_and(|tab| tab.closable)
                {
                    closed = Some(index);
                }
                if self.reorderable && tab_response.drag_started() {
                    drag = Some((index, index, None));
                }
            }
            tab_responses.push(tab_response);
        }
        let mut close_responses = Vec::with_capacity(layout.len());
        for (offset, tab_rect) in layout.iter().enumerate() {
            let index = first + offset;
            let Some(tab) = self.tabs.get(index).filter(|tab| tab.closable) else {
                close_responses.push(None);
                continue;
            };
            let close_rect = close_rect(*tab_rect);
            let close = ui.interact(close_rect, id.with(("close", index)), Sense::CLICK);
            if enabled && close.clicked() {
                closed = Some(index);
            }
            let label = format!("Close {}", tab.label);
            close.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &label));
            close_responses.push(Some(close));
        }

        let mut dragging = None;
        let mut dropped_outside = None;
        if let Some((from, last, last_pos)) = drag {
            let pointer = ui.input(|input| input.pointer.latest_pos()).or(last_pos);
            let target = pointer.map_or(last, |pos| drop_index(&layout, first, from, pos.x));
            let outside = pointer.filter(|pos| !rect.contains(*pos));
            if ui.input(|input| input.pointer.primary_down()) {
                dragging = Some(from);
                if outside.is_none() {
                    drop_at = Some(target);
                }
                ui.data_mut(|data| data.insert_temp(drag_id, (from, target, pointer)));
            } else {
                if let Some(pos) = outside {
                    if enabled {
                        dropped_outside = Some((from, pos));
                    }
                } else if target != from && enabled {
                    moved = Some(TabMove { from, to: target });
                }
                ui.data_mut(|data| data.remove::<(usize, usize, Option<egui::Pos2>)>(drag_id));
            }
        }

        // Overflow menu: every tab, so hidden ones can be reached.
        let mut overflow_response = None;
        if overflowing {
            let button_rect = Rect::from_min_size(
                egui::pos2(rect.max.x - OVERFLOW, rect.min.y),
                Vec2::new(OVERFLOW.max(tab_height), tab_height),
            );
            let more = ui.interact(button_rect, id.with("overflow"), Sense::click());
            more.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, "More tabs"));
            let popup_id = Popup::default_response_id(&more);
            let open_key = more.has_focus()
                && ui.input_mut(|input| {
                    input.consume_key(Modifiers::NONE, Key::Enter)
                        || input.consume_key(Modifiers::NONE, Key::Space)
                });
            if enabled && (more.clicked() || open_key) {
                Popup::toggle_id(ui.ctx(), popup_id);
            }
            let open = Popup::is_id_open(ui.ctx(), popup_id);
            ui.ctx()
                .accesskit_node_builder(more.id, |node| node.set_expanded(open));
            if open
                && enabled
                && let Some(inner) = Popup::menu(&more).open_memory(None).show(|popup| {
                    crate::surfaces::menu_frame(popup)
                        .show(popup, |popup| {
                            let mut chosen = None;
                            for (index, tab) in self.tabs.iter().enumerate() {
                                let text = if tab.dirty {
                                    format!("{} \u{25CF}", tab.label)
                                } else {
                                    tab.label.clone()
                                };
                                let mut item = crate::surfaces::MenuItem::new(text)
                                    .id_salt(("tab_strip_overflow", index))
                                    .check_column(true)
                                    .checked(index == next);
                                if let Some(icon) = tab.icon {
                                    item = item.icon(icon);
                                }
                                if popup.add(item).clicked() {
                                    chosen = Some(index);
                                }
                            }
                            chosen
                        })
                        .inner
                })
                && let Some(chosen) = inner.inner
            {
                next = chosen;
            }
            overflow_response = Some(more);
        }

        if count > 0 && next != *self.selected {
            *self.selected = next;
            response.mark_changed();
        }
        let selected = (count > 0 && next != shown).then_some(next);

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            painter.line_segment(
                [rect.left_bottom(), rect.right_bottom()],
                egui::Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)),
            );
            let rows = layout.iter().zip(&tab_responses).zip(&close_responses);
            for (offset, ((tab_rect, tab_response), close)) in rows.enumerate() {
                let index = first + offset;
                let (Some(tab), Some(galley)) = (self.tabs.get(index), galleys.get(index)) else {
                    continue;
                };
                let hovered = tab_response.hovered();
                let close = close.as_ref();
                let state = TabState {
                    enabled,
                    hovered,
                    selected: index == next,
                    dragged: dragging == Some(index),
                    close_hovered: close.is_some_and(Response::hovered),
                };
                paint_tab(ui, *tab_rect, tab, galley, &theme, state);
            }
            if let (Some(to), Some(from)) = (drop_at, dragging)
                && to != from
            {
                paint_drop_marker(ui, &layout, first, from, to, &theme);
            }
            if let Some(more) = &overflow_response {
                paint_overflow(ui, more, &theme, enabled);
            }
            if focus::focus_visible(ui, &response) {
                let ring = next
                    .checked_sub(first)
                    .and_then(|offset| layout.get(offset))
                    .copied()
                    .unwrap_or(rect);
                paint_focus_ring(ui, ring, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
            }
        }

        let current = self.tabs.get(next).map(|tab| tab.label.as_str()).unwrap_or("");
        response.widget_info(|| WidgetInfo::labeled(WidgetType::SelectableLabel, enabled, current));
        let strip_label = self.label.clone();
        let active = id.with(("tab", next)).accesskit_id();
        let has_active = next >= first && next < end;
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(accesskit::Role::TabList);
            node.set_label(strip_label);
            if has_active {
                node.set_active_descendant(active);
            }
        });
        for (offset, tab_response) in tab_responses.iter().enumerate() {
            let index = first + offset;
            let Some(tab) = self.tabs.get(index) else {
                continue;
            };
            let text = tab.label.clone();
            let dirty = tab.dirty;
            // Fills the node's bounds and click action; the role is set to Tab below.
            tab_response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, &text));
            ui.ctx().accesskit_node_builder(tab_response.id, |node| {
                node.set_role(accesskit::Role::Tab);
                node.set_label(text);
                node.set_selected(index == next);
                if dirty {
                    node.set_description("Modified".to_owned());
                }
                if !enabled {
                    node.set_disabled();
                }
            });
        }

        TabStripResponse {
            response,
            selected,
            closed,
            moved,
            dragging,
            dropped_outside,
            tab_rects,
        }
    }
}

/// A step the keyboard asked for.
#[derive(Clone, Copy)]
enum Step {
    Prev,
    Next,
    First,
    Last,
}

impl Step {
    fn apply(self, current: usize, count: usize) -> usize {
        let last = count.saturating_sub(1);
        match self {
            Step::Prev => current.saturating_sub(1),
            Step::Next => (current + 1).min(last),
            Step::First => 0,
            Step::Last => last,
        }
    }

    fn apply_wrapping(self, current: usize, count: usize) -> usize {
        if count == 0 {
            return 0;
        }
        match self {
            Step::Prev => (current + count - 1) % count,
            Step::Next => (current + 1) % count,
            Step::First | Step::Last => self.apply(current, count),
        }
    }
}

/// Keys read this frame.
#[derive(Default)]
struct Keys {
    select: Option<Step>,
    cycle: Option<Step>,
    shift: Option<Step>,
    close: bool,
}

/// Consume the strip's keys. Arrows need focus; cycle and close need `global`.
fn read_keys(ui: &Ui, focused: bool, global: bool) -> Keys {
    let mut keys = Keys::default();
    ui.input_mut(|input| {
        if focused {
            let move_mods = Modifiers::COMMAND | Modifiers::SHIFT;
            if input.consume_key(move_mods, Key::ArrowLeft) {
                keys.shift = Some(Step::Prev);
            } else if input.consume_key(move_mods, Key::ArrowRight) {
                keys.shift = Some(Step::Next);
            } else if input.consume_key(Modifiers::NONE, Key::ArrowLeft) {
                keys.select = Some(Step::Prev);
            } else if input.consume_key(Modifiers::NONE, Key::ArrowRight) {
                keys.select = Some(Step::Next);
            } else if input.consume_key(Modifiers::NONE, Key::Home) {
                keys.select = Some(Step::First);
            } else if input.consume_key(Modifiers::NONE, Key::End) {
                keys.select = Some(Step::Last);
            }
        }
        if global {
            // Ctrl, not Cmd, for the cycle: Cmd+Tab belongs to macOS.
            let back = Modifiers::CTRL | Modifiers::SHIFT;
            if input.consume_key(back, Key::Tab) || input.consume_key(Modifiers::CTRL, Key::PageUp) {
                keys.cycle = Some(Step::Prev);
            } else if input.consume_key(Modifiers::CTRL, Key::Tab)
                || input.consume_key(Modifiers::CTRL, Key::PageDown)
            {
                keys.cycle = Some(Step::Next);
            }
            keys.close = input.consume_key(Modifiers::COMMAND, Key::W);
        }
    });
    keys
}

/// Width of a tab with a label `label_w` wide.
fn tab_width(tab: &StripTab, label_w: f32) -> f32 {
    let icon = if tab.icon.is_some() { ICON + GAP } else { 0.0 };
    let trailing = if tab.closable || tab.dirty {
        GAP + CLOSE
    } else {
        0.0
    };
    finite(PADDING * 2.0 + icon + finite(label_w) + trailing)
}

/// The run of tabs `[first, end)` that fits in `lane`, containing `selected` and starting as
/// near `preferred` as it can. All tabs show when they fit.
fn visible_range(widths: &[f32], lane: f32, preferred: usize, selected: usize) -> (usize, usize) {
    let count = widths.len();
    if count == 0 {
        return (0, 0);
    }
    let selected = selected.min(count - 1);
    let fits_to = |first: usize| {
        let mut used = 0.0;
        let mut end = first;
        while end < count {
            let w = finite(widths.get(end).copied().unwrap_or(0.0));
            // The first tab always shows, even when it alone is wider than the lane.
            if end > first && used + w > lane {
                break;
            }
            used += w;
            end += 1;
        }
        end
    };
    let mut first = preferred.min(selected);
    while fits_to(first) <= selected {
        first += 1;
    }
    // Pull earlier tabs back in while they still fit with the selected one.
    while first > 0 && fits_to(first - 1) > selected && fits_to(first - 1) >= fits_to(first) {
        first -= 1;
    }
    (first, fits_to(first))
}

/// Final index of the dragged tab `from` when dropped at `x`.
fn drop_index(layout: &[Rect], first: usize, from: usize, x: f32) -> usize {
    let before = layout
        .iter()
        .enumerate()
        .filter(|(offset, rect)| first + offset != from && rect.center().x < x)
        .count();
    first + before
}

fn close_rect(tab: Rect) -> Rect {
    Rect::from_center_size(
        egui::pos2(tab.max.x - PADDING - CLOSE / 2.0, tab.center().y),
        Vec2::splat(CLOSE),
    )
}

/// Per-frame state of one tab.
#[derive(Clone, Copy)]
struct TabState {
    enabled: bool,
    hovered: bool,
    selected: bool,
    dragged: bool,
    close_hovered: bool,
}

fn paint_tab(
    ui: &Ui,
    rect: Rect,
    tab: &StripTab,
    galley: &std::sync::Arc<egui::Galley>,
    theme: &Theme,
    state: TabState,
) {
    let color = &theme.color;
    let radius = CornerRadius::same(rounded_u8(border_radius::MEDIUM));
    let fill = if state.dragged || (state.selected && state.enabled) {
        Some(color.subtle_background_selected)
    } else if state.hovered && state.enabled {
        Some(color.subtle_background_hover)
    } else {
        None
    };
    if let Some(fill) = fill {
        ui.painter().add(RectShape::filled(rect, radius, c(fill)));
    }
    let foreground = if !state.enabled {
        c(color.neutral_foreground_disabled)
    } else if fill.is_some() {
        c(color.neutral_foreground_1_selected)
    } else {
        c(color.neutral_foreground_2)
    };

    let mut x = rect.min.x + PADDING;
    if let Some(icon) = tab.icon {
        let icon_rect = Rect::from_min_size(egui::pos2(x, rect.center().y - ICON / 2.0), Vec2::splat(ICON));
        paint_icon(
            ui,
            icon_rect,
            icon,
            IconSize::S16,
            IconStyle::Regular,
            Some(foreground),
        );
        x += ICON + GAP;
    }
    ui.painter().galley(
        egui::pos2(x, rect.center().y - galley.size().y / 2.0),
        galley.clone(),
        foreground,
    );

    if state.selected && state.enabled {
        let thickness = stroke_width::THICKER;
        let bar = Rect::from_min_size(
            egui::pos2(rect.min.x, rect.max.y - thickness),
            Vec2::new(rect.width(), thickness),
        );
        crate::surfaces::paint_indicator(ui, bar, theme, false);
    }

    let slot = close_rect(rect);
    // A dirty tab shows its dot until the pointer is over it.
    if tab.closable && (!tab.dirty || state.hovered || state.close_hovered) {
        if state.close_hovered && state.enabled {
            ui.painter().add(RectShape::filled(
                slot,
                CornerRadius::same(rounded_u8(border_radius::SMALL)),
                c(color.subtle_background_hover),
            ));
        }
        let glyph = Rect::from_center_size(slot.center(), Vec2::splat(ICON));
        paint_icon(
            ui,
            glyph,
            Icon::Dismiss,
            IconSize::S16,
            IconStyle::Regular,
            Some(foreground),
        );
    } else if tab.dirty {
        ui.painter()
            .circle_filled(slot.center(), DIRTY_DOT / 2.0, foreground);
    }
}

/// A vertical brand bar where the dragged tab would land.
fn paint_drop_marker(ui: &Ui, layout: &[Rect], first: usize, from: usize, to: usize, theme: &Theme) {
    // Edge of the tab the dragged one would sit before or after.
    let others: Vec<&Rect> = layout
        .iter()
        .enumerate()
        .filter(|(offset, _)| first + offset != from)
        .map(|(_, rect)| rect)
        .collect();
    let slot = to.saturating_sub(first);
    let x = match others.get(slot) {
        Some(rect) => rect.min.x,
        None => match others.last() {
            Some(rect) => rect.max.x,
            None => return,
        },
    };
    let Some(top) = layout.first() else {
        return;
    };
    let thickness = stroke_width::THICKER;
    let bar = Rect::from_min_size(
        egui::pos2(x - thickness / 2.0, top.min.y),
        Vec2::new(thickness, top.height()),
    );
    crate::surfaces::paint_indicator(ui, bar, theme, false);
}

fn paint_overflow(ui: &Ui, more: &Response, theme: &Theme, enabled: bool) {
    let color = &theme.color;
    let rect = more.rect;
    let foreground = if !enabled {
        c(color.neutral_foreground_disabled)
    } else if more.hovered() {
        ui.painter().add(RectShape::filled(
            rect,
            CornerRadius::same(rounded_u8(border_radius::MEDIUM)),
            c(color.subtle_background_hover),
        ));
        c(color.neutral_foreground_1_selected)
    } else {
        c(color.neutral_foreground_2)
    };
    let glyph = Rect::from_center_size(rect.center(), Vec2::splat(ICON));
    paint_icon(
        ui,
        glyph,
        Icon::ChevronDown,
        IconSize::S16,
        IconStyle::Regular,
        Some(foreground),
    );
    if focus::focus_visible(ui, more) {
        paint_focus_ring(ui, rect, CornerRadius::same(rounded_u8(border_radius::MEDIUM)));
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_before_the_selection_keeps_the_same_document() {
        let mut docs = vec!["a", "b", "c"];
        let mut selected = 1;
        apply_close(&mut docs, &mut selected, 0);
        assert_eq!(docs[selected], "b");
    }

    #[test]
    fn closing_the_last_selected_tab_selects_the_previous() {
        let mut docs = vec!["a", "b"];
        let mut selected = 1;
        apply_close(&mut docs, &mut selected, 1);
        assert_eq!((selected, docs[selected]), (0, "a"));
    }

    #[test]
    fn closing_the_only_tab_leaves_zero() {
        let mut docs = vec!["a"];
        let mut selected = 0;
        apply_close(&mut docs, &mut selected, 0);
        assert!(docs.is_empty());
        assert_eq!(selected, 0);
    }

    #[test]
    fn moves_keep_the_selection_on_its_document() {
        for (from, to) in [(0, 3), (3, 0), (1, 2), (2, 1)] {
            for start in 0..4 {
                let mut docs = vec!["a", "b", "c", "d"];
                let mut selected = start;
                let before = docs[selected];
                apply_move(&mut docs, &mut selected, TabMove { from, to });
                assert_eq!(docs[selected], before, "move {from}->{to}, selected {start}");
            }
        }
    }

    #[test]
    fn out_of_range_edits_are_ignored() {
        let mut docs = vec!["a"];
        let mut selected = 0;
        apply_close(&mut docs, &mut selected, 5);
        apply_move(&mut docs, &mut selected, TabMove { from: 0, to: 9 });
        assert_eq!(docs, vec!["a"]);
    }

    #[test]
    fn visible_range_follows_the_selection() {
        let widths = [100.0; 10];
        assert_eq!(visible_range(&widths, 1000.0, 0, 9), (0, 10));
        assert_eq!(visible_range(&widths, 300.0, 0, 0), (0, 3));
        assert_eq!(visible_range(&widths, 300.0, 0, 5), (3, 6));
        // Moving back keeps the window where it was while the selection stays inside it.
        assert_eq!(visible_range(&widths, 300.0, 3, 4), (3, 6));
        assert_eq!(visible_range(&widths, 300.0, 3, 1), (1, 4));
        // A lane narrower than one tab still shows the selected tab.
        assert_eq!(visible_range(&widths, 10.0, 0, 2), (2, 3));
        assert_eq!(visible_range(&[f32::NAN, 100.0], 50.0, 0, 1), (1, 2));
    }

    #[test]
    fn drop_index_counts_the_tabs_left_of_the_pointer() {
        let layout: Vec<Rect> = (0..4)
            .map(|i| Rect::from_min_size(egui::pos2(i as f32 * 100.0, 0.0), Vec2::new(100.0, 32.0)))
            .collect();
        assert_eq!(drop_index(&layout, 0, 0, 260.0), 2);
        assert_eq!(drop_index(&layout, 0, 0, 390.0), 3);
        assert_eq!(drop_index(&layout, 0, 3, 10.0), 0);
        assert_eq!(drop_index(&layout, 5, 6, 160.0), 6);
    }
}
