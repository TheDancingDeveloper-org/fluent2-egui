//! Sparkline: a small, axis-free line of a series, for table cells and status rows.
//!
//! The line is `compoundBrandForeground1` at `strokeWidthThick`, with a dot on the last value;
//! the baseline (empty or flat series) is `neutralStroke2`. High contrast maps these tokens to
//! the system colours, so the line stays visible there. Non-finite values are gaps: the line
//! breaks around them. Larger charts belong to `egui_plot` in the app.
//!
//! The AccessKit node is an image labelled with a [`SeriesSummary`], for example
//! "CPU: 12 values, min 3, max 41, last 17".

use egui::accesskit::Role;
use egui::{Pos2, Rect, Response, Sense, Stroke, Ui, Vec2, Widget, WidgetInfo, WidgetType};

use fluent2_tokens::generated::global::{line_height, spacing_horizontal, stroke_width};

use crate::theme::{Theme, c};

/// Count, range and last value of the finite values in a series.
///
/// ```
/// use fluent2_egui::controls::SeriesSummary;
/// let s = SeriesSummary::of(&[3.0, f32::NAN, 9.0, 5.0]).unwrap();
/// assert_eq!((s.count, s.min, s.max, s.last), (3, 3.0, 9.0, 5.0));
/// assert!(SeriesSummary::of(&[f32::NAN]).is_none());
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeriesSummary {
    /// Number of finite values.
    pub count: usize,
    /// Smallest finite value.
    pub min: f32,
    /// Largest finite value.
    pub max: f32,
    /// Last finite value.
    pub last: f32,
}

impl SeriesSummary {
    /// Summarise `values`, skipping NaN and infinities. `None` when nothing is finite.
    #[must_use]
    pub fn of(values: &[f32]) -> Option<Self> {
        let mut finite = values.iter().copied().filter(|v| v.is_finite());
        let first = finite.next()?;
        let mut s = Self {
            count: 1,
            min: first,
            max: first,
            last: first,
        };
        for v in finite {
            s.count += 1;
            s.min = s.min.min(v);
            s.max = s.max.max(v);
            s.last = v;
        }
        Some(s)
    }

    /// The accessible label: `"{name}: 12 values, min 3, max 41, last 17"`, or
    /// `"{name}: no data"`.
    #[must_use]
    pub fn label(summary: Option<Self>, name: &str) -> String {
        match summary {
            None => format!("{name}: no data"),
            Some(s) => format!(
                "{name}: {} value{}, min {}, max {}, last {}",
                s.count,
                if s.count == 1 { "" } else { "s" },
                fmt(s.min),
                fmt(s.max),
                fmt(s.last)
            ),
        }
    }
}

/// Shortest readable form: integers without a fraction, others to two decimals.
fn fmt(v: f32) -> String {
    if v.fract() == 0.0 && v.abs() < 1e9 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

/// Map `v` from `min..=max` to `0..=1`. A flat or non-finite range maps to the middle, a
/// non-finite value to 0.
pub(crate) fn unit(v: f32, min: f32, max: f32) -> f32 {
    let span = max - min;
    if span.is_finite() && span > 0.0 {
        let u = (v - min) / span;
        if u.is_finite() { u.clamp(0.0, 1.0) } else { 0.0 }
    } else {
        0.5
    }
}

/// Default size: one body line high, five lines wide.
pub(crate) fn default_size() -> Vec2 {
    Vec2::new(line_height::BASE_300 * 5.0, line_height::BASE_300)
}

pub(crate) fn finite_size(size: Vec2) -> Vec2 {
    let fallback = default_size();
    let pick = |v: f32, f: f32| if v.is_finite() && v > 0.0 { v } else { f };
    Vec2::new(pick(size.x, fallback.x), pick(size.y, fallback.y))
}

/// Set the image role and summary label on a chart's response.
pub(crate) fn describe(ui: &Ui, response: &Response, label: &str) {
    let enabled = ui.is_enabled();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, enabled, label));
    let label = label.to_owned();
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_role(Role::Image);
        node.set_label(label);
    });
}

/// A sparkline over a series of values.
///
/// ```
/// use fluent2_egui::controls::Sparkline;
/// # fn demo(ui: &mut egui::Ui) {
/// ui.add(Sparkline::new(&[3.0, 8.0, 5.0, 12.0]).label("Requests"));
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct Sparkline<'a> {
    values: &'a [f32],
    label: String,
    size: Vec2,
    range: Option<(f32, f32)>,
    last_dot: bool,
}

impl<'a> Sparkline<'a> {
    /// A sparkline of `values`, oldest first.
    #[must_use]
    pub fn new(values: &'a [f32]) -> Self {
        Self {
            values,
            label: "Trend".into(),
            size: default_size(),
            range: None,
            last_dot: true,
        }
    }

