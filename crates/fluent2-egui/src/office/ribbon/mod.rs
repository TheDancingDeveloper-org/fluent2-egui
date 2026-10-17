//! Ribbon tab strip (spec §4): the File tab, content and contextual tabs, collapse, and the
//! peek popup.
//!
//! The strip is a [`TabList`], so its layout, indicator, arrow keys and Home/End come from
//! the list. Tab bodies are empty placeholders until groups (WI-1563) and galleries (WI-1564)
//! arrive. The pin button is WI-1563's (spec §4.5).

mod tab;

use std::hash::Hash;
use std::sync::Arc;

use egui::{
    Area, CornerRadius, Frame, Galley, Id, Key, Modifiers, Order, Pos2, Rect, Response, Sense, Stroke,
    TextStyle, Ui, Vec2, WidgetInfo, WidgetText, WidgetType, accesskit,
};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use crate::foundation::focus::{self, paint_focus_ring};
use crate::foundation::text::TypeRamp;
use crate::surfaces::{TabList, TabSize, tab_widths};
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

/// What a control in the ribbon did this frame (spec §4.6).
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
    /// A button, split, menu, or dialog launcher was pressed. The File tab reports this too.
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
    /// Keys used more than once in this ribbon. The first occurrence keeps its control (D4).
    pub conflicts: Vec<&'static str>,
    /// Responses of custom widgets, keyed by their key.
    pub custom: Vec<(&'static str, Response)>,
}

/// The File tab. It is styled as the accent tab and is never a content tab (spec §4.1). A
/// click reports [`RibbonEventKind::Clicked`] on `key`; the app opens the backstage (§7).
///
/// ```
/// # use fluent2_egui::office::ribbon::FileTab;
/// let file = FileTab::new("file", "File");
/// ```
pub struct FileTab {
    key: &'static str,
    label: String,
}

impl FileTab {
    /// The File tab. `key` names it in events and must be unique within the ribbon.
    pub fn new(key: &'static str, label: &str) -> Self {
        Self {
            key,
            label: label.to_owned(),
        }
    }
}

/// A ribbon: the File tab, the tab strip and, per [`RibbonState`], its body or peek popup.
///
/// ```
/// # use fluent2_egui::office::ribbon::{FileTab, Ribbon, RibbonState, RibbonTab};
/// # fn demo(ui: &mut egui::Ui, state: &mut RibbonState) {
/// let response = Ribbon::new("main")
///     .file_tab(FileTab::new("file", "File"))
///     .tab(RibbonTab::new("home", "Home", |_tab| {}))
///     .tab(RibbonTab::new("insert", "Insert", |_tab| {}))
///     .show(ui, state);
/// # let _ = response;
/// # }
/// ```
pub struct Ribbon<'a> {
    id: Id,
    file_tab: Option<FileTab>,
    tabs: Vec<RibbonTab<'a>>,
}

impl<'a> Ribbon<'a> {
    /// A ribbon. `id` is the widget id and is never derived from a label.
    pub fn new(id: impl Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::new(id),
            file_tab: None,
            tabs: Vec::new(),
        }
    }

    /// The File tab, placed before the strip.
    #[must_use]
    pub fn file_tab(mut self, file_tab: FileTab) -> Self {
        self.file_tab = Some(file_tab);
        self
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
        let file_tab = self.file_tab;
        let mut out = RibbonResponse::default();
        let reserved = file_tab.as_ref().map(|file| file.key);
        let strip = visible_tabs(self.tabs, reserved, &mut out.conflicts);

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
        let label_refs: Vec<&str> = labels.iter().map(String::as_str).collect();
        let active_label = active
            .and_then(|key| keys.iter().position(|k| *k == key))
            .and_then(|index| labels.get(index).cloned());
        let mut selected = active
            .and_then(|key| keys.iter().position(|k| *k == key))
            .unwrap_or(0);

        // One header per contextual set, over the set's first tab.
        let headers = contextual_headers(&strip);
        let widths = tab_widths(ui, &label_refs, selected, TabSize::Medium);
        let theme = Theme::get(ui.ctx());
        let header_galleys: Vec<Arc<Galley>> = headers
            .iter()
            .map(|header| {
                let caption = TextStyle::Name(TypeRamp::Caption1.name().into());
                WidgetText::from(header.label.as_str()).into_galley(
                    ui,
                    Some(egui::TextWrapMode::Extend),
                    f32::INFINITY,
                    caption,
                )
            })
            .collect();
        let header_height = header_galleys
            .iter()
            .map(|galley| galley.size().y)
            .fold(0.0_f32, f32::max);
        let file_width = match &file_tab {
            Some(file) => tab_widths(ui, &[file.label.as_str()], 0, TabSize::Medium)
                .first()
                .copied()
                .unwrap_or(0.0),
            None => 0.0,
        };

        // Bodies are built once per frame, before painting. Groups arrive with WI-1563.
        for tab in strip {
            let RibbonTab { build, .. } = tab;
            build(&mut TabBuilder::new());
        }

        let mut list_response = None;
        let mut file_response = None;
        ui.vertical(|ui| {
            if !headers.is_empty() {
                let (row, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), header_height), Sense::hover());
                let offsets = header_offsets(&headers, &widths);
                for ((header, galley), offset) in headers.iter().zip(&header_galleys).zip(&offsets) {
                    let x = row.left() + file_width + offset + spacing_horizontal::S;
                    ui.painter().galley(
                        Pos2::new(x, row.top()),
                        Arc::clone(galley),
                        header.accent.color(&theme),
                    );
                }
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if let Some(file) = &file_tab {
                    file_response = Some(file_tab_button(ui, file, file_width, &theme));
                }
                let mut list = TabList::new(id, &mut selected);
                for label in &label_refs {
                    list = list.tab(*label);
                }
                list_response = Some(list.show(ui));
            });
        });
        let Some(strip_response) = list_response else {
            return out;
        };

        // The strip's own node carries the role and label, so it has a rect for assistive tech.
        ui.ctx().accesskit_node_builder(strip_response.id, |node| {
            node.set_role(accesskit::Role::TabList);
            node.set_label("Ribbon tabs");
        });
        for header in &headers {
            ui.ctx()
                .accesskit_node_builder(id.with(("header", header.set)), |node| {
                    // A `Label` role keeps its text in `value`, not `label`.
                    node.set_role(accesskit::Role::Label);
                    node.set_value(header.label.clone());
                });
        }

        if let Some(file) = file_response {
            let key_pressed = file.has_focus()
                && ui.input_mut(|input| {
                    input.consume_key(Modifiers::NONE, Key::Enter)
                        | input.consume_key(Modifiers::NONE, Key::Space)
                });
            if file.clicked() {
                file.request_focus();
            }
            if let Some(file_tab) = &file_tab
                && (file.clicked() || key_pressed)
            {
                out.events.push(RibbonEvent {
                    key: file_tab.key,
                    kind: RibbonEventKind::Clicked,
                });
            }
        }

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

