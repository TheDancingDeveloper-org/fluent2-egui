//! DataGrid: a virtualised table with a sticky header (`docs/specs/data-views.md` §1), with
//! sorting, resizing and autofit (§2) in `header.rs`, selection and keyboard focus (§3) in
//! `selection.rs`, and the AccessKit nodes (§4) in `a11y.rs`.
//!
//! Only the rows in view are laid out and passed to the cell closure.

mod a11y;
mod column;
mod copy;
mod header;
mod selection;
mod state;

use egui::{
    Align as EguiAlign, CornerRadius, EventFilter, Id, Key, Layout, Pos2, Rect, ScrollArea, Sense, Stroke,
    Ui, UiBuilder, Vec2,
};
use std::collections::HashMap;

mod menu;

pub use column::{Align, Column, ColumnWidth};
pub use copy::{rows_to_copy, tsv_field, tsv_rows};
pub use menu::{ColumnMenuResponse, column_menu};
pub use state::{ColumnLayout, DataGridState, GridFocus, Selection, SortDirection, SortSpec};

use crate::data::rows::{all_selected, visible_rows};
use crate::foundation::focus::paint_focus_ring;
use crate::theme::{Theme, c};
use crate::{CheckState, Checkbox};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};
use selection::Layout as Rows;

/// Identifies a row across sorts (D3). Selection and focus are stored as ids, never indices.
///
/// ```
/// let id = fluent2_egui::data::RowId(7);
/// assert_eq!(id.0, 7);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RowId(pub u64);

/// What a cell closure returns: the cell's focusable child, or nothing.
///
/// Return `()` when the cell has no interactive child. Return the child's [`egui::Response`]
/// (or `Some` of it) and Enter on a cell moves keyboard focus into that child; Escape returns
/// focus to the grid (§3.3). A response that is not focusable is ignored.
///
/// ```
/// # fn demo(ui: &mut egui::Ui) {
/// // A cell whose button takes focus on Enter: return the button's response.
/// let _ = ui.button("Open");
/// # }
/// ```
pub trait CellResponse {
    /// The widget Enter should focus, if any.
    #[doc(hidden)]
    fn focus_target(self) -> Option<Id>;
}

impl CellResponse for () {
    fn focus_target(self) -> Option<Id> {
        None
    }
}

impl CellResponse for egui::Response {
    fn focus_target(self) -> Option<Id> {
        self.sense.is_focusable().then_some(self.id)
    }
}

impl CellResponse for Option<egui::Response> {
    fn focus_target(self) -> Option<Id> {
        self.and_then(CellResponse::focus_target)
    }
}

/// Focusable cell children from the last frame, by row and column index.
type CellTargets = HashMap<(RowId, usize), Id>;

/// What the cell closure is told about the cell it draws.
#[derive(Clone, Copy, Debug)]
pub struct CellCtx {
    /// Row index into the app's (already sorted) data, `0..row_count`.
    pub row: usize,
    /// Id of the row (D3).
    pub row_id: RowId,
    /// Id of the column (D10).
    pub column: &'static str,
    /// Column index in display order.
    pub column_index: usize,
    /// The row is selected.
    pub selected: bool,
    /// This cell (cell focus) or its row (row focus) has keyboard focus.
    pub focused: bool,
}

/// Row height class of a DataGrid (§10).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum DataGridSize {
    /// 24 pt rows, `Caption1` text.
    ExtraSmall,
    /// 34 pt rows.
    Small,
    /// 44 pt rows.
    #[default]
    Medium,
}

/// Which rows can be selected (§3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum SelectionMode {
    /// Rows highlight on hover and nothing is selected.
    #[default]
    None,
    /// Selection follows focus; one row at a time.
    Single,
    /// Checkboxes, Shift and Ctrl selection.
    Multi,
}

/// What keyboard focus lands on (§3.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum FocusMode {
    /// The whole row is the focus target.
    #[default]
    Row,
    /// A single cell is the focus target.
    Cell,
}