    /// Accessible name, used in the summary label. Defaults to "Trend".
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

    /// Fixed vertical range, for example `(0.0, 100.0)` for a percentage. Defaults to the
    /// series' own min and max. A non-finite or inverted range falls back to the default.
    #[must_use]
    pub fn range(mut self, min: f32, max: f32) -> Self {
        self.range = Some((min, max));
        self
    }

    /// Mark the last value with a dot. Default on.
    #[must_use]
    pub fn last_dot(mut self, on: bool) -> Self {
        self.last_dot = on;
        self
    }

    /// Points in `rect`, `None` for a gap. Pure, for tests.
    fn points(&self, rect: Rect, summary: Option<SeriesSummary>) -> Vec<Option<Pos2>> {
        let Some(s) = summary else { return Vec::new() };
        let (min, max) = match self.range {
            Some((lo, hi)) if lo.is_finite() && hi.is_finite() && hi > lo => (lo, hi),
            _ => (s.min, s.max),
        };
        let n = self.values.len();
        let pad = stroke_width::THICK;
        let inner = rect.shrink(pad);
        self.values
            .iter()
            .enumerate()
            .map(|(i, v)| {
                v.is_finite().then(|| {
                    let x = if n > 1 {
                        inner.left() + inner.width() * i as f32 / (n - 1) as f32
                    } else {
                        inner.center().x
                    };
                    Pos2::new(x, inner.bottom() - inner.height() * unit(*v, min, max))
                })
            })
            .collect()
    }
}

impl Widget for Sparkline<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let (rect, response) = ui.allocate_exact_size(finite_size(self.size), Sense::hover());
        let summary = SeriesSummary::of(self.values);
        if ui.is_rect_visible(rect) {
            let painter = ui.painter_at(rect.expand(stroke_width::THICK));
            let line = c(theme.color.compound_brand_foreground_1);
            let stroke = Stroke::new(stroke_width::THICK, line);
            let points = self.points(rect, summary);
            let mut run: Vec<Pos2> = Vec::new();
            let flush = |run: &mut Vec<Pos2>| {
                match run.len() {
                    0 => {}
                    1 => {
                        if let Some(p) = run.first() {
                            painter.circle_filled(*p, stroke_width::THICK, line);
                        }
                    }
                    _ => {
                        painter.line(std::mem::take(run), stroke);
                    }
                }
                run.clear();
            };
            for p in &points {
                match p {
                    Some(p) => run.push(*p),
                    None => flush(&mut run),
                }
            }
            flush(&mut run);
            if summary.is_none() {
                // Nothing to plot: a baseline, so the cell does not look broken.
                painter.hline(
                    rect.x_range(),
                    rect.center().y,
                    Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)),
                );
            }
            if self.last_dot
                && let Some(last) = points.iter().rev().flatten().next()
            {
                painter.circle_filled(*last, spacing_horizontal::XXS + stroke_width::THIN, line);
            }
        }
        describe(ui, &response, &SeriesSummary::label(summary, &self.label));
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, Vec2::new(104.0, 24.0))
    }

    #[test]
    fn points_span_the_rect_and_invert_y() {
        let values = [0.0, 10.0];
        let s = Sparkline::new(&values);
        let p = s.points(rect(), SeriesSummary::of(&values));
        assert_eq!(p, vec![Some(Pos2::new(2.0, 22.0)), Some(Pos2::new(102.0, 2.0))]);
    }

    #[test]
    fn nan_is_a_gap_and_one_point_is_centred() {
        let values = [1.0, f32::NAN, 3.0];
        let s = Sparkline::new(&values);
        let p = s.points(rect(), SeriesSummary::of(&values));
        assert!(p[1].is_none());
        let one = [5.0];
        let p = Sparkline::new(&one).points(rect(), SeriesSummary::of(&one));
        assert_eq!(p, vec![Some(Pos2::new(52.0, 12.0))]);
    }

    #[test]
    fn bad_range_falls_back_and_values_clamp() {
        let values = [0.0, 200.0];
        let s = Sparkline::new(&values).range(0.0, 100.0);
        let p = s.points(rect(), SeriesSummary::of(&values));
        assert_eq!(p[1].map(|p| p.y), Some(2.0), "clamped to the top");
        let s = Sparkline::new(&values).range(f32::NAN, 1.0);
        let p = s.points(rect(), SeriesSummary::of(&values));
        assert_eq!(p[1].map(|p| p.y), Some(2.0));
    }

    #[test]
    fn unit_handles_flat_and_inverted() {
        assert_eq!(unit(3.0, 3.0, 3.0), 0.5);
        assert_eq!(unit(3.0, 5.0, 1.0), 0.5);
        assert_eq!(unit(f32::NAN, 0.0, 1.0), 0.0);
    }
}
