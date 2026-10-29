//! Grouped virtual list: measured rows under sticky, expandable group headers.
//!
//! Only entries that intersect the viewport are laid out. While a group's rows occupy the
//! top of the viewport, its header is painted stuck there. Headers are AccessKit groups with
//! an expanded state; rows are list-box options (or list items when selection is off).
//! Up and Down move focus. Left collapses the focused group, Right expands it, and Enter or
//! Space toggles a focused header or activates a focused row.

use egui::accesskit::Role;
use egui::{
    Align, CornerRadius, Event, EventFilter, FontId, Id, Key, Layout, Modifiers, Pos2, Rect, ScrollArea,
    Sense, Stroke, Ui, UiBuilder, Vec2, WidgetInfo,
};

use super::RowId;
use super::data_grid::{CellCtx, Selection, SelectionMode};
use crate::data::rows::{
    HeightCache, MAX_MEASURED_HEIGHT, find_row, row_offsets, select_only, toggle, visible_rows_variable,
};
use crate::foundation::focus::paint_focus_ring;
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical};

/// One group: a sticky header and the rows shown while it is expanded.
#[derive(Clone, Debug)]
pub struct Group {
    /// Stable id. Expansion is remembered under this, not under the group's index.
    pub id: RowId,
    /// Header text, also the accessible name of the group.
    pub label: String,
    /// Rows the group holds. A collapsed group draws none of them.
    pub row_count: usize,
}

/// State of a grouped list. Keep it in the app and pass `&mut` every frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GroupedListState {
    /// Selected rows. Headers are never selected.
    pub selection: Selection,
    /// Focused header or row. `None` before the list is focused.
    pub focus: Option<RowId>,
    /// Last known index of `focus` in the flattened list.
    pub focus_hint: usize,
    /// Groups whose expansion differs from the default. With the default (start expanded) this
    /// lists collapsed groups; with [`GroupedList::start_collapsed`] it lists expanded ones.
    pub toggled: Vec<RowId>,
}

impl GroupedListState {
    /// Whether `group` is expanded.
    #[must_use]
    pub fn is_expanded(&self, group: RowId, start_collapsed: bool) -> bool {
        let listed = self.toggled.contains(&group);
        if start_collapsed { listed } else { !listed }
    }

    fn set_expanded(&mut self, group: RowId, expanded: bool, start_collapsed: bool) {
        let list_it = expanded == start_collapsed;
        let has = self.toggled.contains(&group);
        if list_it && !has {
            self.toggled.push(group);
        }
        if !list_it && has {
            self.toggled.retain(|id| *id != group);
        }
    }
}

/// What [`GroupedList::show`] reports.
#[derive(Clone, Debug)]
pub struct GroupedListResponse {
    /// The scroll area.
    pub response: egui::Response,
    /// Selection changed this frame.
    pub selection_changed: bool,
    /// A group was expanded or collapsed this frame.
    pub expansion_changed: bool,
    /// Enter or Space on a focused row.
    pub activated: Option<CellCtx>,
    /// Header stuck to the top of the viewport, if one is.
    pub sticky: Option<RowId>,
    /// How many row closures ran this frame (headers are not counted).
    pub rows_drawn: usize,
}

/// One flattened entry.
struct Slot {
    id: RowId,
    group: RowId,
    group_index: usize,
    /// `None` for the header.
    row: Option<usize>,
    position: usize,
    set_size: usize,
    expanded: bool,
}

/// A virtualised list of expandable groups.
///
/// ```
/// # use fluent2_egui::data::grouped_list::{Group, GroupedList, GroupedListState};
/// # use fluent2_egui::data::RowId;
/// # fn demo(ui: &mut egui::Ui) {
/// let groups = [Group { id: RowId(1), label: "Today".into(), row_count: 4 }];
/// let mut state = GroupedListState::default();
/// let _ = GroupedList::new("inbox", &groups).show(ui, &mut state, |cell, ui| {
///     ui.label(format!("Message {}", cell.row));
/// });
/// # }
/// ```
pub struct GroupedList<'a> {
    id: Id,
    groups: &'a [Group],
    row_id: Option<Box<dyn Fn(RowId, usize) -> RowId + 'a>>,
    selection_mode: SelectionMode,
    row_height: Option<f32>,
    header_height: Option<f32>,
    estimate: f32,
    label: Option<String>,
    start_collapsed: bool,
}

