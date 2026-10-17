//! Column definitions and the width policy (§1.1).

use std::collections::BTreeMap;

use egui::{Align as EguiAlign, Layout, WidgetText};

/// Width policy of a column. Values are logical points.
///
/// ```
/// use fluent2_egui::data::data_grid::{Column, ColumnWidth};
/// let column = Column::new("name", "Name").width(ColumnWidth::Fixed(160.0));
/// # let _ = column;
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ColumnWidth {
    /// Exactly this wide.
    Fixed(f32),
    /// Autofit on first show (§2.3), then behaves as `Fixed`. Until autofit lands, it is `min`.
    Auto,
    /// A share of the width left after fixed and auto columns. Shares are relative:
    /// `Fraction(1.0)` and `Fraction(2.0)` split the remainder 1:2.
    Fraction(f32),
}

/// Horizontal alignment of a column's header and cells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Align {
    /// Left, or right in right-to-left layouts.
    #[default]
    Start,
    /// Centred.
    Center,
    /// Right, or left in right-to-left layouts.
    End,
}

/// One column of a DataGrid.
///
/// ```
/// use fluent2_egui::data::data_grid::{Align, Column, ColumnWidth};
/// let column = Column::new("size", "Size")
///     .width(ColumnWidth::Fixed(96.0))
///     .min_width(64.0)
///     .align(Align::End);
/// # let _ = column;
/// ```
#[derive(Clone, Debug)]
pub struct Column {
    pub(super) id: &'static str,
    pub(super) header: WidgetText,
    pub(super) width: ColumnWidth,
    pub(super) min: f32,
    pub(super) max: f32,
    pub(super) sortable: bool,
    pub(super) resizable: bool,
    pub(super) align: Align,
}

impl Column {
    /// A column with id `id` (see D10) and visible header `header`.
    pub fn new(id: &'static str, header: impl Into<WidgetText>) -> Self {
        Self {
            id,
            header: header.into(),
            width: ColumnWidth::Fraction(1.0),
            min: 44.0,
            max: f32::INFINITY,
            sortable: true,
            resizable: true,
            align: Align::Start,
        }
    }

    /// Width policy. Default `ColumnWidth::Fraction(1.0)`.
    #[must_use]
    pub fn width(mut self, width: ColumnWidth) -> Self {
        self.width = width;
        self
    }

    /// Smallest width after the policy and resizing. Default 44.
    #[must_use]
    pub fn min_width(mut self, min: f32) -> Self {
        self.min = min;
        self
    }

    /// Largest width after the policy and resizing. Default unbounded.
    #[must_use]
    pub fn max_width(mut self, max: f32) -> Self {
        self.max = max;
        self
    }

    /// Whether the header sorts the column. Default true.
    #[must_use]
    pub fn sortable(mut self, sortable: bool) -> Self {
        self.sortable = sortable;
        self
    }

    /// Whether the column can be resized. Default true.
    #[must_use]
    pub fn resizable(mut self, resizable: bool) -> Self {
        self.resizable = resizable;
        self
    }

    /// Alignment of header text and cell layout. Default `Align::Start`.
    #[must_use]
    pub fn align(mut self, align: Align) -> Self {
        self.align = align;
        self
    }

    /// Layout of a cell or header cell: vertically centred, aligned by [`Align`].
    pub(super) fn cell_layout(&self) -> Layout {
        match self.align {
            Align::Start => Layout::left_to_right(EguiAlign::Center),
            Align::Center => Layout::left_to_right(EguiAlign::Center).with_main_align(EguiAlign::Center),
            Align::End => Layout::right_to_left(EguiAlign::Center),
        }
    }
}

/// Resolves every column's width for one frame (§1.1), in display order.
///
/// Order: a width in `widths` for the column id wins; then `Fixed`; then `Auto` (its `min` until
/// autofit lands); then `Fraction` columns share what is left. Every result is clamped to
/// `[min, max]`, where `min` wins if `min > max`. Non-finite inputs are treated as 0 or
/// unbounded, never propagated.
#[must_use]
pub(super) fn resolve_widths(columns: &[Column], available: f32, widths: &BTreeMap<String, f32>) -> Vec<f32> {
    let available = finite_or(available, 0.0).max(0.0);
    let mut resolved = vec![0.0_f32; columns.len()];
    let mut fixed_sum = 0.0_f32;
    let mut share_sum = 0.0_f32;
    for (index, column) in columns.iter().enumerate() {
        let stored = widths.get(column.id).copied().filter(|w| w.is_finite());
        let fraction = match column.width {
            ColumnWidth::Fraction(share) if stored.is_none() => Some(finite_or(share, 0.0).max(0.0)),
            _ => None,
        };
        match fraction {
            Some(share) => {
                share_sum += share;
                resolved[index] = share;
            }
            None => {
                let width = stored.unwrap_or(match column.width {
                    ColumnWidth::Fixed(w) => finite_or(w, column.min),
                    _ => column.min,
                });
                let width = clamp_width(width, column.min, column.max);
                fixed_sum += width;
                resolved[index] = width;
            }
        }
    }
    let remainder = (available - fixed_sum).max(0.0);
    for (index, column) in columns.iter().enumerate() {
        let is_share = matches!(column.width, ColumnWidth::Fraction(_))
            && widths.get(column.id).is_none_or(|w| !w.is_finite());
        if is_share {
            let share = resolved[index];
            let width = if share_sum > 0.0 {
                remainder * share / share_sum
            } else {
                0.0
            };
            resolved[index] = clamp_width(width, column.min, column.max);
        }
    }
    resolved
}

