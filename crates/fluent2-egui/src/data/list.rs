//! List: a virtualised one-column list with no header (`docs/specs/data-views.md` §6).
//!
//! The List uses the shared viewport maths, selection and roving focus in `rows.rs` (D7), so its
//! selection behaves as the DataGrid's does. Only the rows in view are laid out and passed to the
//! row closure. With [`List::variable_height`] rows are measured and cached, and the viewport is
//! anchored to its top row (§6.1).

use egui::accesskit::Role;
use egui::{
    Align, CornerRadius, Event, EventFilter, Id, Key, Layout, Modifiers, Pos2, Rect, ScrollArea, Sense, Ui,
    UiBuilder, Vec2, WidgetInfo, WidgetType,
};

use super::RowId;
use super::data_grid::{CellCtx, Selection, SelectionMode};
use crate::data::rows::{
    HeightCache, MAX_MEASURED_HEIGHT, ScrollAnchor, anchor_at, anchored_scroll, find_row, row_offsets,
    select_all, select_only, select_range, toggle, visible_rows, visible_rows_variable,
};
use crate::foundation::focus::paint_focus_ring;
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, spacing_vertical};

/// State of a List. Store it in the app and pass `&mut` to every `show`.
///
/// ```
/// let state = fluent2_egui::data::list::ListState::default();
/// assert!(state.focus.is_none());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ListState {
    /// Selected rows, by id, and the anchor for Shift ranges.
    pub selection: Selection,
    /// Focused row. `None` before first focus.
    pub focus: Option<RowId>,
    /// Last known index of `focus`, checked first when the focus is looked up (§3.4).
    pub focus_hint: usize,
}

/// Hash a salt to the `u64` that names the list's id.
fn hashed<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut hasher);
    hasher.finish()
}

/// What a List reports after `show`.
#[derive(Clone, Debug)]
pub struct ListResponse {
    /// The whole list. `has_focus` while the list takes the keyboard.
    pub response: egui::Response,
    /// The selection changed this frame.
    pub selection_changed: bool,
    /// Enter on the focused row.
    pub activated: Option<CellCtx>,
    /// Secondary click or Shift+F10 on a row. Attach a context menu to this.
    pub context_menu: Option<CellCtx>,
}

/// A virtualised list. Build it, then call [`List::show`].
///
/// ```
/// # use fluent2_egui::data::SelectionMode;
/// # use fluent2_egui::data::list::{List, ListState};
/// # fn demo(ui: &mut egui::Ui) {
/// let mut state = ListState::default();
/// let _response = List::new("recent", 1_000)
///     .label("Recent files")
///     .selection_mode(SelectionMode::Multi)
///     .show(ui, &mut state, |cell, ui| {
///         ui.label(format!("File {}", cell.row));
///     });
/// # }
/// ```
pub struct List<'a> {
    id: Id,
    row_count: usize,
    row_id: Option<Box<dyn Fn(usize) -> RowId + 'a>>,
    selection_mode: SelectionMode,
    row_height: Option<f32>,
    estimate: Option<f32>,
    label: Option<String>,
    row_label: Option<Box<dyn Fn(usize) -> String + 'a>>,
}

/// Per-list memory of the variable-height mode (§6.1): measured heights and the scroll anchor.
#[derive(Clone, Debug, Default)]
struct VariableRows {
    cache: HeightCache,
    anchor: Option<ScrollAnchor>,
    /// A keyboard move to reveal, kept until the rows around it stop changing height. It wins
    /// over the anchor.
    reveal: Option<RowId>,
    /// Frames the reveal has left, so it can never hold the scroll position for long.
    reveal_frames: u8,
    last_scroll: f32,
}

/// Where rows sit: one height for all (D2), or measured prefix offsets (§6.1).
enum Geometry {
    Fixed(f32),
    Variable(Vec<f32>),
}

