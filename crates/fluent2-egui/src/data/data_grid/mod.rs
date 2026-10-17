//! DataGrid: a virtualised table with a sticky header (`docs/specs/data-views.md` §1), with
//! sorting, resizing and autofit (§2).
//!
//! Only the rows in view are laid out and passed to the cell closure. Selection and keyboard
//! handling are later items (§3 and §4).

mod column;
mod header;
mod state;

use egui::{Id, Pos2, Rect, ScrollArea, Sense, Stroke, Ui, UiBuilder, Vec2};

pub use column::{Align, Column, ColumnWidth};
pub use state::{DataGridState, GridFocus, Selection, SortDirection, SortSpec};

use crate::data::rows::visible_rows;
use crate::theme::{Theme, c};
use fluent2_tokens::generated::global::{spacing_horizontal, stroke_width};

/// Identifies a row across sorts (D3). Selection and focus are stored as ids, never indices.
///
/// ```
/// let id = fluent2_egui::data::RowId(7);
/// assert_eq!(id.0, 7);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct RowId(pub u64);

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
/// Selection and keyboard fields are filled in by later items; this item leaves them unset.
#[derive(Clone, Debug)]
pub struct DataGridResponse {
    /// The whole grid.
    pub response: egui::Response,
    /// The user changed `state.sort` this frame (§2.1). Re-sort before the next frame.
    pub sort_changed: bool,
    /// The selection changed this frame.
    pub selection_changed: bool,
    /// A column was resized or autofitted this frame (§2.2, §2.3).
    pub column_resized: Option<&'static str>,
    /// Enter or double-click on a row (or cell, in cell mode).
    pub activated: Option<CellCtx>,
    /// F2 on a row or cell. The grid does not edit.
    pub edit_requested: Option<CellCtx>,
    /// Secondary click or Shift+F10 or Menu. Attach a context menu to this.
    pub context_menu: Option<CellCtx>,
}

/// A virtualised DataGrid. Build it, then call [`DataGrid::show`].
///
/// ```
/// # use fluent2_egui::data::{DataGrid, DataGridState};
/// # use fluent2_egui::data::data_grid::{Column, ColumnWidth};
/// # fn demo(ui: &mut egui::Ui) {
/// let columns = [
///     Column::new("name", "Name"),
///     Column::new("size", "Size").width(ColumnWidth::Fixed(96.0)),
/// ];
/// let mut state = DataGridState::default();
/// let _response = DataGrid::new("files", &columns, 100_000).label("Files").show(ui, &mut state, |cell, ui| {
///     ui.label(format!("{} {}", cell.column, cell.row));
/// });
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

    /// Paint the grid. `cell` is called once per visible cell, with a child `Ui` clipped to it.
    ///
    /// Clicking a sortable header cell cycles its sort (§2.1) and sets `sort_changed`; the app
    /// sorts its data before the next frame. Dragging a column's right edge resizes it (§2.2);
    /// double-clicking that edge, or the first show of an [`ColumnWidth::Auto`] column, autofits
    /// it to the widest visible cell (§2.3).
    ///
    /// With `row_count == 0` the grid shows only the header and never calls `cell`; the app
    /// draws its own empty state below it.
    pub fn show(
        self,
        ui: &mut Ui,
        state: &mut DataGridState,
        mut cell: impl FnMut(CellCtx, &mut Ui),
    ) -> DataGridResponse {
        let theme = Theme::get(ui.ctx());
        let metrics = theme.data;
        let row_height = self.size.row_height(
            metrics.grid_row_height_medium,
            metrics.grid_row_height_small,
            metrics.grid_row_height_extra_small,
        );
        let header_height = metrics.grid_header_height;
        let widths = column::resolve_widths(self.columns, ui.available_width(), &state.widths);
        let offsets = column::prefix_offsets(&widths);
        let total_width: f32 = widths.iter().sum();
        let content_height = header_height + self.row_count as f32 * row_height;
        let row_id = |row: usize| self.row_id.as_ref().map_or(RowId(row as u64), |f| f(row));
        let separator = (self.size != DataGridSize::ExtraSmall)
            .then(|| Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)));
        let hover = c(theme.color.subtle_background_hover);
        let mut events = header::HeaderEvents::default();

        let output = ScrollArea::both()
            .id_salt(self.id)
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let (content, response) =
                    ui.allocate_exact_size(Vec2::new(total_width, content_height), Sense::hover());
                let origin = content.min;
                let rows = visible_rows(viewport, header_height, row_height, self.row_count);

                for row in rows.clone() {
                    let top = origin.y + header_height + row as f32 * row_height;
                    let row_rect =
                        Rect::from_min_size(Pos2::new(origin.x, top), Vec2::new(total_width, row_height));
                    if ui.rect_contains_pointer(row_rect) {
                        ui.painter().rect_filled(row_rect, 0.0, hover);
                    }
                    if let Some(stroke) = separator {
                        ui.painter().hline(row_rect.x_range(), row_rect.bottom(), stroke);
                    }
                    let id = row_id(row);
                    for (index, column) in self.columns.iter().enumerate() {
                        let cell_rect = Rect::from_min_size(
                            Pos2::new(origin.x + offsets[index], top),
                            Vec2::new(widths[index], row_height),
                        );
                        let ctx = cell_ctx(state, row, id, index, column);
                        let mut child = ui.new_child(
                            UiBuilder::new()
                                .max_rect(padded(cell_rect))
                                .layout(column.cell_layout()),
                        );
                        child.set_clip_rect(cell_rect);
                        cell(ctx, &mut child);
                    }
                }

                // Painted last so it stays over the rows scrolled under it.
                let header_rect = Rect::from_min_size(
                    Pos2::new(origin.x, origin.y + viewport.min.y),
                    Vec2::new(total_width, header_height),
                );
                let layout = header::HeaderLayout {
                    id: self.id,
                    rect: header_rect,
                    columns: self.columns,
                    widths: &widths,
                    offsets: &offsets,
                };
                events = header::show(ui, &layout, state, &theme);

                // Autofit (§2.3): a double-clicked handle, and an Auto column on its first show.
                // Cells are measured in a sizing pass over the visible rows, so the new width
                // reaches the rows on the next frame.
                for (index, column) in self.columns.iter().enumerate() {
                    let first_show =
                        matches!(column.width, ColumnWidth::Auto) && !state.widths.contains_key(column.id);
                    if events.autofit != Some(index) && !first_show {
                        continue;
                    }
                    let mut widest = 0.0_f32;
                    for row in rows.clone() {
                        let id = row_id(row);
                        let ctx = cell_ctx(state, row, id, index, column);
                        let width = header::natural_width(
                            ui,
                            row_height,
                            egui::Layout::left_to_right(egui::Align::Center),
                            |child| cell(ctx, child),
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
                response
            });

        DataGridResponse {
            response: output.inner,
            sort_changed: events.sort_changed,
            selection_changed: false,
            column_resized: events.resized,
            activated: None,
            edit_requested: None,
            context_menu: None,
        }
    }
}

/// What the cell closure is told about one cell.
fn cell_ctx(state: &DataGridState, row: usize, row_id: RowId, index: usize, column: &Column) -> CellCtx {
    let focused = state.focus.is_some_and(|focus| {
        focus.row == row_id && focus.column.is_none_or(|column_index| column_index == index)
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
