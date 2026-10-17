//! Ribbon tab strip (spec §4): content and contextual tabs, collapse, and the peek popup.
//!
//! The strip is a [`TabList`], so its layout, indicator, arrow keys and Home/End come from
//! the list. Tab bodies are empty placeholders until groups (WI-1563) and galleries (WI-1564)
//! arrive. The File tab and the header label of a contextual set are not built yet.

mod tab;

use std::hash::Hash;

use egui::{Area, Frame, Id, Key, Modifiers, Order, Rect, Stroke, Ui, accesskit};
use fluent2_tokens::generated::global::{border_radius, stroke_width};

use crate::surfaces::TabList;
use crate::theme::{Theme, c};

pub use tab::{ContextualAccent, RibbonTab, TabBuilder};

/// What the ribbon shows (spec §4.5).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RibbonDisplay {
    /// Tab strip and ribbon body visible.
    #[default]
    Expanded,
    /// Only the tab strip is visible.
    Collapsed,
    /// Collapsed, with the body open as a popup over the content, for one tab.
    Peek,
}

/// Display state the ribbon keeps across frames (spec §4.5). The app persists it (D3).
///
/// ```
/// # use fluent2_egui::office::ribbon::{RibbonDisplay, RibbonState};
/// let state = RibbonState::default();
/// assert_eq!(state.display, RibbonDisplay::Expanded);
/// assert_eq!(state.active, None);
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RibbonState {
    /// The active tab, by key. `None` means the first content tab.
    pub active: Option<&'static str>,
    /// Expanded, collapsed or peeking.
    pub display: RibbonDisplay,
    /// Whether the open ribbon stays open after a click outside it (pinned).
    pub pinned: bool,
}

/// What a control in the ribbon did this frame (spec §4.6). No control produces one yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RibbonEvent {
    /// The control's key.
    pub key: &'static str,
    /// What happened.
    pub kind: RibbonEventKind,
}

/// The kind of a [`RibbonEvent`] (spec §4.6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RibbonEventKind {
    /// A button, split, menu, or dialog launcher was pressed.
    Clicked,
    /// The bound state changed this frame.
    Changed,
}

/// What the ribbon reports after a frame (spec §4.6).
#[derive(Debug, Default)]
pub struct RibbonResponse {
    /// Events from controls this frame, in visual order.
    pub events: Vec<RibbonEvent>,
    /// The tab that became active this frame, if it changed.
    pub tab_changed: Option<&'static str>,
    /// Keys used more than once in this ribbon. The first occurrence keeps its tab (D4).
    pub conflicts: Vec<&'static str>,
    /// Responses of custom widgets, keyed by their key.
    pub custom: Vec<(&'static str, egui::Response)>,
}

/// A ribbon: the tab strip and, per [`RibbonState`], its body or peek popup.
///
/// ```
/// # use fluent2_egui::office::ribbon::{Ribbon, RibbonState, RibbonTab};
/// # fn demo(ui: &mut egui::Ui, state: &mut RibbonState) {
/// let response = Ribbon::new("main")
///     .tab(RibbonTab::new("home", "Home", |_tab| {}))
///     .tab(RibbonTab::new("insert", "Insert", |_tab| {}))
///     .show(ui, state);
/// # let _ = response;
/// # }
/// ```
pub struct Ribbon<'a> {
    id: Id,
    tabs: Vec<RibbonTab<'a>>,
}