impl Geometry {
    fn top(&self, index: usize) -> f32 {
        match self {
            Self::Fixed(h) => index as f32 * h,
            Self::Variable(offsets) => offsets.get(index).copied().unwrap_or(0.0),
        }
    }

    fn height(&self, index: usize) -> f32 {
        match self {
            Self::Fixed(h) => *h,
            Self::Variable(offsets) => offsets.get(index + 1).copied().unwrap_or(0.0) - self.top(index),
        }
    }

    fn content_height(&self, row_count: usize) -> f32 {
        match self {
            Self::Fixed(h) => row_count as f32 * h,
            Self::Variable(offsets) => offsets.last().copied().unwrap_or(0.0),
        }
    }

    fn visible(&self, viewport: Rect, row_count: usize) -> std::ops::Range<usize> {
        match self {
            Self::Fixed(h) => visible_rows(viewport, 0.0, *h, row_count),
            Self::Variable(offsets) => visible_rows_variable(viewport, offsets),
        }
    }
}

/// Most frames a keyboard reveal is repeated while the rows around it are measured.
const REVEAL_FRAMES: u8 = 30;

fn variable_id(id: Id) -> Id {
    id.with("variable_rows")
}

impl<'a> List<'a> {
    /// A list of `row_count` rows. `id_salt` keeps the list's id stable across frames.
    #[must_use]
    pub fn new(id_salt: impl std::hash::Hash, row_count: usize) -> Self {
        Self {
            id: Id::new(hashed(&id_salt)),
            row_count,
            row_id: None,
            selection_mode: SelectionMode::Single,
            row_height: None,
            estimate: None,
            label: None,
            row_label: None,
        }
    }

    /// Accessible text of each row, from its index (WI-1848).
    ///
    /// AccessKit derives no name from a ListBoxOption's or ListItem's children, so without this
    /// a screen reader says nothing as focus moves through the list. Usually the row's main text.
    #[must_use]
    pub fn row_label(mut self, f: impl Fn(usize) -> String + 'a) -> Self {
        self.row_label = Some(Box::new(f));
        self
    }

    /// Rows of different heights (§6.1). Each row is laid out top to bottom, measured when it is
    /// shown and cached by its [`RowId`]; rows not measured yet count as `estimate` points. The
    /// row at the top of the viewport stays put when rows above it change height or are
    /// inserted. [`Self::row_height`] becomes the minimum height.
    #[must_use]
    pub fn variable_height(mut self, estimate: f32) -> Self {
        self.estimate = Some(estimate);
        self
    }

    /// Forget the measured heights of the variable-height list made with `id_salt`, for example
    /// after the font or the content of many rows changed. Rows are measured again as shown.
    pub fn forget_heights(ctx: &egui::Context, id_salt: impl std::hash::Hash) {
        let id = variable_id(Id::new(hashed(&id_salt)));
        ctx.data_mut(|data| data.get_temp_mut_or_default::<VariableRows>(id).cache.clear());
    }

    /// The id of row `index` (D3). The default is the index itself.
    #[must_use]
    pub fn row_id(mut self, f: impl Fn(usize) -> RowId + 'a) -> Self {
        self.row_id = Some(Box::new(f));
        self
    }

    /// Which rows can be selected. The default is [`SelectionMode::Single`].
    #[must_use]
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Row height in points. The default is `Theme::data.list_row_height`.
    #[must_use]
    pub fn row_height(mut self, height: f32) -> Self {
        self.row_height = Some(height);
        self
    }

