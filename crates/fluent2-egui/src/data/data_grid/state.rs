//! DataGrid state: plain data the app owns and passes to `DataGrid::show` (§1.3).

use std::collections::{BTreeMap, BTreeSet};

use super::RowId;

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
