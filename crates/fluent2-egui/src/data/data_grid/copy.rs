//! Clipboard text for the DataGrid (issue #105). Ctrl+C copies the focused cell, or the selected
//! rows as TSV, and the app's context menu uses the same functions, so both give the same text.
//!
//! The functions only build strings. The grid and the app pass them to `Context::copy_text`,
//! which uses the browser Clipboard API through eframe on wasm.

use super::{Column, DataGridState, RowId};

/// One cell's text as a TSV field. A field that holds a tab, a line break or a double quote is
/// wrapped in double quotes, with each quote doubled, so a spreadsheet keeps it in one cell.
///
/// ```
/// use fluent2_egui::data::data_grid::tsv_field;
/// assert_eq!(tsv_field("plain"), "plain");
/// assert_eq!(tsv_field("a\tb"), "\"a\tb\"");
/// assert_eq!(tsv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
/// ```
#[must_use]
pub fn tsv_field(text: &str) -> String {
    if text.contains(['\t', '\n', '\r', '"']) {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text.to_owned()
    }
}

/// The rows at `rows` (data indices) as TSV: each row's cells in column order, tab-separated,
/// rows joined with `\n`. There is no header line and no trailing newline.
///
/// ```
/// use fluent2_egui::data::data_grid::{Column, tsv_rows};
/// let columns = [Column::new("name", "Name"), Column::new("size", "Size")];
/// let text = tsv_rows(&columns, &[0, 2], |row, column| format!("{column}{row}"));
/// assert_eq!(text, "name0\tsize0\nname2\tsize2");
/// ```
#[must_use]
pub fn tsv_rows(
    columns: &[Column],
    rows: &[usize],
    cell_text: impl Fn(usize, &'static str) -> String,
) -> String {
    rows.iter()
        .map(|&row| {
            columns
                .iter()
                .map(|column| tsv_field(&cell_text(row, column.id)))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The data indices of the rows a copy takes: the selected rows in display order. With no
/// selection, the focused row (in row focus, the header excluded). Empty when neither applies.
///
/// `row_id` maps a data index to its [`RowId`], as in [`super::DataGrid::row_id`].
///
/// ```
/// use fluent2_egui::data::data_grid::{DataGridState, RowId, rows_to_copy};
/// let state = DataGridState::default();
/// assert!(rows_to_copy(&state, 10, |row| RowId(row as u64)).is_empty());
/// ```
#[must_use]
pub fn rows_to_copy(state: &DataGridState, row_count: usize, row_id: impl Fn(usize) -> RowId) -> Vec<usize> {
    let selected: Vec<usize> = (0..row_count)
        .filter(|&row| state.selection.selected.contains(&row_id(row)))
        .collect();
    if !selected.is_empty() {
        return selected;
    }
    match state.focus {
        Some(focus) if !focus.header && focus.row_index_hint < row_count => vec![focus.row_index_hint],
        _ => Vec::new(),
    }
}

/// What Ctrl+C copies from the grid's focus. A focused cell gives its text. A focused row gives
/// the rows from [`rows_to_copy`] as TSV. The header and an empty grid give `None`.
pub(super) fn copy_payload(
    state: &DataGridState,
    columns: &[Column],
    row_count: usize,
    row_id: impl Fn(usize) -> RowId,
    cell_text: &dyn Fn(usize, &'static str) -> String,
) -> Option<String> {
    let focus = state.focus?;
    if focus.header {
        return None;
    }
    match focus.column {
        Some(index) => {
            let column = columns.get(index)?;
            (focus.row_index_hint < row_count).then(|| cell_text(focus.row_index_hint, column.id))
        }
        None => {
            let rows = rows_to_copy(state, row_count, row_id);
            (!rows.is_empty()).then(|| tsv_rows(columns, &rows, cell_text))
        }
    }
}
