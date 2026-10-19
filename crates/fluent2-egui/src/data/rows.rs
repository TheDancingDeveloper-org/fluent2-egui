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

/// Largest row height a measurement may record, so one runaway row cannot make the content
/// height non-finite.
pub(crate) const MAX_MEASURED_HEIGHT: f32 = 100_000.0;

/// Measured row heights for variable-height rows (`docs/specs/data-views.md` §6.1), keyed by
/// [`RowId`] so a measurement follows its row through inserts and sorts. Rows not measured yet
/// use the estimate.
#[derive(Clone, Debug, Default)]
pub(crate) struct HeightCache {
    heights: std::collections::HashMap<RowId, f32>,
}

impl HeightCache {
    /// The height of `id`, or `estimate` if it has not been measured.
    #[must_use]
    pub(crate) fn get(&self, id: RowId, estimate: f32) -> f32 {
        self.heights.get(&id).copied().unwrap_or(estimate)
    }

    /// Record a measured height (clamped to `1..=MAX_MEASURED_HEIGHT`; non-finite is ignored).
    /// Returns whether the stored height changed by more than half a point.
    pub(crate) fn set(&mut self, id: RowId, height: f32) -> bool {
        if !height.is_finite() {
            return false;
        }
        let height = height.clamp(1.0, MAX_MEASURED_HEIGHT);
        let old = self.heights.insert(id, height);
        old.is_none_or(|old| (old - height).abs() > 0.5)
    }

    /// Forget every measurement.
    pub(crate) fn clear(&mut self) {
        self.heights.clear();
    }

    /// Number of measured rows.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn len(&self) -> usize {
        self.heights.len()
    }
}

/// Row tops in content coordinates: `offsets[r]` is the top of row `r`, and the last entry is
/// the content height. O(`row_count`).
#[must_use]
pub(crate) fn row_offsets(row_count: usize, height: &dyn Fn(usize) -> f32) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(row_count + 1);
    let mut y = 0.0f32;
    offsets.push(y);
    for row in 0..row_count {
        let h = height(row);
        y += if h.is_finite() {
            h.clamp(1.0, MAX_MEASURED_HEIGHT)
        } else {
            1.0
        };
        offsets.push(y);
    }
    offsets
}

/// The row containing content `y` (clamped to the rows). `offsets` comes from [`row_offsets`].
/// O(log `row_count`).
#[must_use]
pub(crate) fn row_at(offsets: &[f32], y: f32) -> usize {
    let rows = offsets.len().saturating_sub(1);
    if rows == 0 {
        return 0;
    }
    let y = finite_or(y, 0.0);
    // First row whose bottom is below y.
    let after = offsets
        .get(1..)
        .unwrap_or(&[])
        .partition_point(|&bottom| bottom <= y);
    after.min(rows - 1)
}

/// Rows of variable height whose band intersects `viewport`, as `first..end`.
#[must_use]
pub(crate) fn visible_rows_variable(viewport: Rect, offsets: &[f32]) -> Range<usize> {
    let rows = offsets.len().saturating_sub(1);
    let total = offsets.last().copied().unwrap_or(0.0);
    let top = finite_or(viewport.min.y, 0.0).max(0.0);
    let bottom = finite_or(viewport.max.y, 0.0);
    if rows == 0 || top >= total || bottom <= top {
        return 0..0;
    }
    let first = row_at(offsets, top);
    // One past the last row whose top is above the viewport's bottom.
    let tops = offsets.get(..rows).unwrap_or(&[]);
    let end = tops.partition_point(|&top| top < bottom).clamp(first, rows);
    first..end
}

/// Scroll anchor for variable-height rows: the row at the top of the viewport and how far into
/// it the viewport starts. Keeping it fixed keeps the content on screen still when rows above
/// change height or are inserted.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ScrollAnchor {
    /// The row at the top of the viewport.
    pub(crate) row: RowId,
    /// Its last known index, checked first (§3.4).
    pub(crate) hint: usize,
    /// Points from the row's top to the viewport's top.
    pub(crate) offset: f32,
}

/// The anchor for a viewport whose top is at content `scroll_y`.
#[must_use]
pub(crate) fn anchor_at(
    offsets: &[f32],
    scroll_y: f32,
    row_id: &dyn Fn(usize) -> RowId,
) -> Option<ScrollAnchor> {
    if offsets.len() < 2 {
        return None;
    }
    let row = row_at(offsets, scroll_y);
    let top = offsets.get(row).copied().unwrap_or(0.0);
    Some(ScrollAnchor {
        row: row_id(row),
        hint: row,
        offset: (finite_or(scroll_y, 0.0) - top).max(0.0),
    })
}

/// The scroll offset that puts `anchor` back where it was, or `None` if its row is gone.
#[must_use]
pub(crate) fn anchored_scroll(
    anchor: ScrollAnchor,
    offsets: &[f32],
    row_id: &dyn Fn(usize) -> RowId,
) -> Option<f32> {
    let rows = offsets.len().saturating_sub(1);
    let index = find_row(anchor.hint, rows, row_id, anchor.row)?;
    let top = offsets.get(index).copied()?;
    let height = offsets.get(index + 1).copied().unwrap_or(top) - top;
    Some(top + anchor.offset.min(height))
}

#[cfg(test)]
mod tests {
    use egui::{Pos2, Rect};