fn hashed<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

impl<'a> GroupedList<'a> {
    /// Groups in order. `id_salt` keeps the widget id stable.
    #[must_use]
    pub fn new(id_salt: impl std::hash::Hash, groups: &'a [Group]) -> Self {
        Self {
            id: Id::unique(hashed(&id_salt)),
            groups,
            row_id: None,
            selection_mode: SelectionMode::Single,
            row_height: None,
            header_height: None,
            estimate: 32.0,
            label: None,
            start_collapsed: false,
        }
    }

    /// Id of the row at `index` in `group`. The default mixes the two so ids stay unique.
    #[must_use]
    pub fn row_id(mut self, f: impl Fn(RowId, usize) -> RowId + 'a) -> Self {
        self.row_id = Some(Box::new(f));
        self
    }

    /// How rows are selected. Headers are never selected. Default: [`SelectionMode::Single`].
    #[must_use]
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Minimum row height. Default: `Theme::data.list_row_height`.
    #[must_use]
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = Some(height);
        self
    }

    /// Header height. Default: `Theme::data.list_row_height`.
    #[must_use]
    pub fn header_height(mut self, height: f32) -> Self {
        self.header_height = Some(height);
        self
    }

    /// Height used for a row that has not been measured yet.
    #[must_use]
    pub fn estimate(mut self, height: f32) -> Self {
        self.estimate = height;
        self
    }

    /// Accessible name of the list.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Groups start collapsed. The default is expanded.
    #[must_use]
    pub fn start_collapsed(mut self) -> Self {
        self.start_collapsed = true;
        self
    }

    /// Draw the list. `row` paints one visible row of an expanded group (`cell.row` is the
    /// index inside that group). Collapsed groups and rows outside the viewport are skipped.
    pub fn show(
        self,
        ui: &mut Ui,
        state: &mut GroupedListState,
        mut row: impl FnMut(CellCtx, &mut Ui),
    ) -> GroupedListResponse {
        let theme = Theme::get(ui.ctx());
        let row_min = finite_min(self.row_height, theme.data.list_row_height);
        let header_h = finite_min(self.header_height, theme.data.list_row_height);
        let estimate = if self.estimate.is_finite() {
            self.estimate.clamp(row_min, MAX_MEASURED_HEIGHT)
        } else {
            row_min
        };
        let fill_hover = c(theme.color.subtle_background_hover);
        let fill_selected = c(theme.color.subtle_background_selected);
        let header_fill = c(theme.color.subtle_background_hover);
        let ink = c(theme.color.neutral_foreground_1);
        let stroke = Stroke::new(1.0, c(theme.color.neutral_stroke_2));
        let ring = CornerRadius::same(border_radius::MEDIUM as u8);
        let start_collapsed = self.start_collapsed;
        let mode = self.selection_mode;
        let id = self.id;

        let mut slots: Vec<Slot> = Vec::new();
        for (group_index, group) in self.groups.iter().enumerate() {
            let expanded = state.is_expanded(group.id, start_collapsed);
            slots.push(Slot {
                id: group.id,
                group: group.id,
                group_index,
                row: None,
                position: 0,
                set_size: group.row_count,
                expanded,
            });
            if !expanded {
                continue;
            }
            for index in 0..group.row_count {
                let row_id = self.row_id.as_ref().map_or_else(
                    || RowId(group.id.0.wrapping_mul(1_000_003).wrapping_add(index as u64)),
                    |f| f(group.id, index),
                );
                slots.push(Slot {
                    id: row_id,
                    group: group.id,
                    group_index,
                    row: Some(index),
                    position: index + 1,
                    set_size: group.row_count,
                    expanded: true,
                });
            }
        }

        let cache_id = id.with("grouped_heights");
        let mut cache = ui
            .ctx()
            .data_mut(|data| data.remove_temp::<HeightCache>(cache_id))
            .unwrap_or_default();
        let offsets = row_offsets(slots.len(), &|index| match slots.get(index) {
            Some(slot) if slot.row.is_none() => header_h,
            Some(slot) => cache.get(slot.id, estimate).max(row_min),
            None => estimate,
        });

        let groups = self.groups;
        let label = self.label.clone();
        let mut selection_changed = false;
        let mut expansion_changed = false;
        let mut activated = None;
        let mut sticky_id = None;
        let mut rows_drawn = 0usize;

        let scroll = ScrollArea::vertical()
            .id_salt(id)
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let content_h = offsets.last().copied().unwrap_or(0.0);
                let (content, _) =
                    ui.allocate_exact_size(Vec2::new(ui.available_width(), content_h), Sense::hover());
                let origin = content.min;
                let width = content.width();
                let clip = ui.clip_rect();
                let list_response = ui.interact(clip, id, Sense::click());
                let visible = visible_rows_variable(viewport, &offsets);
                let page = visible.len().saturating_sub(1).max(1);
                let focused = list_response.has_focus();
                let slot_id = |index: usize| slots.get(index).map(|slot| slot.id).unwrap_or(RowId(u64::MAX));

                let mut moved = false;
                if focused {
                    ui.memory_mut(|memory| {
                        memory.set_focus_lock_filter(
                            id,
                            EventFilter {
                                tab: false,
                                horizontal_arrows: true,
                                vertical_arrows: true,
                                escape: false,
                            },
                        );
                    });
                    for (key, modifiers) in keys(ui) {
                        match key {
                            Key::ArrowUp
                            | Key::ArrowDown
                            | Key::Home
                            | Key::End
                            | Key::PageUp
                            | Key::PageDown => {
                                step_focus(state, slots.len(), &slot_id, key, page);
                                moved = true;
                                if mode != SelectionMode::None
                                    && !modifiers.command
                                    && let Some(index) = focus_at(state, slots.len(), &slot_id)
                                    && slots.get(index).is_some_and(|slot| slot.row.is_some())
                                {
                                    select_only(&mut state.selection, slot_id(index));
                                    selection_changed = true;
                                }
                            }
                            Key::ArrowLeft => {
                                expansion_changed |=
                                    set_focused_group(state, &slots, &slot_id, false, start_collapsed);
                            }
                            Key::ArrowRight => {
                                expansion_changed |=
                                    set_focused_group(state, &slots, &slot_id, true, start_collapsed);
                            }
                            Key::Enter | Key::Space => {
                                if let Some(index) = focus_at(state, slots.len(), &slot_id)
                                    && let Some(slot) = slots.get(index)
                                {
                                    if let Some(row_index) = slot.row {
                                        activated = Some(CellCtx {
                                            row: row_index,
                                            row_id: slot.id,
                                            column: "",
                                            column_index: 0,
                                            selected: state.selection.selected.contains(&slot.id),
                                            focused: true,
                                        });
                                    } else {
                                        let open = state.is_expanded(slot.group, start_collapsed);
                                        state.set_expanded(slot.group, !open, start_collapsed);
                                        expansion_changed = true;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }

                if moved && let Some(index) = focus_at(state, slots.len(), &slot_id) {
                    let top = offsets.get(index).copied().unwrap_or(0.0);
                    let height = offsets.get(index + 1).copied().unwrap_or(top) - top;
                    ui.scroll_to_rect(
                        Rect::from_min_size(Pos2::new(origin.x, origin.y + top), Vec2::new(width, height)),
                        None,
                    );
                }

                let focus_index = focus_at(state, slots.len(), &slot_id);
                let mut clicked = false;
                let mut heights_changed = false;
                let mut active = None;

                // Last header whose top is at or above the viewport, and whose successor has
                // not yet reached the pin line, is the sticky one.
                let sticky_index = (0..slots.len())
                    .filter(|&index| slots.get(index).is_some_and(|slot| slot.row.is_none()))
                    .rfind(|&index| offsets.get(index).copied().unwrap_or(0.0) <= viewport.min.y + 0.5)
                    .filter(|&index| {
                        let next = ((index + 1)..slots.len())
                            .find(|&i| slots.get(i).is_some_and(|slot| slot.row.is_none()));
                        next.is_none_or(|next| {
                            offsets.get(next).copied().unwrap_or(f32::MAX) > viewport.min.y + header_h * 0.5
                        })
                    });

                for index in visible.clone() {
                    let Some(slot_group) = slots.get(index).map(|slot| slot.group) else {
                        continue;
                    };
                    let top = origin.y + offsets.get(index).copied().unwrap_or(0.0);
                    let height = (offsets.get(index + 1).copied().unwrap_or(0.0)
                        - offsets.get(index).copied().unwrap_or(0.0))
                    .max(1.0);
                    let rect = Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, height));
                    let is_header = slots.get(index).is_some_and(|slot| slot.row.is_none());
                    if is_header {
                        let text = groups
                            .get(slots[index].group_index)
                            .map(|group| group.label.as_str())
                            .unwrap_or("");
                        let expanded = slots[index].expanded;
                        paint_header(ui, rect, text, expanded, header_fill, ink, stroke);
                        let hit = ui.interact(rect, id.with(("hdr", slots[index].id.0)), Sense::click());
                        hit.widget_info(|| WidgetInfo::labeled(Role::Unknown, ui.is_enabled(), text));
                        let set_size = slots[index].set_size;
                        ui.ctx().accesskit_node_builder(hit.id, |node| {
                            node.set_role(Role::Group);
                            node.set_label(text);
                            node.set_expanded(expanded);
                            if set_size > 0 {
                                node.set_size_of_set(set_size);
                            }
                        });
                        if hit.clicked() {
                            clicked = true;
                            state.focus = Some(slots[index].id);
                            state.focus_hint = index;
                            let open = state.is_expanded(slot_group, start_collapsed);
                            state.set_expanded(slot_group, !open, start_collapsed);
                            expansion_changed = true;
                        }
                    } else {
                        let slot_id_now = slots[index].id;
                        let row_index = slots[index].row.unwrap_or(0);
                        let selected = state.selection.selected.contains(&slot_id_now);
                        if selected {
                            ui.painter().rect_filled(rect, 0.0, fill_selected);
                        } else if ui.rect_contains_pointer(rect) {
                            ui.painter().rect_filled(rect, 0.0, fill_hover);
                        }
                        let hit = ui.interact(rect, id.with(("row", slot_id_now.0)), Sense::click());
                        // A rect whose min is the row's screen position makes min_rect().height()
                        // include that offset, so the row would grow every frame. Measure in a
                        // rect that starts at y 0.
                        let inner = Rect::from_min_size(
                            Pos2::new(rect.min.x + spacing_horizontal::S, spacing_vertical::S_NUDGE),
                            Vec2::new(
                                (rect.width() - 2.0 * spacing_horizontal::S).max(1.0),
                                MAX_MEASURED_HEIGHT,
                            ),
                        );
                        let mut child = ui.new_child(
                            UiBuilder::new()
                                .max_rect(inner)
                                .layout(Layout::top_down(Align::Min))
                                .id_salt(("grow", slot_id_now.0))
                                .accessibility_parent(hit.id),
                        );
                        child.set_clip_rect(rect.intersect(clip));
                        row(
                            CellCtx {
                                row: row_index,
                                row_id: slot_id_now,
                                column: "",
                                column_index: 0,
                                selected,
                                focused: focused && focus_index == Some(index),
                            },
                            &mut child,
                        );
                        rows_drawn += 1;
                        let measured = child.min_rect().height() + 2.0 * spacing_vertical::S_NUDGE;
                        heights_changed |= cache.set(slot_id_now, measured.max(row_min));
                        let name = groups
                            .get(slots[index].group_index)
                            .map(|group| format!("{} {}", group.label, row_index + 1))
                            .unwrap_or_default();
                        hit.widget_info(|| {
                            WidgetInfo::labeled(Role::Unknown, ui.is_enabled(), name.as_str())
                        });
                        let position = slots[index].position;
                        let set_size = slots[index].set_size;
                        ui.ctx().accesskit_node_builder(hit.id, |node| {
                            if mode == SelectionMode::None {
                                node.set_role(Role::ListItem);
                            } else {
                                node.set_role(Role::ListBoxOption);
                                node.set_selected(selected);
                            }
                            node.set_label(name.as_str());
                            node.set_position_in_set(position);
                            node.set_size_of_set(set_size);
                        });
                        if focused && focus_index == Some(index) {
                            active = Some(hit.id.accesskit_id());
                        }
                        if hit.clicked() {
                            clicked = true;
                            state.focus = Some(slot_id_now);
                            state.focus_hint = index;
                            if mode == SelectionMode::Multi && ui.input(|input| input.modifiers.command) {
                                toggle(&mut state.selection, slot_id_now);
                            } else if mode != SelectionMode::None {
                                select_only(&mut state.selection, slot_id_now);
                            }
                            selection_changed = true;
                        }
                    }
                }

                if let Some(index) = sticky_index {
                    let real_top = origin.y + offsets.get(index).copied().unwrap_or(0.0);
                    if real_top < clip.min.y - 0.5 {
                        let stuck =
                            Rect::from_min_size(Pos2::new(origin.x, clip.min.y), Vec2::new(width, header_h));
                        let text = groups
                            .get(slots[index].group_index)
                            .map(|group| group.label.as_str())
                            .unwrap_or("");
                        paint_header(ui, stuck, text, slots[index].expanded, header_fill, ink, stroke);
                        sticky_id = Some(slots[index].group);
                    }
                }

                if focused && let Some(index) = focus_index {
                    let top = origin.y + offsets.get(index).copied().unwrap_or(0.0);
                    let height = (offsets.get(index + 1).copied().unwrap_or(0.0)
                        - offsets.get(index).copied().unwrap_or(0.0))
                    .max(1.0);
                    paint_focus_ring(
                        ui,
                        Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, height)).shrink(2.0),
                        ring,
                    );
                }
                if clicked {
                    list_response.request_focus();
                }
                let enabled = ui.is_enabled();
                list_response.widget_info(|| match &label {
                    Some(text) => WidgetInfo::labeled(Role::Unknown, enabled, text.as_str()),
                    None => WidgetInfo::new(Role::Unknown),
                });
                ui.ctx().accesskit_node_builder(list_response.id, |node| {
                    node.set_role(if mode == SelectionMode::None {
                        Role::List
                    } else {
                        Role::ListBox
                    });
                    if let Some(text) = &label {
                        node.set_label(text.as_str());
                    }
                    if let Some(row_node) = active {
                        node.set_active_descendant(row_node);
                    }
                });
                (list_response, heights_changed)
            });

        let (response, heights_changed) = scroll.inner;
        if heights_changed {
            ui.ctx().request_repaint();
        }
        ui.ctx().data_mut(|data| data.insert_temp(cache_id, cache));
        GroupedListResponse {
            response,
            selection_changed,
            expansion_changed,
            activated,
            sticky: sticky_id,
            rows_drawn,
        }
    }
}

