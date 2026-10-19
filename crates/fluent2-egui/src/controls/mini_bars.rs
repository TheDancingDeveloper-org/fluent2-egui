//! MiniBars: a small, axis-free bar chart, for counts per day or per bucket in a dashboard tile.
//!
//! Bars are `compoundBrandBackground` and grow from zero, so negative values hang below a
//! `neutralStroke2` zero line. Bars are separated by `spacingHorizontalXXS` while they stay at
//! least that wide. A non-finite value leaves its slot empty. High contrast maps the tokens to
//! system colours. Larger charts belong to `egui_plot` in the app.
//!
//! The AccessKit node is an image labelled with a
//! [`SeriesSummary`](super::SeriesSummary).

use egui::{CornerRadius, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget};

use fluent2_tokens::generated::global::{border_radius, spacing_horizontal, stroke_width};

use super::sparkline::{SeriesSummary, default_size, describe, finite_size, unit};
use crate::theme::{Theme, c};

/// A mini bar chart over a series of values.
///
/// ```
/// use fluent2_egui::controls::MiniBars;
/// # fn demo(ui: &mut egui::Ui) {
/// ui.add(MiniBars::new(&[4.0, 7.0, 2.0, 9.0]).label("Deploys per day"));
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct MiniBars<'a> {
    values: &'a [f32],
    label: String,
    size: Vec2,
    max: Option<f32>,
}

impl<'a> MiniBars<'a> {
    /// Bars for `values`, oldest first.
    #[must_use]
    pub fn new(values: &'a [f32]) -> Self {
        Self {
            values,
            label: "Bars".into(),
            size: default_size(),
            max: None,
        }
    }

    /// Accessible name, used in the summary label. Defaults to "Bars".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Size. Defaults to one body line high (20) and five wide (100).
    #[must_use]
    pub fn size(mut self, size: Vec2) -> Self {
        self.size = size;
        self
    }

    /// Fixed top of the scale, so several charts compare. Defaults to the largest value. A
    /// non-finite or non-positive value falls back to the default.
    #[must_use]
    pub fn max(mut self, max: f32) -> Self {
        self.max = Some(max);
        self
    }

    /// The bar rectangles in `rect`, `None` for an empty slot, and the zero line's y. Pure,
    /// for tests.
    fn bars(&self, rect: Rect, summary: Option<SeriesSummary>) -> (Vec<Option<Rect>>, f32) {
        let Some(s) = summary else {
            return (Vec::new(), rect.bottom());
        };
        let hi = match self.max {
            Some(m) if m.is_finite() && m > 0.0 => m,
            _ => s.max.max(0.0),
        };
        let lo = s.min.min(0.0);
        // All zeros (hi == lo) put every bar on the bottom edge.
        let to_y = |v: f32| {
            if hi > lo {
                rect.bottom() - rect.height() * unit(v, lo, hi)
            } else {
                rect.bottom()
            }
        };
        let zero = to_y(0.0);
        let n = self.values.len().max(1) as f32;
        let mut gap = spacing_horizontal::XXS;
        let mut width = (rect.width() - gap * (n - 1.0)) / n;
        if width < gap {
            gap = 0.0;
            width = rect.width() / n;
        }
        let bars = self
            .values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.is_finite().then(|| {
                    let x = rect.left() + i as f32 * (width + gap);
                    let y = to_y(*v);
                    let (top, bottom) = if y <= zero { (y, zero) } else { (zero, y) };
                    // A zero value still shows a hairline, so the slot reads as data.
                    let bottom = bottom.max(top + stroke_width::THIN);
                    Rect::from_min_max(Pos2::new(x, top), Pos2::new(x + width, bottom))
                })
            })
            .collect();
        (bars, zero)
    }
}

impl Widget for MiniBars<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(finite_size(self.size), Sense::hover());
        let summary = SeriesSummary::of(self.values);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect);
            let (bars, zero) = self.bars(rect, summary);
            let fill = c(theme.color.compound_brand_background);
            let radius = CornerRadius::same(border_radius::SMALL as u8);
            for bar in bars.iter().flatten() {
                let r = if bar.width() > 2.0 * border_radius::SMALL {
                    radius
                } else {
                    CornerRadius::ZERO
                };
                painter.rect_filled(*bar, r, fill);
            }
            painter.hline(
                rect.x_range(),
                zero - stroke_width::THIN / 2.0,
                Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)),
            );
        }
        describe(ui, &response, &SeriesSummary::label(summary, &self.label));
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 20.0))
    }

    #[test]
    fn bars_grow_from_zero_with_gaps() {
        let values = [5.0, 10.0];
        let (bars, zero) = MiniBars::new(&values).bars(rect(), SeriesSummary::of(&values));
        assert_eq!(zero, 20.0);
        assert_eq!(
            bars[0],
            Some(Rect::from_min_max(Pos2::new(0.0, 10.0), Pos2::new(49.0, 20.0)))
        );
        assert_eq!(
            bars[1],
            Some(Rect::from_min_max(Pos2::new(51.0, 0.0), Pos2::new(100.0, 20.0)))
        );
    }

    #[test]
    fn negatives_hang_below_the_zero_line() {
        let values = [-10.0, 10.0];
        let (bars, zero) = MiniBars::new(&values).bars(rect(), SeriesSummary::of(&values));
        assert_eq!(zero, 10.0);
        assert_eq!(bars[0].map(|b| (b.top(), b.bottom())), Some((10.0, 20.0)));
        assert_eq!(bars[1].map(|b| (b.top(), b.bottom())), Some((0.0, 10.0)));
    }

    #[test]
    fn many_bars_drop_the_gap_and_nan_is_empty() {
        let values: Vec<f32> = (0..200).map(|i| if i == 3 { f32::NAN } else { 1.0 }).collect();
        let (bars, _) = MiniBars::new(&values).bars(rect(), SeriesSummary::of(&values));
        assert_eq!(bars.len(), 200);
        assert!(bars[3].is_none());
        assert_eq!(bars[1].map(|b| b.left()), Some(0.5));
    }

    #[test]
    fn zeros_and_fixed_max() {
        let values = [0.0, 0.0];
        let (bars, zero) = MiniBars::new(&values).bars(rect(), SeriesSummary::of(&values));
        assert_eq!(zero, 20.0);
        assert!(bars.iter().flatten().all(|b| b.height() >= stroke_width::THIN));
        let values = [5.0];
        let (bars, _) = MiniBars::new(&values)
            .max(10.0)
            .bars(rect(), SeriesSummary::of(&values));
        assert_eq!(bars[0].map(|b| b.top()), Some(10.0));
        let (bars, _) = MiniBars::new(&values)
            .max(f32::NAN)
            .bars(rect(), SeriesSummary::of(&values));
        assert_eq!(bars[0].map(|b| b.top()), Some(0.0));
    }
}