    /// Accessible name of the list.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Draw the list. `row` draws the content of each visible row; it is not called for rows out
    /// of view.
    ///
    /// Arrow keys, Home, End, PageUp and PageDown move focus while the list has it. Enter
    /// activates the focused row. Ctrl and Shift select in Multi mode, as in the DataGrid.
    pub fn show(
        self,
        ui: &mut Ui,
        state: &mut ListState,
        mut row: impl FnMut(CellCtx, &mut Ui),
    ) -> ListResponse {
        let theme = Theme::get(ui.ctx());
        let requested = self.row_height.unwrap_or(theme.data.list_row_height);
        let row_height = if requested.is_finite() {
            requested.max(1.0)
        } else {
            theme.data.list_row_height
        };
        let fill_hover = c(theme.color.subtle_background_hover);
        let fill_pressed = c(theme.color.subtle_background_pressed);
        let fill_selected = c(theme.color.subtle_background_selected);
        let ink_hover = c(theme.color.neutral_foreground_1_hover);
        let ink_pressed = c(theme.color.neutral_foreground_1_pressed);
        let ink_selected = c(theme.color.neutral_foreground_1_selected);
        let ring_radius = CornerRadius::same(border_radius::MEDIUM as u8);

        let List {
            id,
            row_count,
            row_id,
            selection_mode: mode,
            label,
            estimate,
            ..
        } = self;
        let ids_fn = |index: usize| row_id.as_ref().map_or(RowId(index as u64), |f| f(index));
        let ids: &dyn Fn(usize) -> RowId = &ids_fn;

        // Variable-height mode: take the measurements out of memory for this frame.
        let estimate = estimate.map(|e| {
            if e.is_finite() {
                e.clamp(row_height, MAX_MEASURED_HEIGHT)
            } else {
                row_height
            }
        });
        let mut vars = estimate.map(|_| {
            ui.ctx()
                .data_mut(|data| data.remove_temp::<VariableRows>(variable_id(id)))
                .unwrap_or_default()
        });
        let geometry = match (&vars, estimate) {
            (Some(vars), Some(estimate)) => Geometry::Variable(row_offsets(row_count, &|row| {
                vars.cache.get(ids(row), estimate).max(row_height)
            })),
            _ => Geometry::Fixed(row_height),
        };
        let mut scroll = ScrollArea::vertical().id_salt(id).auto_shrink([false, false]);
        if let (Some(vars), Geometry::Variable(offsets)) = (&vars, &geometry)
            && vars.reveal.is_none()
            && let Some(anchor) = vars.anchor
            && let Some(y) = anchored_scroll(anchor, offsets, ids)
            && (y - vars.last_scroll).abs() > 0.5
        {
            // Rows above the viewport changed height: keep the anchored row where it was.
            scroll = scroll.vertical_scroll_offset(y);
        }

        let output = scroll.show_viewport(ui, |ui, viewport| {
            let (content, _) = ui.allocate_exact_size(
                Vec2::new(ui.available_width(), geometry.content_height(row_count)),
                Sense::hover(),
            );
            let origin = content.min;
            let width = content.width();
            let clip = ui.clip_rect();
            let list_response = ui.interact(clip, id, Sense::click());
            let visible = geometry.visible(viewport, row_count);
            let mut heights_changed = false;
            let page = visible.len().saturating_sub(1).max(1);
            let focused = list_response.has_focus();

            let mut reports = Reports::default();
            let mut moved = false;
            let before = state.selection.clone();

            if focused {
                // Arrow keys move focus in the list, not to a neighbouring widget.
                let lock = EventFilter {
                    tab: false,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: false,
                };
                ui.memory_mut(|memory| memory.set_focus_lock_filter(id, lock));
                for (key, modifiers) in pressed_keys(ui) {
                    let out = handle_key(state, mode, row_count, ids, key, modifiers, page);
                    moved |= out.moved;
                    if out.activate || out.context_menu {
                        let current = state
                            .focus
                            .and_then(|f| find_row(state.focus_hint, row_count, ids, f));
                        if let Some(index) = current {
                            let ctx = cell_ctx(state, index, ids(index), focused, mode);
                            if out.activate {
                                reports.activated = Some(ctx);
                            }
                            if out.context_menu {
                                reports.context_menu = Some(ctx);
                            }
                        }
                    }
                }
            }
            let focus_index = state
                .focus
                .and_then(|f| find_row(state.focus_hint, row_count, ids, f));
            if moved && let Some(vars) = vars.as_mut() {
                vars.reveal = state.focus;
                vars.reveal_frames = REVEAL_FRAMES;
            }
            let pending = vars.as_ref().and_then(|v| v.reveal);
            let reveal_index = pending.and_then(|r| find_row(state.focus_hint, row_count, ids, r));
            if let Some(index) = reveal_index.or(focus_index.filter(|_| moved)) {
                // Keyboard moves bring the focused row into view (§3.3). Measured rows repeat
                // this until their heights settle.
                let top = origin.y + geometry.top(index);
                ui.scroll_to_rect(
                    Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, geometry.height(index))),
                    None,
                );
            }

            let mut clicked = false;
            // The focused row, for the list's active descendant (see TreeView).
            let mut active_row = None;
            for index in visible.clone() {
                let top = origin.y + geometry.top(index);
                let row_rect =
                    Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, geometry.height(index)));
                let row_id = ids(index);
                // The row's own response goes first, so the content drawn after it takes
                // clicks in its own rects.
                let hit = ui.interact(row_rect.intersect(clip), id.with(("row", row_id)), Sense::CLICK);
                if focus_index == Some(index) {
                    active_row = Some(hit.id.accesskit_id());
                }
                if hit.clicked() {
                    clicked = true;
                    let modifiers = ui.input(|input| input.modifiers);
                    reports.selection_changed |= click(state, mode, row_count, ids, index, modifiers);
                }
                if hit.secondary_clicked() {
                    clicked = true;
                    reports.context_menu = Some(cell_ctx(state, index, row_id, focused, mode));
                }