/// What a DataGrid reports after `show`.
///
/// Sort and resize fields come from the header (WI-1582). Selection and activation fields come
/// from WI-1583.
#[derive(Clone, Debug)]
pub struct DataGridResponse {
    /// The whole grid. `has_focus` while the grid takes the keyboard.
    pub response: egui::Response,
    /// The user changed `state.sort` this frame (§2.1). Re-sort before the next frame.
    pub sort_changed: bool,
    /// The selection changed this frame.
    pub selection_changed: bool,
    /// A column was resized or autofitted this frame (§2.2, §2.3).
    pub column_resized: Option<&'static str>,
    /// Enter, or double-click on a row (or cell, in cell mode).
    pub activated: Option<CellCtx>,
    /// F2 on a row or cell. The grid does not edit.
    pub edit_requested: Option<CellCtx>,
    /// Secondary click or Shift+F10. Attach a context menu to this.
    pub context_menu: Option<CellCtx>,
    /// Secondary click or Shift+F10 on a header cell, by column id (§2.4). Open a menu with
    /// [`column_menu`] in it.
    pub header_context_menu: Option<&'static str>,
    /// The user dragged a header or pressed Alt+Left/Right on one, and `state.layout` holds the
    /// new column order (§2.4).
    pub columns_reordered: bool,
    /// Ctrl+C on the focused grid copied this text: the focused cell, or the selected rows (else
    /// the focused row) as TSV. Only set with [`DataGrid::cell_text`]. The grid has already passed
    /// it to the clipboard (issue #105).
    pub copied: Option<String>,
}

/// A virtualised DataGrid. Build it, then call [`DataGrid::show`].
///
/// ```
/// # use fluent2_egui::data::{DataGrid, DataGridState, SelectionMode};
/// # use fluent2_egui::data::data_grid::{Column, ColumnWidth};
/// # fn demo(ui: &mut egui::Ui) {
/// let columns = [
///     Column::new("name", "Name"),
///     Column::new("size", "Size").width(ColumnWidth::Fixed(96.0)),
/// ];
/// let mut state = DataGridState::default();
/// let _response = DataGrid::new("files", &columns, 100_000)
///     .label("Files")
///     .selection_mode(SelectionMode::Multi)
///     .show(ui, &mut state, |cell, ui| {
///         ui.label(format!("{} {}", cell.column, cell.row));
///     });
/// # }
/// ```
pub struct DataGrid<'a> {
    id: Id,
    columns: &'a [Column],
    row_count: usize,
    row_id: Option<Box<dyn Fn(usize) -> RowId + 'a>>,
    size: DataGridSize,
    selection_mode: SelectionMode,
    focus_mode: FocusMode,
    label: Option<String>,
    cell_text: Option<CellText<'a>>,
}

/// The accessible text of one cell, from its row index and column id.
type CellText<'a> = Box<dyn Fn(usize, &'static str) -> String + 'a>;

/// Reports gathered inside the scroll area and returned by [`DataGrid::show`].
#[derive(Default)]
struct Reports {
    selection_changed: bool,
    activated: Option<CellCtx>,
    edit_requested: Option<CellCtx>,
    context_menu: Option<CellCtx>,
    copied: Option<String>,
}

/// Where the grid's cells are, in content coordinates, for one frame.
struct Geometry<'a> {
    origin: Pos2,
    header_height: f32,
    row_height: f32,
    total_width: f32,
    widths: &'a [f32],
    /// Left edge of each data column, including the selection column when there is one.
    offsets: &'a [f32],
}

impl<'a> DataGrid<'a> {
    /// A grid with `row_count` rows and the given columns. `id_salt` identifies the grid's
    /// scroll state and must be unique among siblings (ARCHITECTURE rule 4).
    pub fn new(id_salt: impl std::hash::Hash, columns: &'a [Column], row_count: usize) -> Self {
        Self {
            id: salt_id(id_salt),
            columns,
            row_count,
            row_id: None,
            size: DataGridSize::Medium,
            selection_mode: SelectionMode::None,
            focus_mode: FocusMode::Row,
            label: None,
            cell_text: None,
        }
    }