fn finite_min(requested: Option<f32>, fallback: f32) -> f32 {
    requested
        .filter(|height| height.is_finite() && *height >= 1.0)
        .unwrap_or(fallback)
        .max(1.0)
}

fn paint_header(
    ui: &Ui,
    rect: Rect,
    text: &str,
    expanded: bool,
    fill: egui::Color32,
    ink: egui::Color32,
    stroke: Stroke,
) {
    ui.painter().rect_filled(rect, 0.0, fill);
    ui.painter().hline(rect.x_range(), rect.bottom(), stroke);
    // A stroked chevron, same shape as TreeView, so it does not need a glyph the
    // proportional font may not have.
    let slot = Rect::from_min_size(
        Pos2::new(rect.min.x + spacing_horizontal::XS, rect.min.y),
        Vec2::new(spacing_horizontal::L, rect.height()),
    );
    paint_chevron(ui, slot, if expanded { 1.0 } else { 0.0 }, ink);
    let galley = ui
        .painter()
        .layout_no_wrap(text.to_owned(), FontId::proportional(13.0), ink);
    let pos = Pos2::new(slot.max.x, rect.center().y - galley.size().y * 0.5);
    ui.painter().galley(pos, galley, ink);
}

/// Pointing right when collapsed (`turn` 0) and down when expanded (`turn` 1).
fn paint_chevron(ui: &Ui, slot: Rect, turn: f32, color: egui::Color32) {
    let arm_x = spacing_horizontal::S / 4.0;
    let arm_y = spacing_horizontal::S / 2.0;
    let rotation = egui::emath::Rot2::from_angle(turn.clamp(0.0, 1.0) * std::f32::consts::FRAC_PI_2);
    let centre = slot.center().to_vec2();
    let points = [
        Vec2::new(-arm_x, -arm_y),
        Vec2::new(arm_x, 0.0),
        Vec2::new(-arm_x, arm_y),
    ]
    .map(|offset| (centre + rotation * offset).to_pos2());
    ui.painter().line(points.to_vec(), Stroke::new(1.0, color));
}

