//! DataGrid state: plain data the app owns and passes to `DataGrid::show` (§1.3).

use std::collections::{BTreeMap, BTreeSet};

use super::RowId;
use super::column::Column;

/// State of a DataGrid. Store it in the app and pass `&mut` to every `show`.
///
/// ```
/// let state = fluent2_egui::data::data_grid::DataGridState::default();
/// assert!(state.sort.is_none());
/// ```
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DataGridState {
    /// `None` means unsorted.
    pub sort: Option<SortSpec>,
    /// User-resized or autofitted widths by column id. Owned strings so serde works.
    pub widths: BTreeMap<String, f32>,
    /// Selected rows, by id.
    pub selection: Selection,
    /// Focused row and, in cell focus mode, the column index. `None` before first focus.
    pub focus: Option<GridFocus>,
    /// Column order and hidden columns (§2.4). Persist it with `widths`.
    #[cfg_attr(feature = "serde", serde(default))]
    pub layout: ColumnLayout,
    /// Id of a column to autofit on the next `show` (§2.4), as [`super::column_menu`]'s
    /// "Resize to fit" asks. The grid clears it. Not persisted.
    #[cfg_attr(feature = "serde", serde(skip))]
    pub autofit_request: Option<String>,
}

/// Which columns show, and in what order (§2.4). Plain data the app persists with the widths.
///
/// `order` lists column ids in display order. Columns missing from it keep their definition
/// order after the listed ones, so a column added by a new app version still shows. Ids that
/// no longer name a column are ignored.
///
/// ```
/// use fluent2_egui::data::data_grid::{Column, ColumnLayout};
/// let columns = [Column::new("name", "Name"), Column::new("size", "Size")];
/// let mut layout = ColumnLayout::default();
/// layout.move_column(&columns, "size", 0);
/// layout.set_hidden("name", true);
/// let shown: Vec<_> = layout.apply(&columns).iter().map(|c| c.id()).collect();
/// assert_eq!(shown, ["size"]);
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ColumnLayout {
    /// Column ids in display order. Empty means definition order.
    pub order: Vec<String>,
    /// Ids of hidden columns. A [`Column::required`] column is shown even when listed here.
    pub hidden: BTreeSet<String>,
}

impl ColumnLayout {
    /// The columns to show, in display order: hidden columns left out, the rest ordered by
    /// `order` and then by definition.
    #[must_use]
    pub fn apply(&self, columns: &[Column]) -> Vec<Column> {
        self.ordered(columns)
            .into_iter()
            .filter(|column| column.required || !self.hidden.contains(column.id))
            .cloned()
            .collect()
    }

    /// Every column, hidden ones included, in display order.
    #[must_use]
    pub fn ordered<'c>(&self, columns: &'c [Column]) -> Vec<&'c Column> {
        let mut sorted: Vec<(usize, &Column)> = columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let rank = self
                    .order
                    .iter()
                    .position(|id| id == column.id)
                    .unwrap_or(self.order.len() + index);
                (rank, column)
            })
            .collect();
        sorted.sort_by_key(|(rank, _)| *rank);
        sorted.into_iter().map(|(_, column)| column).collect()
    }

    /// Whether the column `id` is hidden.
    #[must_use]
    pub fn is_hidden(&self, id: &str) -> bool {
        self.hidden.contains(id)
    }

    /// Hide or show the column `id`. Returns whether anything changed.
    pub fn set_hidden(&mut self, id: &str, hidden: bool) -> bool {
        if hidden {
            self.hidden.insert(id.to_owned())
        } else {
            self.hidden.remove(id)
        }
    }

    /// Move the column `id` to position `to` among all columns (hidden ones included), in
    /// display order. `to` past the end moves it last. Writes the full order to `order`.
    /// Returns whether the order changed.
    pub fn move_column(&mut self, columns: &[Column], id: &str, to: usize) -> bool {
        let mut ids: Vec<&'static str> = self.ordered(columns).iter().map(|column| column.id).collect();
        let Some(from) = ids.iter().position(|other| *other == id) else {
            return false;
        };
        let moved = ids.remove(from);
        let to = to.min(ids.len());
        ids.insert(to, moved);
        let order: Vec<String> = ids.into_iter().map(str::to_owned).collect();
        let changed = from != to;
        self.order = order;
        changed
    }

    /// Move the column `id` just before the column `before` in display order, or last when
    /// `before` is `None` or names no column. Returns whether the order changed.
    pub(super) fn move_before(&mut self, columns: &[Column], id: &str, before: Option<&str>) -> bool {
        let mut ids: Vec<&'static str> = self.ordered(columns).iter().map(|column| column.id).collect();
        let original = ids.clone();
        let Some(from) = ids.iter().position(|other| *other == id) else {
            return false;
        };
        let moved = ids.remove(from);
        let to = before
            .and_then(|before| ids.iter().position(|other| *other == before))
            .unwrap_or(ids.len());
        ids.insert(to, moved);
        if ids == original {
            return false;
        }
        self.order = ids.into_iter().map(str::to_owned).collect();
        true
    }

    /// Back to definition order with every column shown.
    pub fn reset(&mut self) {
        self.order.clear();
        self.hidden.clear();
    }
}

/// The sorted column and its direction.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SortSpec {
    /// Column id.
    pub column: String,
    /// Sort direction.
    pub direction: SortDirection,
}

/// Direction of a sort.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum SortDirection {
    /// Smallest first.
    Ascending,
    /// Largest first.
    Descending,
}

/// Selected rows and the anchor for Shift ranges.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Selection {
    /// Selected row ids.
    pub selected: BTreeSet<RowId>,
    /// Anchor for Shift ranges.
    pub anchor: Option<RowId>,
}

/// Where keyboard focus is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct GridFocus {
    /// Focused row.
    pub row: RowId,
    /// Last known index of `row`, used when `row` is no longer found (§3.4).
    pub row_index_hint: usize,
    /// Column index in cell focus. `None` in row focus.
    pub column: Option<usize>,
    /// Focus is on the header row. The header cell is then `column`.
    pub header: bool,
}