    /// Stable ids for rows (D3). Default: the row index.
    #[must_use]
    pub fn row_id(mut self, f: impl Fn(usize) -> RowId + 'a) -> Self {
        self.row_id = Some(Box::new(f));
        self
    }

    /// Row height class. Default `DataGridSize::Medium`.
    #[must_use]
    pub fn size(mut self, size: DataGridSize) -> Self {
        self.size = size;
        self
    }

    /// Selection mode. Default `SelectionMode::None`.
    #[must_use]
    pub fn selection_mode(mut self, mode: SelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Focus mode. Default `FocusMode::Row`.
    #[must_use]
    pub fn focus_mode(mut self, mode: FocusMode) -> Self {
        self.focus_mode = mode;
        self
    }

    /// Accessible name of the grid.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Accessible text of each data cell, from its row index and column id (WI-1848).
    ///
    /// Screen readers read the focused cell's text, and a row reads its cells' text joined with
    /// ", ". AccessKit does not derive a cell's or row's name from the widgets drawn in it, so
    /// without this the grid is silent as focus moves. Usually the same string the cell draws.
    #[must_use]
    pub fn cell_text(mut self, f: impl Fn(usize, &'static str) -> String + 'a) -> Self {
        self.cell_text = Some(Box::new(f));
        self
    }

    /// Paint the grid. `cell` is called once per visible cell, with a child `Ui` clipped to it.
    ///
    /// Clicking a sortable header cell cycles its sort (§2.1) and sets `sort_changed`; the app
    /// sorts its data before the next frame. Dragging a column's right edge resizes it (§2.2);
    /// double-clicking that edge, or the first show of an [`ColumnWidth::Auto`] column, autofits
    /// it to the widest visible cell (§2.3). On a focused header cell, Enter or Space sorts,
    /// Ctrl+Left and Ctrl+Right resize by 8 pt, and Ctrl+Enter autofits (§3.3).
    ///
    /// With `row_count == 0` the grid shows only the header and never calls `cell`; the app
    /// draws its own empty state below it.
    pub fn show<R: CellResponse>(
        self,
        ui: &mut Ui,
        state: &mut DataGridState,
        mut cell: impl FnMut(CellCtx, &mut Ui) -> R,
    ) -> DataGridResponse {
        let theme = Theme::get(ui.ctx());
        let metrics = theme.data;
        let row_height = self.size.row_height(
            metrics.grid_row_height_medium,
            metrics.grid_row_height_small,
            metrics.grid_row_height_extra_small,
        );
        let header_height = metrics.grid_header_height;
        let multi = self.selection_mode == SelectionMode::Multi;
        let lead = if multi { metrics.selection_cell_width } else { 0.0 };
        let lead_columns = usize::from(multi);
        // Hidden columns drop out and the rest take the layout's order (§2.4), so everything
        // below, AccessKit indices included, works in display order.
        let all_columns = self.columns;
        let shown = state.layout.apply(all_columns);
        let columns: &[Column] = &shown;
        let widths = column::resolve_widths(columns, ui.available_width(), &state.widths);
        let offsets: Vec<f32> = column::prefix_offsets(&widths)
            .into_iter()
            .map(|offset| offset + lead)
            .collect();
        let total_width = lead + widths.iter().sum::<f32>();
        let content_height = header_height + self.row_count as f32 * row_height;
        let row_id = |row: usize| self.row_id.as_ref().map_or(RowId(row as u64), |f| f(row));
        let separator = (self.size != DataGridSize::ExtraSmall)
            .then(|| Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)));
        let hover = c(theme.color.subtle_background_hover);
        let selected_fill = c(theme.color.subtle_background_selected);
        let hover_ink = c(theme.color.neutral_foreground_1_hover);
        let selected_ink = c(theme.color.neutral_foreground_1_selected);
        let ring_radius = CornerRadius::same(border_radius::MEDIUM as u8);
        let grid_key = self.id.with("grid");
        let inside_key = grid_key.with("inside");
        let targets_key = grid_key.with("targets");
        let mut events = header::HeaderEvents::default();
        if let Some(id) = state.autofit_request.take() {
            events.autofit = columns.iter().position(|column| column.id == id);
        }

        let output = ScrollArea::both()
            .id_salt(self.id)
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let (content, _) =
                    ui.allocate_exact_size(Vec2::new(total_width, content_height), Sense::hover());
                let origin = content.min;
                let clip = ui.clip_rect();
                let body = Rect::from_min_max(Pos2::new(clip.min.x, clip.min.y + header_height), clip.max);
                let grid_response = ui.interact(clip, grid_key, Sense::click());
                // The focused cell, header cell or row, for the grid's active descendant: arrow
                // keys move focus inside the grid, and without it screen readers say nothing.
                let mut active: Option<Id> = None;

                let page_rows = ((viewport.height() - header_height) / row_height)
                    .floor()
                    .max(0.0);
                let visible_range = visible_rows(viewport, header_height, row_height, self.row_count);
                let rows = Rows {
                    row_count: self.row_count,
                    column_count: columns.len(),
                    mode: self.selection_mode,
                    focus_mode: self.focus_mode,
                    page: (page_rows as usize).saturating_sub(1).max(1),
                    row_id: &row_id,
                };
                selection::resolve_focus(state, &rows);

                let mut reports = Reports::default();
                let mut targets = CellTargets::new();
                let mut clicked = false;
                let mut activate_at = None;
                let mut context_at = None;
                let geo = Geometry {
                    origin,
                    header_height,
                    row_height,
                    total_width,
                    widths: &widths,
                    offsets: &offsets,
                };
                // The header strip stays at the top of the viewport and scrolls sideways.
                let header_rect = Rect::from_min_size(
                    Pos2::new(origin.x, origin.y + viewport.min.y),
                    Vec2::new(total_width, header_height),
                );
                let header_layout = header::HeaderLayout {
                    id: self.id,
                    rect: header_rect,
                    columns,
                    widths: &widths,
                    offsets: &offsets,
                    all_columns,
                };

                // Focus is inside a cell's child (Enter, §3.3). egui releases focus on Escape
                // before the grid runs, so Escape is checked first and does not need the focus.
                if let Some(child) = ui.ctx().data(|data| data.get_temp::<Id>(inside_key)) {
                    ui.ctx().data_mut(|data| data.remove::<Id>(inside_key));
                    if ui.input(|input| input.key_pressed(Key::Escape)) {
                        grid_response.request_focus();
                    } else if ui.memory(|memory| memory.focused()) == Some(child) {
                        ui.ctx().data_mut(|data| data.insert_temp(inside_key, child));
                    }
                }

                let mut header_menu = None;
                if grid_response.has_focus() {
                    // Arrow keys move focus in the grid, not to a neighbouring widget (§3.3).
                    let lock = EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    };
                    ui.memory_mut(|memory| memory.set_focus_lock_filter(grid_key, lock));
                    for (key, modifiers) in selection::pressed_keys(ui) {
                        let out = selection::handle_key(state, &rows, key, modifiers);
                        reports.selection_changed |= out.selection_changed;
                        if out.scroll_x != 0.0 {
                            ui.scroll_with_delta(Vec2::new(-out.scroll_x, 0.0));
                        }
                        if out.moved
                            && let Some(rect) = focus_rect(state, &geo)
                        {
                            ui.scroll_to_rect(rect, None);
                        }
                        if out.activate {
                            // In cell focus, a cell with a focusable child takes the focus (§3.3).
                            match focus_child(ui, targets_key, state) {
                                Some(child) => {
                                    ui.memory_mut(|memory| memory.request_focus(child));
                                    ui.ctx().data_mut(|data| data.insert_temp(inside_key, child));
                                }
                                None => reports.activated = focus_ctx(state, columns, &row_id),
                            }
                        }
                        if let Some(index) = state
                            .focus
                            .filter(|focus| focus.header)
                            .and_then(|focus| focus.column)
                        {
                            header_key(state, &header_layout, &mut events, index, &out);
                        }
                        if out.edit {
                            reports.edit_requested = focus_ctx(state, columns, &row_id);
                        }
                        if out.context_menu {
                            reports.context_menu = focus_ctx(state, columns, &row_id);
                            header_menu = header_focus_column(state, columns);
                        }
                    }
                }