fn focus_at(state: &GroupedListState, len: usize, ids: &dyn Fn(usize) -> RowId) -> Option<usize> {
    state
        .focus
        .and_then(|id| find_row(state.focus_hint, len, ids, id))
}

fn step_focus(state: &mut GroupedListState, len: usize, ids: &dyn Fn(usize) -> RowId, key: Key, page: usize) {
    if len == 0 {
        return;
    }
    let now = focus_at(state, len, ids).unwrap_or(0);
    let next = match key {
        Key::ArrowUp => now.saturating_sub(1),
        Key::ArrowDown => (now + 1).min(len - 1),
        Key::Home => 0,
        Key::End => len - 1,
        Key::PageUp => now.saturating_sub(page),
        Key::PageDown => (now + page).min(len - 1),
        _ => now,
    };
    state.focus = Some(ids(next));
    state.focus_hint = next;
}

/// Collapse or expand the group that owns the focused entry. Returns whether it changed.
fn set_focused_group(
    state: &mut GroupedListState,
    slots: &[Slot],
    ids: &dyn Fn(usize) -> RowId,
    expanded: bool,
    start_collapsed: bool,
) -> bool {
    let Some(index) = focus_at(state, slots.len(), ids) else {
        return false;
    };
    let Some(slot) = slots.get(index) else {
        return false;
    };
    let group = slot.group;
    if state.is_expanded(group, start_collapsed) == expanded {
        return false;
    }
    state.set_expanded(group, expanded, start_collapsed);
    if !expanded {
        state.focus = Some(group);
        state.focus_hint = slots
            .iter()
            .position(|slot| slot.row.is_none() && slot.group == group)
            .unwrap_or(index);
    }
    true
}

fn keys(ui: &Ui) -> Vec<(Key, Modifiers)> {
    ui.input(|input| {
        input
            .events
            .iter()
            .filter_map(|event| match event {
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if matches!(
                    key,
                    Key::ArrowUp
                        | Key::ArrowDown
                        | Key::ArrowLeft
                        | Key::ArrowRight
                        | Key::Home
                        | Key::End
                        | Key::PageUp
                        | Key::PageDown
                        | Key::Enter
                        | Key::Space
                ) =>
                {
                    Some((*key, *modifiers))
                }
                _ => None,
            })
            .collect()
    })
}