/// The header of one contextual set: its text, accent, and the index of its first tab.
struct Header {
    set: &'static str,
    label: String,
    accent: ContextualAccent,
    first: usize,
}

/// One header per visible contextual set, in strip order.
fn contextual_headers(strip: &[RibbonTab<'_>]) -> Vec<Header> {
    let mut headers: Vec<Header> = Vec::new();
    for (index, tab) in strip.iter().enumerate() {
        let Some(placement) = &tab.contextual else {
            continue;
        };
        if headers.iter().any(|header| header.set == placement.set) {
            continue;
        }
        headers.push(Header {
            set: placement.set,
            label: placement.label.clone(),
            accent: placement.accent,
            first: index,
        });
    }
    headers
}

/// Distance from the strip's left edge to each header's first tab.
fn header_offsets(headers: &[Header], widths: &[f32]) -> Vec<f32> {
    headers
        .iter()
        .map(|header| widths.iter().take(header.first).sum())
        .collect()
}

/// Keep the first tab of each key and every visible contextual tab, grouped by set.
///
/// A later tab with a key already used, or with the File tab's key, is dropped and its key is
/// reported in `conflicts` (D4). A contextual tab that is not visible this frame is dropped.
fn visible_tabs<'a>(
    tabs: Vec<RibbonTab<'a>>,
    reserved: Option<&'static str>,
    conflicts: &mut Vec<&'static str>,
) -> Vec<RibbonTab<'a>> {
    let mut seen: Vec<&'static str> = reserved.into_iter().collect();
    let mut content = Vec::new();
    let mut contextual = Vec::new();
    for tab in tabs {
        if seen.contains(&tab.key) {
            conflicts.push(tab.key);
            continue;
        }
        seen.push(tab.key);
        match &tab.contextual {
            None => content.push(tab),
            Some(placement) if placement.visible => contextual.push(tab),
            Some(_) => {}
        }
    }
    // Tabs of one set sit together, in the order their sets first appear.
    let mut sets: Vec<&'static str> = Vec::new();
    for tab in &contextual {
        if let Some(placement) = &tab.contextual
            && !sets.contains(&placement.set)
        {
            sets.push(placement.set);
        }
    }
    contextual.sort_by_key(|tab| {
        tab.contextual
            .as_ref()
            .and_then(|placement| sets.iter().position(|set| *set == placement.set))
            .unwrap_or(usize::MAX)
    });
    content.extend(contextual);
    content
}

/// The File tab: the accent tab, filled with the brand colour. It reports a click (§4.1).
fn file_tab_button(ui: &mut Ui, file: &FileTab, width: f32, theme: &Theme) -> Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width, theme.chrome.tab_strip_height), Sense::click());
    let color = &theme.color;
    let fill = if response.is_pointer_button_down_on() {
        color.brand_background_pressed
    } else if response.hovered() {
        color.brand_background_hover
    } else {
        color.brand_background
    };
    let corner = radius(border_radius::MEDIUM);
    ui.painter().rect_filled(rect, corner, c(fill));
    let text = WidgetText::from(file.label.as_str()).into_galley(
        ui,
        Some(egui::TextWrapMode::Extend),
        f32::INFINITY,
        TextStyle::Name(TypeRamp::Body1Strong.name().into()),
    );
    let origin = rect.center() - text.size() / 2.0;
    ui.painter()
        .galley(origin, Arc::clone(&text), c(color.neutral_foreground_on_brand));
    if focus::focus_visible(ui, &response) {
        paint_focus_ring(ui, rect, corner);
    }
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, file.label.as_str()));
    response
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
        .corner_radius(radius(border_radius::LARGE))
}

/// A corner radius from a Fluent border-radius token.
fn radius(value: f32) -> CornerRadius {
    CornerRadius::same(value.round().clamp(0.0, 255.0) as u8)
}