                // Ctrl+C copies what the focus names (issue #105). Without `cell_text` there is no
                // text to copy, so the grid copies nothing.
                if grid_response.has_focus()
                    && let Some(text) = self.cell_text.as_deref().and_then(|cell_text| {
                        copy::copy_payload(state, self.columns, self.row_count, row_id, cell_text)
                    })
                    && ui.input(|input| {
                        input
                            .events
                            .iter()
                            .any(|event| matches!(event, egui::Event::Copy))
                    })
                {
                    ui.ctx().copy_text(text.clone());
                    reports.copied = Some(text);
                }

                for row in visible_range.clone() {
                    let top = origin.y + header_height + row as f32 * row_height;
                    let row_rect =
                        Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(total_width, row_height));
                    let id = row_id(row);
                    // The row's own response goes first, so the cells and checkbox drawn after
                    // it take clicks in their own rects.
                    let row_hit = ui.interact(
                        row_rect.intersect(body),
                        grid_key.with(("row", id.0)),
                        Sense::CLICK,
                    );
                    let column_at = row_hit
                        .interact_pointer_pos()
                        .map_or(0, |pos| column_at(pos.x - origin.x, &offsets));
                    if row_hit.clicked() {
                        clicked = true;
                        let modifiers = ui.input(|input| input.modifiers);
                        reports.selection_changed |=
                            selection::click(state, &rows, row, column_at, modifiers);
                    }
                    if row_hit.double_clicked() {
                        activate_at = Some((row, column_at));
                    }
                    if row_hit.secondary_clicked() {
                        clicked = true;
                        context_at = Some((row, column_at));
                    }

