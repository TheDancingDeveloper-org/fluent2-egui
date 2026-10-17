//! NavigationView: a navigation pane beside the content (`docs/specs/data-views.md` §7).
//!
//! The pane is one Tab stop. Up and Down move among the items, Enter or Space selects one, and
//! the selected item carries TabList's vertical indicator. Below 1008 pt the pane collapses to
//! an icon rail, and below 641 pt it is replaced by a hamburger over the content; in both the
//! full pane opens as an overlay.

use std::collections::HashSet;

use egui::{
    Area, CornerRadius, Id, Key, Layout, Modifiers, Order, Pos2, Rect, Sense, TextStyle, TextWrapMode, Ui,
    UiBuilder, Vec2, WidgetInfo, WidgetText, WidgetType, accesskit,
};
use fluent2_icons::{Icon, Size as IconSize, Style as IconStyle};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical, stroke_width};

use crate::controls::{Appearance, Button, button};
use crate::foundation::elevation::{Elevation, paint_shadow};
use crate::foundation::focus;
use crate::foundation::icon::paint_icon;
use crate::foundation::motion::{self, Curve, Duration};
use crate::foundation::text::TypeRamp;
use crate::surfaces::{TooltipExt, paint_indicator};
use crate::theme::{DataMetrics, Theme, c};

/// Height of a section header row in the full pane, in points (8 pt margins around `Caption1`).
const SECTION_HEIGHT: f32 = 32.0;
/// Height of a divider row, in points (8 pt margins around a 1 pt line).
const DIVIDER_HEIGHT: f32 = 17.0;
/// Icon size of an item, in points (§10).
const ICON_SIZE: f32 = 20.0;
/// Width of the hamburger button, in points.
const HAMBURGER_WIDTH: f32 = 32.0;

/// How the navigation pane is shown (§7.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum PaneMode {
    /// Chosen from the width of the `Ui` the view is shown in, by the breakpoints in
    /// [`DataMetrics`].
    #[default]
    Auto,
    /// Full pane beside the content.
    Expanded,
    /// Icon rail beside the content. The full pane opens as an overlay.
    Compact,
    /// No pane. A hamburger above the content opens the full pane as an overlay.
    Minimal,
}

/// State of a NavigationView. The caller owns it.
///
/// `pane_open` is the hamburger's choice in [`PaneMode::Expanded`]: `true` shows the full pane,
/// `false` collapses it to the icon rail. In Compact and Minimal the overlay's open state is kept
/// in egui memory instead, and it closes on Escape, a click outside, or a selection.
///
/// ```
/// let state = fluent2_egui::data::nav_view::NavState::default();
/// assert!(state.pane_open);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct NavState {
    /// Key of the selected item.
    pub selected: Option<String>,
    /// The hamburger's choice in Expanded mode. See the type docs.
    pub pane_open: bool,
    /// `Auto` follows the breakpoints.
    pub mode: PaneMode,
}

impl Default for NavState {
    /// `pane_open` starts `true`, so a wide window opens with the full pane.
    fn default() -> Self {
        Self {
            selected: None,
            pane_open: true,
            mode: PaneMode::Auto,
        }
    }
}

/// What a NavigationView reports after `show`.
pub struct NavResponse<R> {
    /// Whatever the content closure returned.
    pub inner: R,
    /// The item the user selected this frame (also written to `state.selected`).
    pub selected: Option<&'static str>,
    /// The hamburger was used this frame.
    pub pane_toggled: bool,
}

enum Entry {
    Item(NavItem),
    Section(String),
    Divider,
}

struct NavItem {
    key: &'static str,
    icon: Icon,
    label: String,
    footer: bool,
}

/// Where a row sits in the pane, in display order.
#[derive(Clone, Copy)]
enum Slot {
    Item { entry: usize, nav: usize },
    Section(usize),
    Divider,
}