/// Left edge of each column, relative to the grid's left edge (a prefix sum of `widths`).
#[must_use]
pub(super) fn prefix_offsets(widths: &[f32]) -> Vec<f32> {
    let mut offsets = Vec::with_capacity(widths.len());
    let mut x = 0.0_f32;
    for width in widths {
        offsets.push(x);
        x += width;
    }
    offsets
}

fn clamp_width(width: f32, min: f32, max: f32) -> f32 {
    let min = finite_or(min, 0.0).max(0.0);
    let max = if max.is_nan() { f32::INFINITY } else { max };
    if max < min {
        return min;
    }
    finite_or(width, min).clamp(min, max)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{Column, ColumnWidth, resolve_widths};

    fn none() -> BTreeMap<String, f32> {
        BTreeMap::new()
    }

    fn assert_close(actual: &[f32], expected: &[f32]) {
        assert_eq!(actual.len(), expected.len());
        for (a, e) in actual.iter().zip(expected) {
            assert!((a - e).abs() < 0.01, "{actual:?} != {expected:?}");
        }
    }

    #[test]
    fn fixed_is_exact_within_limits() {
        let columns = [Column::new("a", "A").width(ColumnWidth::Fixed(100.0))];
        assert_close(&resolve_widths(&columns, 500.0, &none()), &[100.0]);
    }

    #[test]
    fn fixed_is_clamped_to_min_and_max() {
        let columns = [
            Column::new("small", "S").width(ColumnWidth::Fixed(10.0)),
            Column::new("big", "B")
                .width(ColumnWidth::Fixed(300.0))
                .max_width(200.0),
        ];
        assert_close(&resolve_widths(&columns, 500.0, &none()), &[44.0, 200.0]);
    }

    #[test]
    fn auto_is_min_until_autofit_lands() {
        let columns = [Column::new("a", "A").width(ColumnWidth::Auto).min_width(72.0)];
        assert_close(&resolve_widths(&columns, 500.0, &none()), &[72.0]);
    }

    #[test]
    fn fractions_split_the_remainder() {
        let columns = [
            Column::new("fixed", "F").width(ColumnWidth::Fixed(100.0)),
            Column::new("one", "1").width(ColumnWidth::Fraction(1.0)),
            Column::new("two", "2").width(ColumnWidth::Fraction(2.0)),
        ];
        assert_close(&resolve_widths(&columns, 400.0, &none()), &[100.0, 100.0, 200.0]);
    }

    #[test]
    fn fraction_respects_min_and_max() {
        let columns = [
            Column::new("narrow", "N")
                .width(ColumnWidth::Fraction(1.0))
                .min_width(150.0),
            Column::new("wide", "W")
                .width(ColumnWidth::Fraction(3.0))
                .max_width(120.0),
        ];
        assert_close(&resolve_widths(&columns, 200.0, &none()), &[150.0, 120.0]);
    }

    #[test]
    fn no_remainder_gives_min() {
        let columns = [
            Column::new("fixed", "F").width(ColumnWidth::Fixed(300.0)),
            Column::new("share", "S").width(ColumnWidth::Fraction(1.0)),
        ];
        assert_close(&resolve_widths(&columns, 200.0, &none()), &[300.0, 44.0]);
    }

    #[test]
    fn stored_width_wins_over_policy() {
        let columns = [Column::new("a", "A")
            .width(ColumnWidth::Fixed(100.0))
            .max_width(150.0)];
        let mut stored = BTreeMap::new();
        stored.insert("a".to_owned(), 400.0);
        assert_close(&resolve_widths(&columns, 500.0, &stored), &[150.0]);
    }

    #[test]
    fn min_wins_when_min_exceeds_max() {
        let columns = [Column::new("a", "A")
            .width(ColumnWidth::Fixed(80.0))
            .min_width(60.0)
            .max_width(30.0)];
        assert_close(&resolve_widths(&columns, 500.0, &none()), &[60.0]);
    }

    #[test]
    fn non_finite_input_never_leaks_into_widths() {
        let columns = [
            Column::new("nan", "N").width(ColumnWidth::Fixed(f32::NAN)),
            Column::new("share", "S").width(ColumnWidth::Fraction(f32::INFINITY)),
        ];
        let widths = resolve_widths(&columns, f32::NAN, &none());
        assert!(widths.iter().all(|w| w.is_finite() && *w >= 0.0), "{widths:?}");
    }
}