                    let selected = state.selection.selected.contains(&id);
                    // High contrast fills these rows with the highlight colour, so cell text
                    // takes the matching foreground.
                    let row_ink = if selected {
                        ui.painter().rect_filled(row_rect, 0.0, selected_fill);
                        Some(selected_ink)
                    } else if ui.rect_contains_pointer(row_rect) {
                        ui.painter().rect_filled(row_rect, 0.0, hover);
                        Some(hover_ink)
                    } else {
                        None
                    };
                    if let Some(stroke) = separator {
                        ui.painter().hline(row_rect.x_range(), row_rect.bottom(), stroke);
                    }
                    let row_selected = (self.selection_mode != SelectionMode::None).then_some(selected);
                    let row_label = self.cell_text.as_ref().map(|text| {
                        columns
                            .iter()
                            .map(|column| text(row, column.id))
                            .filter(|part| !part.is_empty())
                            .collect::<Vec<_>>()
                            .join(", ")
                    });
                    a11y::row(ui.ctx(), row_hit.id, row + 1, row_selected, row_label);
                    if state
                        .focus
                        .is_some_and(|focus| !focus.header && focus.row == id && focus.column.is_none())
                    {
                        active = Some(row_hit.id);
                    }

                    if multi {
                        let box_rect =
                            Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(lead, row_height));
                        let mut child = ui.new_child(
                            UiBuilder::new()
                                .max_rect(padded(box_rect))
                                .layout(Layout::left_to_right(EguiAlign::Center)),
                        );
                        child.set_clip_rect(box_rect.intersect(body));
                        let mut on = selected;
                        let check = child.add(Checkbox::bool(&mut on, "").not_a_tab_stop());
                        ui.ctx().accesskit_node_builder(check.id, |node| {
                            node.set_label(format!("Select row {}", row + 1));
                        });
                        if check.clicked() {
                            clicked = true;
                            reports.selection_changed |= selection::row_checkbox(state, &rows, row);
                        }
                    }

                    for (index, column) in columns.iter().enumerate() {
                        let cell_rect = Rect::from_min_size(
                            Pos2::new(origin.x + offsets[index], top),
                            Vec2::new(widths[index], row_height),
                        );
                        let cell_hit = ui.interact(
                            cell_rect.intersect(body),
                            grid_key.with(("cell", id.0, index)),
                            Sense::hover(),
                        );
                        let cell_label = self.cell_text.as_ref().map(|text| text(row, column.id));
                        a11y::cell(ui.ctx(), cell_hit.id, row + 1, index + lead_columns, cell_label);
                        if state.focus.is_some_and(|focus| {
                            !focus.header && focus.row == id && focus.column == Some(index)
                        }) {
                            active = Some(cell_hit.id);
                        }
                        let focused = state.focus.is_some_and(|focus| {
                            !focus.header
                                && focus.row == id
                                && focus.column.is_none_or(|column_index| column_index == index)
                        });
                        let ctx = CellCtx {
                            row,
                            row_id: id,
                            column: column.id,
                            column_index: index,
                            selected,
                            focused,
                        };
                        // Hang the cell's content under its GridCell node, so a screen reader
                        // reads the text when focus lands on the cell (WI-1586 Orca walk).
                        let mut child = ui.new_child(
                            UiBuilder::new()
                                .max_rect(padded(cell_rect))
                                .layout(column.cell_layout())
                                .accessibility_parent(cell_hit.id),
                        );
                        child.set_clip_rect(cell_rect);
                        // A selectable label would take the click meant for the row (§3).
                        child.style_mut().interaction.selectable_labels = false;
                        if let Some(ink) = row_ink {
                            child.visuals_mut().override_text_color = Some(ink);
                        }
                        if let Some(target) = cell(ctx, &mut child).focus_target() {
                            targets.insert((id, index), target);
                        }
                    }
                }

                ui.ctx().data_mut(|data| data.insert_temp(targets_key, targets));

                // Painted after the rows so the header stays over the rows scrolled under it.
                let header_events = header::show(ui, &header_layout, state, &theme);
                events.sort_changed |= header_events.sort_changed;
                events.resized = header_events.resized.or(events.resized);
                events.autofit = header_events.autofit.or(events.autofit);
                events.reordered |= header_events.reordered;
                events.context_menu = header_events.context_menu.or(header_menu);
                if multi {
                    let box_rect = Rect::from_min_size(header_rect.min, Vec2::new(lead, header_height));
                    let mut child = ui.new_child(
                        UiBuilder::new()
                            .max_rect(padded(box_rect))
                            .layout(Layout::left_to_right(EguiAlign::Center)),
                    );
                    child.set_clip_rect(box_rect);
                    let mut check = if all_selected(&state.selection, self.row_count, &row_id) {
                        CheckState::Checked
                    } else if state.selection.selected.is_empty() {
                        CheckState::Unchecked
                    } else {
                        CheckState::Mixed
                    };
                    let response = child.add(Checkbox::new(&mut check, "").not_a_tab_stop());
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_label("Select all rows");
                    });
                    if response.clicked() {
                        clicked = true;
                        reports.selection_changed |= selection::header_checkbox(state, &rows);
                    }
                }
                let header_row = ui.interact(
                    header_rect.intersect(clip),
                    grid_key.with("header-row"),
                    Sense::hover(),
                );
                a11y::row(ui.ctx(), header_row.id, 0, None, None);
                for (index, column) in columns.iter().enumerate() {
                    // Every header cell is interacted by `header::show`; its node is named here.
                    let id = header::cell_id(self.id, column.id);
                    a11y::header_cell(ui.ctx(), id, column.header.text(), index + lead_columns);
                    if state
                        .focus
                        .is_some_and(|focus| focus.header && focus.column == Some(index))
                    {
                        active = Some(id);
                    }
                }

                // Autofit (§2.3): a double-clicked handle or a header key, and an Auto column on
                // its first show. Cells are measured in a sizing pass over the visible rows, so
                // the new width reaches the rows on the next frame.
                for (index, column) in columns.iter().enumerate() {
                    let first_show =
                        matches!(column.width, ColumnWidth::Auto) && !state.widths.contains_key(column.id);
                    if events.autofit != Some(index) && !first_show {
                        continue;
                    }
                    let mut widest = 0.0_f32;
                    for row in visible_range.clone() {
                        let id = row_id(row);
                        let ctx = cell_ctx(state, row, id, index, column);
                        let width = header::natural_width(
                            ui,
                            row_height,
                            Layout::left_to_right(EguiAlign::Center),
                            |child| {
                                cell(ctx, child);
                            },
                        );
                        widest = widest.max(width);
                    }
                    let header_width = header::header_width(ui, column, header_height);
                    let width = header::autofit_width(
                        header_width,
                        widest,
                        spacing_horizontal::S,
                        column.min,
                        column.max,
                    );
                    state.widths.insert(column.id.to_owned(), width);
                    events.resized = Some(column.id);
                }

                if grid_response.has_focus()
                    && let Some(rect) = focus_rect(state, &geo)
                {
                    paint_focus_ring(ui, rect.shrink(2.0), ring_radius);
                }

                if clicked {
                    grid_response.request_focus();
                }
                if let Some((row, column)) = activate_at {
                    reports.activated = cell_report(state, columns, &row_id, row, column);
                }
                if let Some((row, column)) = context_at {
                    reports.context_menu = cell_report(state, columns, &row_id, row, column);
                }
                a11y::grid(
                    ui.ctx(),
                    grid_response.id,
                    self.label.as_deref(),
                    self.row_count,
                    columns.len() + lead_columns,
                    multi,
                    active,
                );
                (grid_response, reports)
            });

        let (response, reports) = output.inner;
        DataGridResponse {
            response,
            sort_changed: events.sort_changed,
            selection_changed: reports.selection_changed,
            column_resized: events.resized,
            activated: reports.activated,
            edit_requested: reports.edit_requested,
            context_menu: reports.context_menu,
            header_context_menu: events.context_menu,
            columns_reordered: events.reordered,
            copied: reports.copied,
        }
    }
}