                let selected = state.selection.selected.contains(&row_id);
                // High contrast fills these rows with the highlight colour, so row text takes
                // the matching foreground.
                let fill = if selected {
                    Some((fill_selected, ink_selected))
                } else if hit.is_pointer_button_down_on() {
                    Some((fill_pressed, ink_pressed))
                } else if hit.hovered() {
                    Some((fill_hover, ink_hover))
                } else {
                    None
                };
                if let Some((fill, _)) = fill {
                    ui.painter().rect_filled(row_rect, ring_radius, fill);
                }
                hit.widget_info(|| WidgetInfo::new(WidgetType::Other));
                let row_text = self.row_label.as_ref().map(|label| label(index));
                ui.ctx().accesskit_node_builder(hit.id, |node| {
                    if let Some(text) = row_text {
                        node.set_label(text);
                    }
                    if mode == SelectionMode::None {
                        node.set_role(Role::ListItem);
                    } else {
                        node.set_role(Role::ListBoxOption);
                        node.set_selected(selected);
                    }
                    node.set_position_in_set(index + 1);
                    node.set_size_of_set(row_count);
                });

                // Hang the row's content under its option node, so it is read as the row.
                let builder = if vars.is_some() {
                    // Measured rows lay out top to bottom with no height limit.
                    let inner = Rect::from_min_size(
                        row_rect.min + Vec2::new(spacing_horizontal::S, spacing_vertical::S_NUDGE),
                        Vec2::new(
                            (width - 2.0 * spacing_horizontal::S).max(1.0),
                            MAX_MEASURED_HEIGHT,
                        ),
                    );
                    UiBuilder::new()
                        .max_rect(inner)
                        .layout(Layout::top_down(Align::Min))
                } else {
                    UiBuilder::new()
                        .max_rect(row_rect.shrink2(Vec2::new(spacing_horizontal::S, 0.0)))
                        .layout(Layout::left_to_right(Align::Center))
                };
                let mut child = ui.new_child(builder.accessibility_parent(hit.id));
                child.set_clip_rect(row_rect.intersect(clip));
                // A selectable label would take the click meant for the row (§3).
                child.style_mut().interaction.selectable_labels = false;
                if let Some((_, ink)) = fill {
                    child.visuals_mut().override_text_color = Some(ink);
                }
                row(
                    CellCtx {
                        row: index,
                        row_id,
                        column: "",
                        column_index: 0,
                        selected,
                        focused: focused && focus_index == Some(index),
                    },
                    &mut child,
                );
                if let Some(vars) = vars.as_mut() {
                    let measured = child.min_rect().height() + 2.0 * spacing_vertical::S_NUDGE;
                    heights_changed |= vars.cache.set(row_id, measured.max(row_height));
                }
            }
            if let (Some(vars), Geometry::Variable(offsets)) = (vars.as_mut(), &geometry) {
                vars.anchor = anchor_at(offsets, viewport.min.y, ids);
                // In view: wholly, or from its top for a row taller than the viewport.
                let revealed = reveal_index.is_none_or(|index| {
                    let top = geometry.top(index);
                    let bottom = (top + geometry.height(index)).min(top + viewport.height());
                    top >= viewport.min.y - 0.5 && bottom <= viewport.max.y + 0.5
                });
                vars.reveal_frames = vars.reveal_frames.saturating_sub(1);
                if (revealed && !heights_changed && !moved) || vars.reveal_frames == 0 {
                    vars.reveal = None;
                }
                vars.last_scroll = viewport.min.y;
                if heights_changed {
                    ui.ctx().request_repaint();
                }
            }

            if focused && let Some(index) = focus_index {
                let top = origin.y + geometry.top(index);
                let rect =
                    Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(width, geometry.height(index)));
                paint_focus_ring(ui, rect.shrink(2.0), ring_radius);
            }
            if clicked {
                list_response.request_focus();
            }

            let enabled = ui.is_enabled();
            list_response.widget_info(|| match &label {
                Some(label) => WidgetInfo::labeled(WidgetType::Other, enabled, label.as_str()),
                None => WidgetInfo::new(WidgetType::Other),
            });
            ui.ctx().accesskit_node_builder(list_response.id, |node| {
                if mode == SelectionMode::None {
                    node.set_role(Role::List);
                } else {
                    node.set_role(Role::ListBox);
                }
                if let Some(label) = &label {
                    node.set_label(label.as_str());
                }
                if mode == SelectionMode::Multi {
                    node.set_multiselectable();
                }
                if let Some(row) = active_row {
                    node.set_active_descendant(row);
                }
            });

            reports.selection_changed |= state.selection != before;
            (list_response, reports)
        });

        let (response, reports) = output.inner;
        if let Some(vars) = vars {
            ui.ctx().data_mut(|data| data.insert_temp(variable_id(id), vars));
        }
        ListResponse {
            response,
            selection_changed: reports.selection_changed,
            activated: reports.activated,
            context_menu: reports.context_menu,
        }
    }
}