    use super::{
        HeightCache, all_selected, anchor_at, anchored_scroll, find_row, row_at, row_offsets, select_all,
        select_only, select_range, step_column, step_row, toggle, visible_rows, visible_rows_variable,
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

    // ── Variable-height rows (§6.1) ──────────────────────────────────────────

    fn offsets_of(heights: &[f32]) -> Vec<f32> {
        row_offsets(heights.len(), &|row| heights.get(row).copied().unwrap_or(1.0))
    }

    #[test]
    fn offsets_are_prefix_sums_and_clamp_bad_heights() {
        assert_eq!(offsets_of(&[10.0, 20.0, 5.0]), vec![0.0, 10.0, 30.0, 35.0]);
        assert_eq!(offsets_of(&[f32::NAN, -4.0, 0.0]), vec![0.0, 1.0, 2.0, 3.0]);
        assert_eq!(offsets_of(&[]), vec![0.0]);
    }

    #[test]
    fn row_at_finds_the_row_under_y() {
        let o = offsets_of(&[10.0, 20.0, 5.0]);
        assert_eq!(row_at(&o, 0.0), 0);
        assert_eq!(row_at(&o, 9.9), 0);
        assert_eq!(row_at(&o, 10.0), 1);
        assert_eq!(row_at(&o, 29.0), 1);
        assert_eq!(row_at(&o, 30.0), 2);
        assert_eq!(row_at(&o, 1e9), 2);
        assert_eq!(row_at(&o, -5.0), 0);
        assert_eq!(row_at(&o, f32::NAN), 0);
        assert_eq!(row_at(&[0.0], 5.0), 0);
    }

    #[test]
    fn variable_visible_rows_cover_the_viewport() {
        let o = offsets_of(&[10.0, 20.0, 5.0, 40.0, 10.0]);
        assert_eq!(visible_rows_variable(viewport(0.0, 10.0), &o), 0..1);
        assert_eq!(visible_rows_variable(viewport(5.0, 31.0), &o), 0..3);
        assert_eq!(visible_rows_variable(viewport(36.0, 1000.0), &o), 3..5);
        assert_eq!(visible_rows_variable(viewport(1000.0, 2000.0), &o), 0..0);
        assert_eq!(visible_rows_variable(viewport(10.0, 5.0), &o), 0..0);
        assert_eq!(visible_rows_variable(viewport(0.0, 100.0), &[0.0]), 0..0);
    }

    #[test]
    fn variable_rows_of_ten_thousand_are_cheap_to_query() {
        let heights: Vec<f32> = (0..10_000).map(|i| 20.0 + (i % 7) as f32 * 13.0).collect();
        let o = offsets_of(&heights);
        let total = *o.last().unwrap_or(&0.0);
        let range = visible_rows_variable(viewport(total / 2.0, total / 2.0 + 600.0), &o);
        assert!(range.len() >= 6 && range.len() <= 31, "{range:?}");
        // Every row in the range intersects the viewport, and its neighbours do not.
        assert!(o[range.start] <= total / 2.0 && o[range.start + 1] > total / 2.0);
        assert!(o[range.end - 1] < total / 2.0 + 600.0 && o[range.end] >= total / 2.0 + 600.0);
    }

    #[test]
    fn height_cache_uses_the_estimate_until_measured() {
        let mut cache = HeightCache::default();
        assert_eq!(cache.get(index(3), 40.0), 40.0);
        assert!(cache.set(index(3), 72.0));
        assert_eq!(cache.get(index(3), 40.0), 72.0);
        assert!(!cache.set(index(3), 72.2), "sub-point jitter is not a change");
        assert!(cache.set(index(3), 90.0));
        assert!(!cache.set(index(4), f32::NAN));
        assert!(cache.set(index(5), 1e12));
        assert_eq!(cache.get(index(5), 1.0), super::MAX_MEASURED_HEIGHT);
        assert_eq!(cache.len(), 2);
        cache.clear();
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn anchor_holds_the_view_when_rows_above_grow() {
        let ids = |row: usize| index(row);
        let before = offsets_of(&[40.0; 100]);
        // Viewport top is 10 pt into row 25.
        let anchor = anchor_at(&before, 25.0 * 40.0 + 10.0, &ids).unwrap_or_else(|| unreachable!());
        assert_eq!((anchor.row, anchor.offset), (index(25), 10.0));
        // Rows 0..10 are measured at 100 instead of 40: everything below moves down 600.
        let mut heights = [40.0; 100];
        heights[..10].fill(100.0);
        let after = offsets_of(&heights);
        assert_eq!(
            anchored_scroll(anchor, &after, &ids),
            Some(25.0 * 40.0 + 600.0 + 10.0)
        );
    }

    #[test]
    fn anchor_follows_its_row_through_an_insert_above() {
        let before_ids = |row: usize| index(row + 100);
        let before = offsets_of(&[30.0; 50]);
        let anchor = anchor_at(&before, 300.0, &before_ids).unwrap_or_else(|| unreachable!());
        assert_eq!(anchor.row, index(110));
        // Three rows are inserted at the top: the anchored row is now at index 13.
        let after_ids = |row: usize| {
            if row < 3 {
                index(row + 9_000)
            } else {
                index(row - 3 + 100)
            }
        };
        let after = offsets_of(&[30.0; 53]);
        assert_eq!(anchored_scroll(anchor, &after, &after_ids), Some(13.0 * 30.0));
    }

    #[test]
    fn anchor_offset_is_clamped_when_its_row_shrinks_and_none_when_it_goes() {
        let ids = |row: usize| index(row);
        let anchor = anchor_at(&offsets_of(&[100.0, 100.0]), 180.0, &ids).unwrap_or_else(|| unreachable!());
        assert_eq!(anchor.offset, 80.0);
        assert_eq!(
            anchored_scroll(anchor, &offsets_of(&[100.0, 20.0]), &ids),
            Some(120.0)
        );
        let gone = |row: usize| index(row + 50);
        assert_eq!(anchored_scroll(anchor, &offsets_of(&[100.0, 20.0]), &gone), None);
        assert_eq!(anchor_at(&[0.0], 10.0, &ids), None);
    }
}