/// The column of the focused header cell, or `None` when focus is not on the header.
fn header_focus_column(state: &DataGridState, columns: &[Column]) -> Option<&'static str> {
    let focus = state.focus.filter(|focus| focus.header)?;
    columns.get(focus.column?).map(|column| column.id)
}

/// The rectangle of the focused row, cell or header cell, in screen coordinates, or `None`
/// before the first focus.
fn focus_rect(state: &DataGridState, geo: &Geometry) -> Option<Rect> {
    let focus = state.focus?;
    let column = focus.column.unwrap_or(0).min(geo.offsets.len().saturating_sub(1));
    if focus.header {
        return Some(Rect::from_min_size(
            Pos2::new(geo.origin.x + geo.offsets.get(column).copied()?, geo.origin.y),
            Vec2::new(geo.widths.get(column).copied()?, geo.header_height),
        ));
    }
    let top = geo.origin.y + geo.header_height + focus.row_index_hint as f32 * geo.row_height;
    match focus.column {
        Some(column) => Some(Rect::from_min_size(
            Pos2::new(geo.origin.x + geo.offsets.get(column).copied()?, top),
            Vec2::new(geo.widths.get(column).copied()?, geo.row_height),
        )),
        None => Some(Rect::from_min_size(
            Pos2::new(geo.origin.x, top),
            Vec2::new(geo.total_width, geo.row_height),
        )),
    }
}