/// The pane's rows, split into the top group and the footer group, and the item order that
/// keyboard focus follows.
struct Plan {
    top: Vec<Slot>,
    footer: Vec<Slot>,
    /// Entry index of each item, in navigation order. The first item with a key wins (D10).
    nav: Vec<usize>,
}

/// What a pane reports after one frame.
struct PaneOut {
    selected: Option<&'static str>,
    focus: usize,
    hamburger_clicked: bool,
    hamburger_row: Rect,
}

/// A navigation pane and content area. Add items, then call [`NavigationView::show`].
///
/// ```
/// # use fluent2_egui::data::nav_view::{NavState, NavigationView};
/// # use fluent2_icons::Icon;
/// # fn demo(ui: &mut egui::Ui) {
/// let mut state = NavState::default();
/// let response = NavigationView::new("main-nav")
///     .title("Library")
///     .item("home", Icon::Home, "Home")
///     .section("Library")
///     .item("files", Icon::Document, "Files")
///     .footer_item("settings", Icon::Settings, "Settings")
///     .show(ui, &mut state, |ui| ui.label("Content"));
/// if let Some(key) = response.selected {
///     println!("selected {key}");
/// }
/// # }
/// ```
pub struct NavigationView {
    id: Id,
    entries: Vec<Entry>,
    title: Option<String>,
    open_label: String,
    close_label: String,
}