/// Reports gathered inside the scroll area.
#[derive(Default)]
struct Reports {
    selection_changed: bool,
    activated: Option<CellCtx>,
    context_menu: Option<CellCtx>,
}

/// What one key press asked for, besides the state changes it made.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct KeyOutcome {
    /// Focus moved. The list scrolls the new focus into view.
    moved: bool,
    /// Enter: report `activated` for the focused row.
    activate: bool,
    /// Shift+F10: report `context_menu` for the focused row.
    context_menu: bool,
}

/// How a move changes the selection, from the modifier keys.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Extend {
    /// Shift: select the range from the anchor.
    shift: bool,
    /// Ctrl: move focus without selecting.
    ctrl: bool,
}

impl Extend {
    fn from(modifiers: Modifiers) -> Self {
        Self {
            shift: modifiers.shift,
            ctrl: modifiers.command || modifiers.ctrl,
        }
    }
}

/// Keys pressed this frame, repeats included, with their modifiers.
fn pressed_keys(ui: &Ui) -> Vec<(Key, Modifiers)> {
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
                } => Some((*key, *modifiers)),
                _ => None,
            })
            .collect()
    })
}

/// The context of the row at `index`, for `activated` and `context_menu`.
fn cell_ctx(state: &ListState, index: usize, row_id: RowId, focused: bool, mode: SelectionMode) -> CellCtx {
    CellCtx {
        row: index,
        row_id,
        column: "",
        column_index: 0,
        selected: mode != SelectionMode::None && state.selection.selected.contains(&row_id),
        focused: focused && state.focus == Some(row_id),
    }
}