/// The cell report for the focused row and column. `None` on the header row.
fn focus_ctx(state: &DataGridState, columns: &[Column], row_id: &dyn Fn(usize) -> RowId) -> Option<CellCtx> {
    let focus = state.focus?;
    if focus.header {
        return None;
    }
    cell_report(
        state,
        columns,
        row_id,
        focus.row_index_hint,
        focus.column.unwrap_or(0),
    )
}

/// The cell report for `row` and `column` (indices). `None` when there are no columns.
fn cell_report(
    state: &DataGridState,
    columns: &[Column],
    row_id: &dyn Fn(usize) -> RowId,
    row: usize,
    column: usize,
) -> Option<CellCtx> {
    let column_index = column.min(columns.len().checked_sub(1)?);
    let id = row_id(row);
    let focused = state.focus.is_some_and(|focus| {
        !focus.header && focus.row == id && focus.column.is_none_or(|focused| focused == column_index)
    });
    Some(CellCtx {
        row,
        row_id: id,
        column: columns.get(column_index)?.id,
        column_index,
        selected: state.selection.selected.contains(&id),
        focused,
    })
}

/// The focusable child of the focused cell, as drawn last frame. `None` in row focus and on the
/// header.
fn focus_child(ui: &Ui, targets_key: Id, state: &DataGridState) -> Option<Id> {
    let focus = state.focus?;
    if focus.header || focus.column.is_none() {
        return None;
    }
    let targets = ui.ctx().data(|data| data.get_temp::<CellTargets>(targets_key))?;
    targets.get(&(focus.row, focus.column?)).copied()
}