impl NavigationView {
    /// A navigation view. `id_salt` must be unique among the views in the same `Ui`.
    #[must_use]
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug) -> Self {
        Self {
            id: Id::new(id_salt),
            entries: Vec::new(),
            title: None,
            open_label: "Open navigation".to_owned(),
            close_label: "Close navigation".to_owned(),
        }
    }

    /// Add an item. `key` identifies it in state and in the response.
    #[must_use]
    pub fn item(mut self, key: &'static str, icon: Icon, label: impl Into<String>) -> Self {
        self.entries.push(Entry::Item(NavItem {
            key,
            icon,
            label: label.into(),
            footer: false,
        }));
        self
    }

    /// A section header above the items that follow. In the icon rail it becomes a divider.
    #[must_use]
    pub fn section(mut self, label: impl Into<String>) -> Self {
        self.entries.push(Entry::Section(label.into()));
        self
    }

    /// A divider line.
    #[must_use]
    pub fn divider(mut self) -> Self {
        self.entries.push(Entry::Divider);
        self
    }

    /// An item pinned to the bottom of the pane, such as Settings.
    #[must_use]
    pub fn footer_item(mut self, key: &'static str, icon: Icon, label: impl Into<String>) -> Self {
        self.entries.push(Entry::Item(NavItem {
            key,
            icon,
            label: label.into(),
            footer: true,
        }));
        self
    }

    /// Header text beside the hamburger in the full pane.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Accessible names of the hamburger: `(open, close)`. The defaults are "Open navigation"
    /// and "Close navigation".
    #[must_use]
    pub fn hamburger_labels(mut self, open: impl Into<String>, close: impl Into<String>) -> Self {
        self.open_label = open.into();
        self.close_label = close.into();
        self
    }

    /// Lay out the pane and the content. `content` runs once, in the area to the right of the
    /// pane (below the hamburger in Minimal mode).
    pub fn show<R>(
        self,
        ui: &mut Ui,
        state: &mut NavState,
        content: impl FnOnce(&mut Ui) -> R,
    ) -> NavResponse<R> {
        let theme = Theme::get(ui.ctx());
        let metrics = theme.data;
        button::ensure_type_ramp(ui);
        let full = ui.available_rect_before_wrap();
        if !full.is_finite() {
            let inner = content(ui);
            return NavResponse {
                inner,
                selected: None,
                pane_toggled: false,
            };
        }
        let id = ui.make_persistent_id(self.id);
        let mode = resolve_mode(state.mode, ui.available_width(), &metrics);
        let plan = self.plan();
        let overlay_key = id.with("overlay");
        let focus_key = id.with("focus");
        let overlay_open = ui.data(|data| data.get_temp::<bool>(overlay_key).unwrap_or(false));
        let selected_nav = state.selected.as_deref().and_then(|key| {
            plan.nav
                .iter()
                .position(|&entry| matches!(&self.entries[entry], Entry::Item(item) if item.key == key))
        });
        let focus = ui
            .data(|data| data.get_temp::<usize>(focus_key))
            .unwrap_or(selected_nav.unwrap_or(0))
            .min(plan.nav.len().saturating_sub(1));
        let item_h = metrics.nav_item_height;
        let expanded_w = metrics.nav_pane_width_expanded;
        let compact_w = metrics.nav_pane_width_compact;

        // Where the pane, the hamburger row and the content go in this frame.
        let (pane_rect, content_rect, minimal_row) = match mode {
            PaneMode::Minimal => {
                let row = Rect::from_min_size(full.min, Vec2::new(full.width(), item_h));
                let content = Rect::from_min_max(Pos2::new(full.min.x, full.min.y + item_h), full.max);
                (None, content, Some(row))
            }
            _ => {
                let pane_w = if mode == PaneMode::Expanded && state.pane_open {
                    expanded_w
                } else {
                    compact_w
                };
                let pane = Rect::from_min_size(full.min, Vec2::new(pane_w, full.height()));
                let content = Rect::from_min_max(Pos2::new(full.min.x + pane_w, full.min.y), full.max);
                (Some(pane), content, None)
            }
        };

        let mut selected = None;
        let mut focus_out = focus;
        let mut pane_toggled = false;
        let mut overlay_next = overlay_open;
        let mut anchor = full;

        if let Some(pane) = pane_rect {
            let rail = !(mode == PaneMode::Expanded && state.pane_open);
            let hamburger_open = if mode == PaneMode::Expanded {
                state.pane_open
            } else {
                overlay_open
            };
            let look = PaneLook {
                rail,
                hamburger_open,
                scope: "pane",
                interactive: true,
                selected: state.selected.as_deref(),
                focus,
            };
            let out = self.draw_pane(ui, pane, &theme, &plan, look);
            anchor = out.hamburger_row;
            if out.hamburger_clicked {
                pane_toggled = true;
                if mode == PaneMode::Expanded {
                    state.pane_open = !state.pane_open;
                } else {
                    overlay_next = !overlay_open;
                }
            }
            if out.selected.is_some() {
                selected = out.selected;
            }
            focus_out = out.focus;
        }

        if let Some(row) = minimal_row {
            anchor = row;
            if self.hamburger(ui, row, "minimal", overlay_open) {
                pane_toggled = true;
                overlay_next = !overlay_open;
            }
        }

        let inner = ui
            .scope_builder(UiBuilder::new().max_rect(content_rect), content)
            .inner;

        // The overlay slides in and out (§7.2). Motion snaps under reduced motion.
        let overlay_w = expanded_w;
        let slide = if mode == PaneMode::Expanded {
            0.0
        } else {
            let target = if overlay_open { 0.0 } else { -overlay_w };
            motion::animate(
                ui.ctx(),
                id.with("overlay_slide"),
                target,
                Duration::Normal,
                Curve::DecelerateMid,
            )
        };
        let overlay_visible = mode != PaneMode::Expanded && (overlay_open || slide > -overlay_w + 0.5);
        if overlay_visible {
            let overlay_rect = Rect::from_min_size(
                Pos2::new(full.min.x + slide, full.min.y),
                Vec2::new(overlay_w, full.height()),
            );
            let look = PaneLook {
                rail: false,
                hamburger_open: true,
                scope: "overlay",
                interactive: overlay_open,
                selected: state.selected.as_deref(),
                focus: focus_out,
            };
            let out = Area::new(id.with("overlay"))
                .order(Order::Foreground)
                .fixed_pos(overlay_rect.min)
                .show(ui.ctx(), |ui| {
                    paint_shadow(
                        ui.painter(),
                        overlay_rect,
                        CornerRadius::ZERO,
                        Elevation::E16,
                        &theme,
                    );
                    self.draw_pane(ui, overlay_rect, &theme, &plan, look)
                })
                .inner;
            if overlay_open {
                if out.hamburger_clicked {
                    pane_toggled = true;
                    overlay_next = false;
                }
                if out.selected.is_some() {
                    selected = out.selected;
                    overlay_next = false;
                }
                focus_out = out.focus;
                let (escape, outside) = ui.input(|input| {
                    let pointer = input.pointer.interact_pos();
                    let outside = input.pointer.any_click()
                        && pointer.is_some_and(|p| !overlay_rect.contains(p) && !anchor.contains(p));
                    (input.key_pressed(Key::Escape), outside)
                });
                if escape || outside {
                    overlay_next = false;
                }
            }
        }

        if overlay_next != overlay_open {
            ui.data_mut(|data| data.insert_temp(overlay_key, overlay_next));
        }
        if focus_out != focus {
            ui.data_mut(|data| data.insert_temp(focus_key, focus_out));
        }
        if let Some(key) = selected {
            state.selected = Some(key.to_owned());
            if mode != PaneMode::Expanded {
                ui.data_mut(|data| data.insert_temp(overlay_key, false));
            }
        }
        ui.allocate_rect(full, egui::Sense::hover());
        NavResponse {
            inner,
            selected,
            pane_toggled,
        }
    }

    /// Split the entries into the top and footer groups and number the items. Duplicate keys
    /// keep their first occurrence: main items come before footer items.
    fn plan(&self) -> Plan {
        let mut seen = HashSet::new();
        let mut nav = Vec::new();
        let mut nav_pos = vec![None; self.entries.len()];
        for footer in [false, true] {
            for (index, entry) in self.entries.iter().enumerate() {
                if let Entry::Item(item) = entry
                    && item.footer == footer
                    && seen.insert(item.key)
                {
                    nav_pos[index] = Some(nav.len());
                    nav.push(index);
                }
            }
        }
        let mut top = Vec::new();
        let mut footer = Vec::new();
        for (index, entry) in self.entries.iter().enumerate() {
            match entry {
                Entry::Item(item) => {
                    if let Some(nav_index) = nav_pos[index] {
                        let slot = Slot::Item {
                            entry: index,
                            nav: nav_index,
                        };
                        if item.footer {
                            footer.push(slot);
                        } else {
                            top.push(slot);
                        }
                    }
                }
                Entry::Section(_) => top.push(Slot::Section(index)),
                Entry::Divider => top.push(Slot::Divider),
            }
        }
        Plan { top, footer, nav }
    }

    /// Draw one pane: background, hamburger row, items with the selected indicator, and the
    /// footer. Keyboard and click input are read here.
    fn draw_pane(&self, ui: &mut Ui, rect: Rect, theme: &Theme, plan: &Plan, look: PaneLook<'_>) -> PaneOut {
        let selected_key = look.selected;
        let focus = look.focus;
        let id = ui.make_persistent_id(self.id).with(look.scope);
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, c(theme.color.neutral_background_4));

        let item_h = theme.data.nav_item_height;
        let hamburger_row = Rect::from_min_size(rect.min, Vec2::new(rect.width(), item_h));
        let hamburger_clicked = self.hamburger(ui, hamburger_row, look.scope, look.hamburger_open);
        if !look.rail
            && let Some(title) = &self.title
        {
            let x = rect.min.x + spacing_horizontal::S + HAMBURGER_WIDTH + spacing_horizontal::S;
            let galley = WidgetText::from(title.as_str()).into_galley(
                ui,
                Some(TextWrapMode::Truncate),
                (rect.max.x - x).max(0.0),
                TextStyle::Name(TypeRamp::Subtitle2.name().into()),
            );
            let pos = Pos2::new(x, hamburger_row.center().y - galley.size().y / 2.0);
            ui.painter()
                .galley(pos, galley, c(theme.color.neutral_foreground_1));
        }

        // Lay the rows out: the top group from below the hamburger, the footer from the bottom.
        let footer_h: f32 = plan
            .footer
            .iter()
            .map(|&slot| slot_height(slot, item_h, look.rail))
            .sum();
        let mut rows = Vec::with_capacity(plan.top.len() + plan.footer.len());
        let mut y = hamburger_row.max.y;
        for &slot in &plan.top {
            let h = slot_height(slot, item_h, look.rail);
            rows.push((
                slot,
                Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(rect.width(), h)),
            ));
            y += h;
        }
        let mut y = rect.max.y - footer_h - spacing_vertical::S;
        for &slot in &plan.footer {
            let h = slot_height(slot, item_h, look.rail);
            rows.push((
                slot,
                Rect::from_min_size(Pos2::new(rect.min.x, y), Vec2::new(rect.width(), h)),
            ));
            y += h;
        }

        let sense = if look.interactive {
            Sense::click()
        } else {
            Sense::hover()
        };
        let pane = ui.interact(rect, id, sense);
        let pointer = pane.interact_pointer_pos();
        let mut nav_rects = vec![Rect::NOTHING; plan.nav.len()];
        for &(slot, row) in &rows {
            if let Slot::Item { nav, .. } = slot {
                nav_rects[nav] = item_rect(row);
            }
        }

        // Keyboard: the pane is one Tab stop. Up and Down move, Home and End jump, Enter and
        // Space select. A click moves focus to the item and selects it.
        let mut focus = focus;
        let mut activate = None;
        if pane.has_focus() && !plan.nav.is_empty() {
            let last = plan.nav.len() - 1;
            ui.input_mut(|input| {
                if input.consume_key(Modifiers::NONE, Key::ArrowDown) {
                    focus = (focus + 1).min(last);
                }
                if input.consume_key(Modifiers::NONE, Key::ArrowUp) {
                    focus = focus.saturating_sub(1);
                }
                if input.consume_key(Modifiers::NONE, Key::Home) {
                    focus = 0;
                }
                if input.consume_key(Modifiers::NONE, Key::End) {
                    focus = last;
                }
                if input.consume_key(Modifiers::NONE, Key::Enter)
                    || input.consume_key(Modifiers::NONE, Key::Space)
                {
                    activate = Some(focus);
                }
            });
        }
        if pane.clicked()
            && let Some(hit) = pointer.and_then(|p| nav_rects.iter().position(|r| r.contains(p)))
        {
            focus = hit;
            activate = Some(hit);
        }
        let mut selected = None;
        if let Some(nav) = activate
            && let Some(&entry) = plan.nav.get(nav)
            && let Entry::Item(item) = &self.entries[entry]
        {
            selected = Some(item.key);
        }

        let selected_nav = selected_key.and_then(|key| {
            plan.nav
                .iter()
                .position(|&entry| matches!(&self.entries[entry], Entry::Item(item) if item.key == key))
        });
        let list_rect = Rect::from_min_max(Pos2::new(rect.min.x, hamburger_row.max.y), rect.max);
        let list_id = id.with("list");
        let item_count = plan.nav.len();
        ui.scope_builder(UiBuilder::new().id(list_id).max_rect(list_rect), |ui| {
            ui.interact(list_rect, list_id, Sense::hover())
                .widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "Navigation items"));
            ui.ctx().accesskit_node_builder(list_id, |node| {
                node.set_role(accesskit::Role::TabList);
                node.set_orientation(accesskit::Orientation::Vertical);
            });
            for &(slot, row) in &rows {
                match slot {
                    Slot::Item { entry, nav } => {
                        let Entry::Item(item) = &self.entries[entry] else {
                            continue;
                        };
                        let rect = nav_rects[nav];
                        let hovered = ui.rect_contains_pointer(rect);
                        let pressed = hovered && pane.is_pointer_button_down_on();
                        let is_selected = selected_key == Some(item.key);
                        let fill = if pressed {
                            Some(c(theme.color.neutral_background_4_pressed))
                        } else if hovered {
                            Some(c(theme.color.neutral_background_4_hover))
                        } else {
                            None
                        };
                        if let Some(fill) = fill {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(border_radius::MEDIUM as u8),
                                fill,
                            );
                        }
                        let text_color = if is_selected {
                            c(theme.color.neutral_foreground_2_brand_selected)
                        } else {
                            c(theme.color.neutral_foreground_2)
                        };
                        let icon_rect = if look.rail {
                            Rect::from_center_size(rect.center(), Vec2::splat(ICON_SIZE))
                        } else {
                            Rect::from_min_size(
                                Pos2::new(
                                    rect.min.x + spacing_horizontal::M_NUDGE,
                                    rect.center().y - ICON_SIZE / 2.0,
                                ),
                                Vec2::splat(ICON_SIZE),
                            )
                        };
                        paint_icon(
                            ui,
                            icon_rect,
                            item.icon,
                            IconSize::S20,
                            IconStyle::Regular,
                            Some(text_color),
                        );
                        if !look.rail {
                            let ramp = if is_selected {
                                TypeRamp::Body1Strong
                            } else {
                                TypeRamp::Body1
                            };
                            let x = icon_rect.max.x + spacing_horizontal::M;
                            let galley = WidgetText::from(item.label.as_str()).into_galley(
                                ui,
                                Some(TextWrapMode::Truncate),
                                (rect.max.x - x - spacing_horizontal::S).max(0.0),
                                TextStyle::Name(ramp.name().into()),
                            );
                            let pos = Pos2::new(x, rect.center().y - galley.size().y / 2.0);
                            ui.painter().galley(pos, galley, text_color);
                        } else if hovered {
                            ui.interact(rect, id.with(("tip", nav)), Sense::hover())
                                .fluent_tooltip(item.label.as_str());
                        }
                        if selected_nav == Some(nav) {
                            let y = motion::animate(
                                ui.ctx(),
                                id.with("indicator_y"),
                                rect.min.y,
                                Duration::Normal,
                                Curve::DecelerateMax,
                            );
                            let h = motion::animate(
                                ui.ctx(),
                                id.with("indicator_h"),
                                rect.height(),
                                Duration::Normal,
                                Curve::DecelerateMax,
                            );
                            let bar = Rect::from_min_size(
                                Pos2::new(rect.min.x, y),
                                Vec2::new(stroke_width::THICKER, h),
                            );
                            paint_indicator(ui, bar, theme);
                        }
                        // A hover-only widget gives the item its accessibility bounds; the pane
                        // still takes the click.
                        let item_id = id.with(("item", nav));
                        let label = item.label.clone();
                        ui.interact(rect, item_id, Sense::hover()).widget_info(|| {
                            WidgetInfo::labeled(WidgetType::SelectableLabel, true, label.as_str())
                        });
                        ui.ctx().accesskit_node_builder(item_id, |node| {
                            node.set_role(accesskit::Role::Tab);
                            node.set_label(label);
                            node.set_selected(is_selected);
                            node.set_position_in_set(nav + 1);
                            node.set_size_of_set(item_count);
                        });
                    }
                    Slot::Section(entry) => {
                        if look.rail {
                            paint_divider(ui, row, theme);
                            continue;
                        }
                        let Entry::Section(label) = &self.entries[entry] else {
                            continue;
                        };
                        let galley = WidgetText::from(label.as_str()).into_galley(
                            ui,
                            Some(TextWrapMode::Truncate),
                            (row.width() - 2.0 * spacing_horizontal::M_NUDGE).max(0.0),
                            TextStyle::Name(TypeRamp::Caption1.name().into()),
                        );
                        let pos = Pos2::new(
                            row.min.x + spacing_horizontal::M_NUDGE,
                            row.min.y + spacing_vertical::S,
                        );
                        ui.painter()
                            .galley(pos, galley, c(theme.color.neutral_foreground_3));
                        let section_id = id.with(("section", entry));
                        ui.interact(row, section_id, Sense::hover())
                            .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, label.as_str()));
                        ui.ctx().accesskit_node_builder(section_id, |node| {
                            node.set_role(accesskit::Role::Heading);
                            node.set_level(2);
                            node.set_label(label.as_str());
                        });
                    }
                    Slot::Divider => paint_divider(ui, row, theme),
                }
            }
        });

        if focus::focus_visible(ui, &pane)
            && let Some(&rect) = nav_rects.get(focus)
        {
            focus::paint_focus_ring(ui, rect, CornerRadius::same(border_radius::MEDIUM as u8));
        }
        let title = self.title.as_deref().unwrap_or("Navigation");
        pane.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, title));
        ui.ctx()
            .accesskit_node_builder(pane.id, |node| node.set_role(accesskit::Role::Navigation));

        PaneOut {
            selected,
            focus,
            hamburger_clicked,
            hamburger_row,
        }
    }

    /// The hamburger button in `row`. Returns `true` when it was clicked this frame.
    fn hamburger(&self, ui: &mut Ui, row: Rect, scope: &'static str, open: bool) -> bool {
        let label = if open {
            self.close_label.as_str()
        } else {
            self.open_label.as_str()
        };
        let mut child = ui.new_child(
            UiBuilder::new()
                .max_rect(row)
                .layout(Layout::left_to_right(egui::Align::Center)),
        );
        child.add_space(spacing_horizontal::S);
        let response = child.add(
            Button::icon_only(Icon::Navigation, label)
                .appearance(Appearance::Subtle)
                .id_salt((scope, "hamburger")),
        );
        let id = response.id;
        ui.ctx()
            .accesskit_node_builder(id, |node| node.set_expanded(open));
        response.clicked()
    }
}

