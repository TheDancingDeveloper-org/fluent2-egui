//! Viewport-to-row maths and selection maths shared by the data views (`docs/specs/data-views.md`
//! D7). Selection works on row indices and converts to [`RowId`] at the boundary.

use std::ops::Range;

use egui::Rect;

use super::RowId;
use super::data_grid::{GridFocus, Selection};

/// Rows whose band intersects `viewport`, as `first..end`, clamped to `0..row_count`.
///
/// Row `r` occupies `header_height + r * row_height ..` in content coordinates, and the sticky
/// header covers the top `header_height` of the viewport. The range may include a row that is
/// hidden under the header, but it never leaves out a visible one. Non-finite or non-positive
/// geometry is clamped (ARCHITECTURE rule 9).
#[must_use]
pub(crate) fn visible_rows(
    viewport: Rect,
    header_height: f32,
    row_height: f32,
    row_count: usize,
) -> Range<usize> {
    let row_height = finite_or(row_height, 1.0).max(1.0);
    let header_height = finite_or(header_height, 0.0).max(0.0);
    let top = (finite_or(viewport.min.y, 0.0) - header_height).max(0.0);
    let bottom = finite_or(viewport.max.y, 0.0) - header_height;
    let first = ((top / row_height).floor() as usize).min(row_count);
    let end = ((bottom / row_height).ceil().max(0.0) as usize).clamp(first, row_count);
    first..end
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

/// Index of the row whose id is `id`, checking `hint` first (§3.4). `None` if no row has it.
#[must_use]
pub(crate) fn find_row(
    hint: usize,
    row_count: usize,
    row_id: &dyn Fn(usize) -> RowId,
    id: RowId,
) -> Option<usize> {
    if hint < row_count && row_id(hint) == id {
        return Some(hint);
    }
    (0..row_count).find(|&row| row_id(row) == id)
}

/// Focus position after a vertical move of `step` rows (negative is up).
///
/// The header is position 0 and data row `r` is position `r + 1`, so `None` is the header.
/// The move clamps to the header and the last row.
#[must_use]
pub(crate) fn step_row(current: Option<usize>, step: isize, row_count: usize) -> Option<usize> {
    let from = current.map_or(0, |row| row + 1) as isize;
    let to = (from + step).clamp(0, row_count as isize) as usize;
    to.checked_sub(1)
}

/// Move a column index by `step`, clamped to `0..column_count`.
#[must_use]
pub(crate) fn step_column(column: usize, step: isize, column_count: usize) -> usize {
    if column_count == 0 {
        return 0;
    }
    (column as isize + step).clamp(0, column_count as isize - 1) as usize
}

/// Make `row` the only selected row and the anchor (plain click, Single mode).
pub(crate) fn select_only(selection: &mut Selection, row: RowId) {
    selection.selected.clear();
    selection.selected.insert(row);
    selection.anchor = Some(row);
}

/// Toggle `row` and make it the anchor (Ctrl+click, row checkbox, Space).
pub(crate) fn toggle(selection: &mut Selection, row: RowId) {
    if !selection.selected.remove(&row) {
        selection.selected.insert(row);
    }
    selection.anchor = Some(row);
}

/// Select the rows between `anchor` and `to` (indices, inclusive). `additive` keeps the current
/// selection (Ctrl+Shift+click); otherwise the range replaces it (Shift+click). The anchor is
/// left alone, so a second Shift move extends from the same row.
pub(crate) fn select_range(
    selection: &mut Selection,
    anchor: usize,
    to: usize,
    row_count: usize,
    row_id: &dyn Fn(usize) -> RowId,
    additive: bool,
) {
    if !additive {
        selection.selected.clear();
    }
    if row_count == 0 {
        return;
    }
    let (low, high) = if anchor <= to { (anchor, to) } else { (to, anchor) };
    for row in low..=high.min(row_count - 1) {
        selection.selected.insert(row_id(row));
    }
}

/// Select every row (header checkbox, Ctrl+A). O(`row_count`), see §3.1.
pub(crate) fn select_all(selection: &mut Selection, row_count: usize, row_id: &dyn Fn(usize) -> RowId) {
    selection.selected = (0..row_count).map(row_id).collect();
}

/// Whether every row is selected. Checks the count first so it is O(1) unless all are.
#[must_use]
pub(crate) fn all_selected(selection: &Selection, row_count: usize, row_id: &dyn Fn(usize) -> RowId) -> bool {
    row_count > 0
        && selection.selected.len() >= row_count
        && (0..row_count).all(|row| selection.selected.contains(&row_id(row)))
}

/// Focus on `row` (index) and `column`, with the hint kept current for §3.4.
pub(crate) fn focus_row(
    focus: &mut Option<GridFocus>,
    row: usize,
    row_id: RowId,
    column: Option<usize>,
    header: bool,
) {
    *focus = Some(GridFocus {
        row: row_id,
        row_index_hint: row,
        column,
        header,
    });
}

#[cfg(test)]
mod tests {
    use egui::{Pos2, Rect};

    use super::{
        all_selected, find_row, select_all, select_only, select_range, step_column, step_row, toggle,
        visible_rows,
    };
    use crate::data::RowId;
    use crate::data::data_grid::Selection;

    fn viewport(top: f32, bottom: f32) -> Rect {
        Rect::from_min_max(Pos2::new(0.0, top), Pos2::new(100.0, bottom))
    }

    fn index(row: usize) -> RowId {
        RowId(row as u64)
    }

    #[test]
    fn top_of_content_starts_at_row_zero() {
        // Header 32, rows 44: the viewport shows the header and rows 0 to 14.
        let rows = visible_rows(viewport(0.0, 700.0), 32.0, 44.0, 100_000);
        assert_eq!(rows.start, 0);
        assert!(rows.end >= 16 && rows.end <= 17, "end was {}", rows.end);
    }

    #[test]
    fn partly_visible_first_row_is_included() {
        // Row 0 spans content y 32..76, so it is partly visible at scroll offset 50.
        let rows = visible_rows(viewport(50.0, 300.0), 32.0, 44.0, 100);
        assert_eq!(rows.start, 0);
    }

    #[test]
    fn range_is_clamped_to_row_count() {
        assert_eq!(visible_rows(viewport(0.0, 10_000.0), 32.0, 44.0, 3), 0..3);
        assert_eq!(visible_rows(viewport(0.0, 700.0), 32.0, 44.0, 0), 0..0);
    }

    #[test]
    fn bad_geometry_never_panics_or_inverts() {
        let nan = Rect::from_min_max(Pos2::new(f32::NAN, f32::NAN), Pos2::new(f32::NAN, f32::NAN));
        let rows = visible_rows(nan, f32::NAN, f32::NAN, 10);
        assert!(rows.start <= rows.end);
        let rows = visible_rows(viewport(500.0, 100.0), 32.0, 0.0, 10);
        assert!(rows.start <= rows.end);
    }

    #[test]
    fn step_row_moves_through_the_header_and_clamps() {
        // Up from row 0 reaches the header (None); Down from the header reaches row 0.
        assert_eq!(step_row(Some(0), -1, 5), None);
        assert_eq!(step_row(None, 1, 5), Some(0));
        assert_eq!(step_row(None, -1, 5), None);
        assert_eq!(step_row(Some(4), 1, 5), Some(4));
        assert_eq!(step_row(Some(1), 10, 5), Some(4));
        assert_eq!(step_row(Some(2), -10, 5), None);
        // No rows: the header is the only position.
        assert_eq!(step_row(None, 1, 0), None);
    }

    #[test]
    fn step_column_clamps_at_both_ends() {
        assert_eq!(step_column(0, -1, 3), 0);
        assert_eq!(step_column(2, 1, 3), 2);
        assert_eq!(step_column(1, 1, 3), 2);
        assert_eq!(step_column(0, 1, 0), 0);
    }

    #[test]
    fn find_row_uses_the_hint_then_searches() {
        let row_id = |row: usize| index(row * 2);
        assert_eq!(find_row(3, 10, &row_id, index(6)), Some(3));
        assert_eq!(find_row(0, 10, &row_id, index(18)), Some(9));
        assert_eq!(find_row(0, 10, &row_id, index(7)), None);
    }

    #[test]
    fn select_only_and_toggle_set_the_anchor() {
        let mut selection = Selection::default();
        select_only(&mut selection, index(2));
        toggle(&mut selection, index(4));
        assert_eq!(selection.selected.len(), 2);
        assert_eq!(selection.anchor, Some(index(4)));
        toggle(&mut selection, index(2));
        assert!(!selection.selected.contains(&index(2)));
    }

    #[test]
    fn range_replaces_or_adds_and_handles_reversed_ends() {
        let row_id = |row: usize| index(row);
        let mut selection = Selection::default();
        select_only(&mut selection, index(0));
        select_range(&mut selection, 5, 2, 10, &row_id, false);
        let got: Vec<u64> = selection.selected.iter().map(|id| id.0).collect();
        assert_eq!(got, vec![2, 3, 4, 5]);
        select_range(&mut selection, 8, 9, 10, &row_id, true);
        assert_eq!(selection.selected.len(), 6);
        assert_eq!(selection.anchor, Some(index(0)));
    }

    #[test]
    fn range_past_the_end_is_clamped() {
        let row_id = |row: usize| index(row);
        let mut selection = Selection::default();
        select_range(&mut selection, 7, 40, 10, &row_id, false);
        assert_eq!(selection.selected.len(), 3);
        select_range(&mut selection, 0, 0, 0, &row_id, false);
        assert!(selection.selected.is_empty());
    }

    #[test]
    fn select_all_and_all_selected_agree() {
        let row_id = |row: usize| index(row);
        let mut selection = Selection::default();
        assert!(!all_selected(&selection, 4, &row_id));
        select_all(&mut selection, 4, &row_id);
        assert!(all_selected(&selection, 4, &row_id));
        assert!(!all_selected(&selection, 0, &row_id));
    }
}