/// What the cell closure is told about one cell.
fn cell_ctx(state: &DataGridState, row: usize, row_id: RowId, index: usize, column: &Column) -> CellCtx {
    let focused = state.focus.is_some_and(|focus| {
        !focus.header && focus.row == row_id && focus.column.is_none_or(|column_index| column_index == index)
    });
    CellCtx {
        row,
        row_id,
        column: column.id,
        column_index: index,
        selected: state.selection.selected.contains(&row_id),
        focused,
    }
}

/// Apply a key on the focused header cell `index` (§2.1, §2.2, §2.3, §3.3): sort, resize by
/// the key's step, or ask for an autofit. Columns that cannot do the thing ignore it.
fn header_key(
    state: &mut DataGridState,
    layout: &header::HeaderLayout,
    events: &mut header::HeaderEvents,
    index: usize,
    out: &selection::KeyOutcome,
) {
    let Some(column) = layout.columns.get(index) else {
        return;
    };
    if out.sort && column.sortable {
        state.sort = header::next_sort(state.sort.as_ref(), column.id);
        events.sort_changed = true;
    }
    if out.resize != 0.0 && column.resizable {
        let Some(&current) = layout.widths.get(index) else {
            return;
        };
        header::freeze_fractions_right_of(index, layout, state);
        let width = header::clamp(current + out.resize, column.min, column.max);
        state.widths.insert(column.id.to_owned(), width);
        events.resized = Some(column.id);
    }
    if out.autofit && column.resizable {
        events.autofit = Some(index);
    }
    if out.reorder != 0 {
        // Alt+Left goes before the left neighbour; Alt+Right before the one after the right
        // neighbour, or last (§2.4). Focus follows the column.
        let target = index
            .checked_add_signed(out.reorder)
            .filter(|&target| target < layout.columns.len());
        if let Some(target) = target {
            let before = layout
                .columns
                .get(if out.reorder < 0 { target } else { target + 1 })
                .map(|other| other.id);
            if state.layout.move_before(layout.all_columns, column.id, before) {
                events.reordered = true;
                if let Some(focus) = state.focus.as_mut() {
                    focus.column = Some(target);
                }
            }
        }
    }
}

/// The data column under `x` (relative to the grid's left edge). Clicks in the selection column
/// count as the first column.
fn column_at(x: f32, offsets: &[f32]) -> usize {
    offsets.iter().rposition(|&offset| x >= offset).unwrap_or(0)
}

impl DataGridSize {
    fn row_height(self, medium: f32, small: f32, extra_small: f32) -> f32 {
        match self {
            Self::Medium => medium,
            Self::Small => small,
            Self::ExtraSmall => extra_small,
        }
    }
}

/// Scroll-state id from a caller's salt. Hashed to a `u64` so any `Hash` value works.
fn salt_id(salt: impl std::hash::Hash) -> Id {
    use std::hash::Hasher;
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    salt.hash(&mut hasher);
    Id::new(hasher.finish())
}

/// A cell's content rect: the cell less `spacingHorizontalS` on each side.
fn padded(rect: Rect) -> Rect {
    let pad = spacing_horizontal::S.min(rect.width() / 2.0).max(0.0);
    rect.shrink2(Vec2::new(pad, 0.0))
}