/// A 1 pt divider line across the middle of `row`.
fn paint_divider(ui: &Ui, row: Rect, theme: &Theme) {
    let y = row.center().y;
    let line = Rect::from_min_max(
        Pos2::new(row.min.x + spacing_horizontal::S, y),
        Pos2::new(row.max.x - spacing_horizontal::S, y + stroke_width::THIN),
    );
    ui.painter()
        .rect_filled(line, CornerRadius::ZERO, c(theme.color.neutral_stroke_2));
}

/// How one pane is drawn this frame.
#[derive(Clone, Copy)]
struct PaneLook<'a> {
    /// Icons only, with labels as tooltips.
    rail: bool,
    /// The hamburger's current state, for its label and `expanded`.
    hamburger_open: bool,
    /// Keeps ids of the pane and the overlay apart.
    scope: &'static str,
    /// `false` while the overlay slides out: it is drawn but takes no input.
    interactive: bool,
    /// Key of the selected item.
    selected: Option<&'a str>,
    /// Navigation position that keyboard focus is on.
    focus: usize,
}

/// The mode the breakpoints (§7.2) give for a window `width` points wide.
fn resolve_mode(mode: PaneMode, width: f32, metrics: &DataMetrics) -> PaneMode {
    match mode {
        PaneMode::Auto => {
            if width.is_nan() || width < metrics.nav_breakpoint_compact {
                PaneMode::Minimal
            } else if width < metrics.nav_breakpoint_expanded {
                PaneMode::Compact
            } else {
                PaneMode::Expanded
            }
        }
        fixed => fixed,
    }
}

fn slot_height(slot: Slot, item_h: f32, rail: bool) -> f32 {
    match slot {
        Slot::Item { .. } => item_h,
        Slot::Section(_) if !rail => SECTION_HEIGHT,
        Slot::Section(_) | Slot::Divider => DIVIDER_HEIGHT,
    }
}

/// The clickable part of an item row: inset from the pane edges.
fn item_rect(row: Rect) -> Rect {
    Rect::from_min_max(
        Pos2::new(row.min.x + spacing_horizontal::XS, row.min.y),
        Pos2::new(row.max.x - spacing_horizontal::XS, row.max.y),
    )
}