/// Apply one key press to the selection and focus (§3.3, row focus).
fn handle_key(
    state: &mut ListState,
    mode: SelectionMode,
    row_count: usize,
    ids: &dyn Fn(usize) -> RowId,
    key: Key,
    modifiers: Modifiers,
    page: usize,
) -> KeyOutcome {
    let mut out = KeyOutcome::default();
    if row_count == 0 {
        return out;
    }
    let extend = Extend::from(modifiers);
    let current = state
        .focus
        .and_then(|f| find_row(state.focus_hint, row_count, ids, f));
    let last = row_count - 1;
    let target = match key {
        Key::ArrowUp => Some(current.map_or(0, |row| row.saturating_sub(1))),
        Key::ArrowDown => Some(current.map_or(0, |row| (row + 1).min(last))),
        Key::PageUp => Some(current.unwrap_or(0).saturating_sub(page)),
        Key::PageDown => Some(current.unwrap_or(0).saturating_add(page).min(last)),
        Key::Home => Some(0),
        Key::End => Some(last),
        _ => None,
    };
    if let Some(to) = target {
        move_to(state, mode, row_count, ids, current, to, extend);
        out.moved = true;
        return out;
    }
    match key {
        Key::Enter => out.activate = current.is_some(),
        Key::F10 if modifiers.shift => out.context_menu = current.is_some(),
        Key::Space if mode == SelectionMode::Multi => {
            if let Some(row) = current {
                let id = ids(row);
                state.focus_hint = row;
                state.focus = Some(id);
                toggle(&mut state.selection, id);
            }
        }
        Key::A if extend.ctrl && mode == SelectionMode::Multi => {
            select_all(&mut state.selection, row_count, ids);
        }
        _ => {}
    }
    out
}

/// Move focus from `from` to `to` and apply the selection rule for a move. Shift extends a range
/// from the anchor, Ctrl only moves focus, and a plain move selects the focused row in Single and
/// Multi.
fn move_to(
    state: &mut ListState,
    mode: SelectionMode,
    row_count: usize,
    ids: &dyn Fn(usize) -> RowId,
    from: Option<usize>,
    to: usize,
    extend: Extend,
) {
    let id = ids(to);
    state.focus = Some(id);
    state.focus_hint = to;
    match mode {
        SelectionMode::None => {}
        SelectionMode::Single => select_only(&mut state.selection, id),
        SelectionMode::Multi if extend.shift => select_from_anchor(state, row_count, ids, from, to, false),
        SelectionMode::Multi if extend.ctrl => {}
        SelectionMode::Multi => select_only(&mut state.selection, id),
    }
}

/// A click on the row at `index`. Returns whether the selection changed.
fn click(
    state: &mut ListState,
    mode: SelectionMode,
    row_count: usize,
    ids: &dyn Fn(usize) -> RowId,
    index: usize,
    modifiers: Modifiers,
) -> bool {
    let before = state.selection.clone();
    let extend = Extend::from(modifiers);
    let id = ids(index);
    state.focus = Some(id);
    state.focus_hint = index;
    match mode {
        SelectionMode::None => {}
        SelectionMode::Single => select_only(&mut state.selection, id),
        SelectionMode::Multi if extend.shift => {
            select_from_anchor(state, row_count, ids, None, index, extend.ctrl)
        }
        SelectionMode::Multi if extend.ctrl => toggle(&mut state.selection, id),
        SelectionMode::Multi => select_only(&mut state.selection, id),
    }
    state.selection != before
}

