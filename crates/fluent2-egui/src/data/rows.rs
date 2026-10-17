//! Viewport-to-row maths shared by the data views (`docs/specs/data-views.md` D7).

use std::ops::Range;

use egui::Rect;

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

#[cfg(test)]
mod tests {
    use egui::{Pos2, Rect};

    use super::visible_rows;

    fn viewport(top: f32, bottom: f32) -> Rect {
        Rect::from_min_max(Pos2::new(0.0, top), Pos2::new(100.0, bottom))
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
}