impl<'a> Ribbon<'a> {
    /// A ribbon. `id` is the widget id and is never derived from a label.
    pub fn new(id: impl Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::new(id),
            tabs: Vec::new(),
        }
    }

    /// Append a content or contextual tab. Content tabs keep their order; contextual tabs
    /// follow them.
    #[must_use]
    pub fn tab(mut self, tab: RibbonTab<'a>) -> Self {
        self.tabs.push(tab);
        self
    }

    /// Show the strip and, per `state`, the body or the peek popup.
    pub fn show(self, ui: &mut Ui, state: &mut RibbonState) -> RibbonResponse {
        let id = self.id;
        let mut out = RibbonResponse::default();
        let strip = visible_tabs(self.tabs, &mut out.conflicts);

        // A contextual tab that is no longer visible hands the active tab to the first
        // content tab. The state changes here, so the app sees it in `RibbonState` (§4.3).
        let first_content = strip
            .iter()
            .find(|tab| tab.contextual.is_none())
            .map(|tab| tab.key);
        let before = state.active.or(first_content);
        let active = state
            .active
            .filter(|key| strip.iter().any(|tab| tab.key == *key))
            .or(first_content);
        if state.active.is_some() && state.active != active {
            state.active = active;
        }
        if before != active {
            out.tab_changed = active;
        }

        let keys: Vec<&'static str> = strip.iter().map(|tab| tab.key).collect();
        let labels: Vec<String> = strip.iter().map(|tab| tab.label.clone()).collect();
        let active_label = active
            .and_then(|key| keys.iter().position(|k| *k == key))
            .and_then(|index| labels.get(index).cloned());
        // Bodies are built once per frame, before painting. Groups arrive with WI-1563.
        for tab in strip {
            let RibbonTab { build, .. } = tab;
            build(&mut TabBuilder::new());
        }

        let mut selected = active
            .and_then(|key| keys.iter().position(|k| *k == key))
            .unwrap_or(0);
        let mut list = TabList::new(id, &mut selected);
        for label in &labels {
            list = list.tab(label.as_str());
        }
        let strip_response = list.show(ui);
        // The strip's own node carries the role and label, so it has a rect for assistive tech.
        ui.ctx().accesskit_node_builder(strip_response.id, |node| {
            node.set_role(accesskit::Role::TabList);
            node.set_label("Ribbon tabs");
        });

        // egui does not focus a strip on click, so the strip takes focus itself and the arrow
        // keys work after a click (roving focus, §9.2).
        // `selected` is the tab the strip shows after this frame's click, changed or not.
        if strip_response.clicked() {
            strip_response.request_focus();
            if state.display == RibbonDisplay::Collapsed {
                state.display = RibbonDisplay::Peek;
            }
        }
        if (strip_response.clicked() || strip_response.changed())
            && let Some(key) = keys.get(selected).copied()
        {
            if active != Some(key) {
                out.tab_changed = Some(key);
            }
            state.active = Some(key);
        }
        if strip_response.double_clicked() {
            toggle_expanded(state);
        }
        if ui.input_mut(|input| input.consume_key(Modifiers::CTRL, Key::F1)) {
            toggle_expanded(state);
        }

        let popup_key = id.with("peek_rect");
        let popup_before = ui.ctx().data(|data| data.get_temp::<Rect>(popup_key));
        let peeking = state.display == RibbonDisplay::Peek && !state.pinned;
        if peeking && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            state.display = RibbonDisplay::Collapsed;
        }
        if peeking {
            // A click outside both the strip and the popup closes the peek. The popup rect is
            // the one drawn last frame, so this check runs before this frame's popup.
            let outside = ui.input(|input| {
                input.pointer.any_click()
                    && input.pointer.latest_pos().is_some_and(|pos| {
                        !strip_response.rect.contains(pos)
                            && !popup_before.is_some_and(|rect| rect.contains(pos))
                    })
            });
            if outside {
                state.display = RibbonDisplay::Collapsed;
            }
        }

        // A pinned peek behaves as expanded (§4.5).
        let body_inline = state.display == RibbonDisplay::Expanded
            || (state.display == RibbonDisplay::Peek && state.pinned);
        if body_inline {
            body_node(ui.ctx(), id.with("body"), active_label.as_deref());
        }
        if state.display == RibbonDisplay::Peek && !state.pinned {
            let theme = Theme::get(ui.ctx());
            let popup = Area::new(id.with("peek"))
                .order(Order::Foreground)
                .fixed_pos(strip_response.rect.left_bottom())
                .show(ui.ctx(), |ui| {
                    popup_frame(&theme).show(ui, |ui| {
                        body_node(ui.ctx(), id.with("body"), active_label.as_deref());
                    });
                });
            ui.ctx()
                .data_mut(|data| data.insert_temp(popup_key, popup.response.rect));
        } else {
            ui.ctx().data_mut(|data| data.remove::<Rect>(popup_key));
        }

        out
    }
}

/// Keep the first tab of each key and every visible contextual tab, grouped by set.
///
/// A later tab with a key already used is dropped and its key is reported in `conflicts`
/// (D4). A contextual tab that is not visible this frame is dropped.
fn visible_tabs<'a>(tabs: Vec<RibbonTab<'a>>, conflicts: &mut Vec<&'static str>) -> Vec<RibbonTab<'a>> {
    let mut seen: Vec<&'static str> = Vec::new();
    let mut content = Vec::new();
    let mut contextual = Vec::new();
    for tab in tabs {
        if seen.contains(&tab.key) {
            conflicts.push(tab.key);
            continue;
        }
        seen.push(tab.key);
        match tab.contextual {
            None => content.push(tab),
            Some(placement) if placement.visible => contextual.push(tab),
            Some(_) => {}
        }
    }
    // Tabs of one set sit together, in the order their sets first appear.
    let mut sets: Vec<&'static str> = Vec::new();
    for tab in &contextual {
        if let Some(placement) = tab.contextual
            && !sets.contains(&placement.set)
        {
            sets.push(placement.set);
        }
    }
    contextual.sort_by_key(|tab| {
        tab.contextual
            .and_then(|placement| sets.iter().position(|set| *set == placement.set))
            .unwrap_or(usize::MAX)
    });
    content.extend(contextual);
    content
}

/// Double-click and Ctrl+F1: expanded collapses, anything else expands (§4.5).
fn toggle_expanded(state: &mut RibbonState) {
    let expanded =
        state.display == RibbonDisplay::Expanded || (state.display == RibbonDisplay::Peek && state.pinned);
    if expanded {
        state.display = RibbonDisplay::Collapsed;
        state.pinned = false;
    } else {
        state.display = RibbonDisplay::Expanded;
    }
}

/// The ribbon body. It is an empty placeholder until groups arrive (WI-1563).
fn body_node(ctx: &egui::Context, id: Id, label: Option<&str>) {
    ctx.accesskit_node_builder(id, |node| {
        node.set_role(accesskit::Role::TabPanel);
        if let Some(label) = label {
            node.set_label(label.to_owned());
        }
    });
}

fn popup_frame(theme: &Theme) -> Frame {
    Frame::new()
        .fill(c(theme.color.neutral_background_1))
        .stroke(Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)))
        .corner_radius(popup_radius())
}

fn popup_radius() -> egui::CornerRadius {
    egui::CornerRadius::same(border_radius::LARGE.round().clamp(0.0, 255.0) as u8)
}