/// Select the rows between the anchor and `to`. The anchor is `from` when none is set, and is
/// set to `from` then, so a second Shift move extends from the same row.
fn select_from_anchor(
    state: &mut ListState,
    row_count: usize,
    ids: &dyn Fn(usize) -> RowId,
    from: Option<usize>,
    to: usize,
    additive: bool,
) {
    if state.selection.anchor.is_none() {
        state.selection.anchor = from.map(ids);
    }
    let anchor = state
        .selection
        .anchor
        .and_then(|anchor| find_row(0, row_count, ids, anchor))
        .or(from)
        .unwrap_or(to);
    select_range(&mut state.selection, anchor, to, row_count, ids, additive);
}

#[cfg(test)]
mod tests {
    use egui::{Key, Modifiers};

    use super::{ListState, SelectionMode, handle_key};
    use crate::data::RowId;

    fn ids(row: usize) -> RowId {
        RowId(row as u64)
    }

    fn selected(state: &ListState) -> Vec<u64> {
        state.selection.selected.iter().map(|id| id.0).collect()
    }

    #[test]
    fn arrows_move_and_clamp_in_single_mode() {
        let mut state = ListState::default();
        handle_key(
            &mut state,
            SelectionMode::Single,
            3,
            &ids,
            Key::ArrowDown,
            Modifiers::NONE,
            2,
        );
        assert_eq!(selected(&state), vec![0]);
        handle_key(
            &mut state,
            SelectionMode::Single,
            3,
            &ids,
            Key::ArrowUp,
            Modifiers::NONE,
            2,
        );
        handle_key(
            &mut state,
            SelectionMode::Single,
            3,
            &ids,
            Key::ArrowUp,
            Modifiers::NONE,
            2,
        );
        assert_eq!(selected(&state), vec![0]);
        handle_key(
            &mut state,
            SelectionMode::Single,
            3,
            &ids,
            Key::End,
            Modifiers::NONE,
            2,
        );
        handle_key(
            &mut state,
            SelectionMode::Single,
            3,
            &ids,
            Key::ArrowDown,
            Modifiers::NONE,
            2,
        );
        assert_eq!(selected(&state), vec![2]);
    }

    #[test]
    fn shift_arrow_extends_a_range_in_multi_mode() {
        let mut state = ListState::default();
        handle_key(
            &mut state,
            SelectionMode::Multi,
            10,
            &ids,
            Key::ArrowDown,
            Modifiers::NONE,
            4,
        );
        handle_key(
            &mut state,
            SelectionMode::Multi,
            10,
            &ids,
            Key::ArrowDown,
            Modifiers::SHIFT,
            4,
        );
        handle_key(
            &mut state,
            SelectionMode::Multi,
            10,
            &ids,
            Key::ArrowDown,
            Modifiers::SHIFT,
            4,
        );
        assert_eq!(selected(&state), vec![0, 1, 2]);
    }

    #[test]
    fn enter_and_space_in_none_mode() {
        let mut state = ListState::default();
        let out = handle_key(
            &mut state,
            SelectionMode::None,
            3,
            &ids,
            Key::Enter,
            Modifiers::NONE,
            2,
        );
        assert!(!out.activate, "Enter needs a focused row first");
        handle_key(
            &mut state,
            SelectionMode::None,
            3,
            &ids,
            Key::ArrowDown,
            Modifiers::NONE,
            2,
        );
        let out = handle_key(
            &mut state,
            SelectionMode::None,
            3,
            &ids,
            Key::Enter,
            Modifiers::NONE,
            2,
        );
        assert!(out.activate);
        assert!(state.selection.selected.is_empty());
    }

    #[test]
    fn empty_list_ignores_every_key() {
        let mut state = ListState::default();
        let out = handle_key(
            &mut state,
            SelectionMode::Multi,
            0,
            &ids,
            Key::Enter,
            Modifiers::NONE,
            2,
        );
        assert!(!out.activate);
        assert!(state.focus.is_none());
    }
}
