//! LineChart: a multi-series time-series chart with a time axis, a value axis, a legend, a
//! hover read-out and time-range chips (5 min, 1 h, 6 h). Behind the `chart` feature.
//!
//! Series come from [`ChartSeries`], a fixed-capacity ring buffer of `(time, value)` samples:
//! push one sample per tick and the oldest falls off. Series colours are Fluent palette tokens
//! in a fixed categorical order; in high contrast every palette token is the system text
//! colour, so series alternate between `neutralForeground1` and `compoundBrandForeground1` and
//! every second series is dashed. Gridlines are `neutralStroke2`, axis labels
//! `neutralForeground3` in caption1.
//!
//! Each frame walks only the samples inside the visible range and keeps at most two points per
//! pixel column (the column's min and max), so a 6 h series at 1 Hz costs about two points per
//! pixel of width, not 21 600.
//!
//! The AccessKit node is an image labelled with a summary of the visible samples, for example
//! "Speed, last 5 minutes: Download: 300 values, min 0 KiB/s, max 812 KiB/s, last 640 KiB/s;
//! Upload: no data".

use std::collections::VecDeque;

use egui::epaint::{Mesh, PathStroke};
use egui::{Align2, Color32, Id, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

use fluent2_tokens::generated::global::{line_height, spacing_horizontal, stroke_width};

use super::sparkline::describe;
use super::{ButtonSize, ToggleButton};
use crate::foundation::text::{TypeRamp, font_id};
use crate::theme::{Theme, ThemeKind, c};

/// Opacity of the optional area fill under a series line. Not a Fluent token: Fluent's chart
/// guidance shows area fills as a tint of the series colour.
const FILL_OPACITY: f32 = 0.18;

/// Default plot height: ten body lines.
const DEFAULT_HEIGHT: f32 = line_height::BASE_300 * 10.0;

/// The visible time window, chosen with the chips above the chart.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum TimeRange {
    /// The last five minutes. The default.
    #[default]
    FiveMinutes,
    /// The last hour.
    OneHour,
    /// The last six hours.
    SixHours,
}

impl TimeRange {
    /// Every range, in chip order.
    pub const ALL: [Self; 3] = [Self::FiveMinutes, Self::OneHour, Self::SixHours];

    /// Length in seconds.
    ///
    /// ```
    /// use fluent2_egui::controls::TimeRange;
    /// assert_eq!(TimeRange::OneHour.seconds(), 3600.0);
    /// ```
    #[must_use]
    pub const fn seconds(self) -> f64 {
        match self {
            Self::FiveMinutes => 300.0,
            Self::OneHour => 3_600.0,
            Self::SixHours => 21_600.0,
        }
    }

    /// Chip text: "5 min", "1 h", "6 h".
    #[must_use]
    pub const fn chip_label(self) -> &'static str {
        match self {
            Self::FiveMinutes => "5 min",
            Self::OneHour => "1 h",
            Self::SixHours => "6 h",
        }
    }

    /// Spoken form used in the accessible summary: "5 minutes", "1 hour", "6 hours".
    #[must_use]
    pub const fn spoken(self) -> &'static str {
        match self {
            Self::FiveMinutes => "5 minutes",
            Self::OneHour => "1 hour",
            Self::SixHours => "6 hours",
        }
    }
}

/// One named series: a ring buffer of `(time, value)` samples, oldest first.
///
/// Times are seconds on any clock the app likes (Unix time, seconds since start); they only
/// need to grow. A sample older than the newest one is refused. A non-finite value is a gap.
///
/// ```
/// use fluent2_egui::controls::ChartSeries;
/// let mut down = ChartSeries::new("Download", 3);
/// for t in 0..5 {
///     down.push(f64::from(t), t as f32 * 10.0);
/// }
/// assert_eq!(down.len(), 3);
/// assert_eq!(down.latest(), Some((4.0, 40.0)));
/// assert!(!down.push(1.0, 5.0), "older than the newest sample");
/// ```
#[derive(Clone, Debug)]
pub struct ChartSeries {
    name: String,
    samples: VecDeque<(f64, f32)>,
    capacity: usize,
    fill: bool,
}

impl ChartSeries {
    /// An empty series that keeps the last `capacity` samples (at least one). Room for all of
    /// them is reserved up front, so pushing never allocates.
    #[must_use]
    pub fn new(name: impl Into<String>, capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self {
            name: name.into(),
            samples: VecDeque::with_capacity(capacity),
            capacity,
            fill: false,
        }
    }

    /// Fill the area under the line with a tint of the series colour. Off by default, and
    /// never drawn in high contrast.
    #[must_use]
    pub fn fill(mut self, on: bool) -> Self {
        self.fill = on;
        self
    }

    /// Append a sample, dropping the oldest when full. Returns `false`, and keeps nothing, when
    /// `time` is not finite or is older than the newest sample.
    pub fn push(&mut self, time: f64, value: f32) -> bool {
        if !time.is_finite() || self.samples.back().is_some_and(|(t, _)| time < *t) {
            return false;
        }
        if self.samples.len() >= self.capacity {
            self.samples.pop_front();
        }
        self.samples.push_back((time, value));
        true
    }

    /// Remove every sample.
    pub fn clear(&mut self) {
        self.samples.clear();
    }

    /// The series name, shown in the legend and the read-out.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Number of samples held.
    #[must_use]
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// `true` when no samples are held.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }

    /// The newest sample.
    #[must_use]
    pub fn latest(&self) -> Option<(f64, f32)> {
        self.samples.back().copied()
    }

    /// Samples oldest first.
    pub fn iter(&self) -> impl Iterator<Item = (f64, f32)> + '_ {
        self.samples.iter().copied()
    }

    /// Samples with `start <= time <= end`, found by binary search.
    fn window(&self, start: f64, end: f64) -> impl Iterator<Item = (f64, f32)> + '_ {
        let from = self.samples.partition_point(|(t, _)| *t < start);
        self.samples
            .range(from..)
            .copied()
            .take_while(move |(t, _)| *t <= end)
    }

    /// The sample nearest `time` inside `start..=end`.
    fn nearest(&self, time: f64, start: f64, end: f64) -> Option<(f64, f32)> {
        let i = self.samples.partition_point(|(t, _)| *t < time);
        let after = self.samples.get(i).copied();
        let before = i.checked_sub(1).and_then(|j| self.samples.get(j)).copied();
        let pick = match (before, after) {
            (Some(b), Some(a)) => {
                if time - b.0 <= a.0 - time {
                    b
                } else {
                    a
                }
            }
            (Some(s), None) | (None, Some(s)) => s,
            (None, None) => return None,
        };
        (pick.0 >= start && pick.0 <= end).then_some(pick)
    }
}

/// What the pointer is over: the hovered time and each series' nearest value (`None` when a
/// series has no sample in range, or a gap there).
#[derive(Clone, Debug, PartialEq)]
pub struct ChartHover {
    /// The time under the pointer.
    pub time: f64,
    /// One entry per series, in series order.
    pub values: Vec<Option<f32>>,
}

/// What [`LineChart::show`] returns.
#[derive(Debug)]
pub struct LineChartResponse {
    /// The plot area.
    pub response: Response,
    /// `true` on the frame a chip changed the range.
    pub range_changed: bool,
    /// The hover read-out, while the pointer is over the plot.
    pub hover: Option<ChartHover>,
}

type Formatter<'a> = Box<dyn Fn(f64) -> String + 'a>;

/// A multi-series time-series chart.
///
/// ```
/// use fluent2_egui::controls::{ChartSeries, LineChart, TimeRange};
/// # fn demo(ui: &mut egui::Ui, down: &ChartSeries, up: &ChartSeries, range: &mut TimeRange) {
/// let series = [down.clone(), up.clone()];
/// LineChart::new("speed", &series)
///     .label("Speed")
///     .unit(" KiB/s")
///     .show(ui, range);
/// # }
/// ```
pub struct LineChart<'a> {
    id: Id,
    series: &'a [ChartSeries],
    label: String,
    height: f32,
    unit: String,
    value_format: Option<Formatter<'a>>,
    time_format: Option<Formatter<'a>>,
    now: Option<f64>,
    legend: bool,
    chips: bool,
}

impl<'a> LineChart<'a> {
    /// A chart of `series`; `id_salt` keeps its widgets apart from other charts.
    #[must_use]
    pub fn new(id_salt: impl std::hash::Hash + std::fmt::Debug, series: &'a [ChartSeries]) -> Self {
        Self {
            id: Id::new(id_salt),
            series,
            label: "Chart".into(),
            height: DEFAULT_HEIGHT,
            unit: String::new(),
            value_format: None,
            time_format: None,
            now: None,
            legend: true,
            chips: true,
        }
    }

    /// Accessible name, used in the summary label. Defaults to "Chart".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Plot height, axes included. Defaults to ten body lines (200). Non-finite or
    /// non-positive heights fall back to the default.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = height;
        self
    }

    /// Suffix for the default value formatter, for example `" KiB/s"`.
    #[must_use]
    pub fn unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = unit.into();
        self
    }

    /// Formats values on the axis, in the legend, the read-out and the summary. Replaces the
    /// default (shortest number plus [`Self::unit`]).
    #[must_use]
    pub fn value_format(mut self, f: impl Fn(f64) -> String + 'a) -> Self {
        self.value_format = Some(Box::new(f));
        self
    }

    /// Formats time-axis labels and the read-out time from seconds before the right edge.
    /// Defaults to [`time_ago`] ("now", "30 s", "5 min", "1 h 30 min").
    #[must_use]
    pub fn time_format(mut self, f: impl Fn(f64) -> String + 'a) -> Self {
        self.time_format = Some(Box::new(f));
        self
    }

    /// The time at the right edge. Defaults to the newest sample of any series.
    #[must_use]
    pub fn now(mut self, now: f64) -> Self {
        self.now = Some(now);
        self
    }

    /// Show the legend row. Default on.
    #[must_use]
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }

    /// Show the time-range chips. Default on.
    #[must_use]
    pub fn chips(mut self, on: bool) -> Self {
        self.chips = on;
        self
    }

    fn fmt_value(&self, v: f64) -> String {
        match &self.value_format {
            Some(f) => f(v),
            None => format!("{}{}", short(v), self.unit),
        }
    }

    fn fmt_time(&self, ago: f64) -> String {
        match &self.time_format {
            Some(f) => f(ago),
            None => time_ago(ago),
        }
    }

    /// The right edge: [`Self::now`], or the newest sample, or 0 with no data.
    fn end(&self) -> f64 {
        self.now.filter(|t| t.is_finite()).unwrap_or_else(|| {
            self.series
                .iter()
                .filter_map(|s| s.latest().map(|(t, _)| t))
                .fold(None, |m: Option<f64>, t| Some(m.map_or(t, |m| m.max(t))))
                .unwrap_or(0.0)
        })
    }

    /// The accessible summary of the samples visible in `range`.
    ///
    /// ```
    /// use fluent2_egui::controls::{ChartSeries, LineChart, TimeRange};
    /// let mut down = ChartSeries::new("Down", 10);
    /// down.push(0.0, 2.0);
    /// down.push(1.0, 6.0);
    /// let series = [down, ChartSeries::new("Up", 10)];
    /// let chart = LineChart::new("c", &series).label("Speed").unit(" KiB/s");
    /// assert_eq!(
    ///     chart.summary(TimeRange::FiveMinutes),
    ///     "Speed, last 5 minutes: Down: 2 values, min 2 KiB/s, max 6 KiB/s, last 6 KiB/s; Up: no data"
    /// );
    /// ```
    #[must_use]
    pub fn summary(&self, range: TimeRange) -> String {
        let end = self.end();
        let start = end - range.seconds();
        let mut out = format!("{}, last {}: ", self.label, range.spoken());
        if self.series.is_empty() {
            out.push_str("no series");
        }
        for (i, s) in self.series.iter().enumerate() {
            if i > 0 {
                out.push_str("; ");
            }
            let mut count = 0usize;
            let (mut min, mut max, mut last) = (f32::INFINITY, f32::NEG_INFINITY, 0.0f32);
            for (_, v) in s.window(start, end).filter(|(_, v)| v.is_finite()) {
                count += 1;
                min = min.min(v);
                max = max.max(v);
                last = v;
            }
            if count == 0 {
                out.push_str(&format!("{}: no data", s.name));
            } else {
                out.push_str(&format!(
                    "{}: {count} value{}, min {}, max {}, last {}",
                    s.name,
                    if count == 1 { "" } else { "s" },
                    self.fmt_value(f64::from(min)),
                    self.fmt_value(f64::from(max)),
                    self.fmt_value(f64::from(last)),
                ));
            }
        }
        out
    }

    /// Paint the chart. `range` is the visible window; the chips change it.
    pub fn show(self, ui: &mut Ui, range: &mut TimeRange) -> LineChartResponse {
        let theme = Theme::get(ui.ctx());
        let mut range_changed = false;
        let end = self.end();

        if self.legend || self.chips {
            ui.horizontal(|ui| {
                if self.legend {
                    for (i, s) in self.series.iter().enumerate() {
                        let style = series_style(&theme, i);
                        let (r, _) = ui.allocate_exact_size(
                            vec2(spacing_horizontal::L, line_height::BASE_300),
                            Sense::hover(),
                        );
                        paint_swatch(ui, r, style);
                        let latest = s
                            .latest()
                            .filter(|(_, v)| v.is_finite())
                            .map(|(_, v)| self.fmt_value(f64::from(v)));
                        ui.label(match latest {
                            Some(v) => format!("{} {v}", s.name),
                            None => s.name.clone(),
                        });
                        ui.add_space(spacing_horizontal::S);
                    }
                }
                if self.chips {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        for r in TimeRange::ALL.iter().rev() {
                            let mut on = *range == *r;
                            let chip = ToggleButton::new(&mut on, r.chip_label())
                                .size(ButtonSize::Small)
                                .id_salt((self.id, *r));
                            if ui.add(chip).clicked() && *range != *r {
                                *range = *r;
                                range_changed = true;
                            }
                        }
                    });
                }
            });
        }

        let start = end - range.seconds();
        let height = if self.height.is_finite() && self.height > 0.0 {
            self.height
        } else {
            DEFAULT_HEIGHT
        };
        let width = ui.available_width().max(spacing_horizontal::XL * 4.0);
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());

        // Value range over the visible samples: from min(0, low) to a nice top.
        let (mut lo, mut hi) = (0.0f64, f64::NEG_INFINITY);
        for s in self.series {
            for (_, v) in s.window(start, end).filter(|(_, v)| v.is_finite()) {
                lo = lo.min(f64::from(v));
                hi = hi.max(f64::from(v));
            }
        }
        if !hi.is_finite() || hi <= lo {
            hi = lo + 1.0;
        }
        let step = nice_step((hi - lo) / 4.0);
        let lo = (lo / step).floor() * step;
        let hi = (hi / step).ceil() * step;

        let font = font_id(TypeRamp::Caption1);
        let label_color = c(theme.color.neutral_foreground_3);
        let grid = Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2));
        let painter = ui.painter_at(rect);

        // Left margin fits the widest value label.
        let mut y_labels = Vec::new();
        let mut v = lo;
        while v <= hi + step * 0.5 {
            y_labels.push((v, self.fmt_value(v)));
            v += step;
        }
        let label_w = y_labels
            .iter()
            .map(|(_, s)| {
                ui.fonts_mut(|f| f.layout_no_wrap(s.clone(), font.clone(), label_color))
                    .size()
                    .x
            })
            .fold(0.0f32, f32::max);
        let plot = Rect::from_min_max(
            pos2(
                rect.left() + label_w + spacing_horizontal::S,
                rect.top() + spacing_horizontal::S,
            ),
            pos2(
                rect.right() - spacing_horizontal::S,
                rect.bottom() - line_height::BASE_200,
            ),
        );
        if plot.width() <= 0.0 || plot.height() <= 0.0 {
            describe(ui, &response, &self.summary(*range));
            return LineChartResponse {
                response,
                range_changed,
                hover: None,
            };
        }
        let span = range.seconds();
        let x_of = |t: f64| plot.left() + (((t - start) / span) as f32).clamp(0.0, 1.0) * plot.width();
        let y_of = |v: f64| {
            let u = ((v - lo) / (hi - lo)) as f32;
            plot.bottom() - if u.is_finite() { u.clamp(0.0, 1.0) } else { 0.0 } * plot.height()
        };

        if ui.is_rect_visible(rect) {
            for (v, s) in &y_labels {
                let y = y_of(*v);
                painter.hline(plot.x_range(), y, grid);
                painter.text(
                    pos2(plot.left() - spacing_horizontal::S, y),
                    Align2::RIGHT_CENTER,
                    s,
                    font.clone(),
                    label_color,
                );
            }
            let t_step = time_step(span);
            let mut ago = 0.0;
            while ago <= span + 0.5 {
                let x = x_of(end - ago);
                painter.vline(x, plot.y_range(), grid);
                let align = if ago >= span - 0.5 {
                    Align2::LEFT_TOP
                } else if ago == 0.0 {
                    Align2::RIGHT_TOP
                } else {
                    Align2::CENTER_TOP
                };
                painter.text(
                    pos2(x, plot.bottom() + stroke_width::THICK),
                    align,
                    self.fmt_time(ago),
                    font.clone(),
                    label_color,
                );
                ago += t_step;
            }

            let base_y = y_of(0.0f64.clamp(lo, hi));
            let mut points: Vec<Option<Pos2>> = Vec::new();
            for (i, s) in self.series.iter().enumerate() {
                let style = series_style(&theme, i);
                decimate(s.window(start, end), &x_of, &y_of, &mut points);
                let fill = s.fill && theme.kind != ThemeKind::HighContrast;
                for run in points.split(Option::is_none) {
                    let run: Vec<Pos2> = run.iter().flatten().copied().collect();
                    if fill && run.len() > 1 {
                        painter.add(area_mesh(&run, base_y, style.color.gamma_multiply(FILL_OPACITY)));
                    }
                    paint_run(&painter, run, style);
                }
            }
        }

        let mut hover = None;
        if let Some(p) = response.hover_pos().filter(|p| plot.contains(*p)) {
            let time = start + f64::from((p.x - plot.left()) / plot.width()) * span;
            let mut values = Vec::with_capacity(self.series.len());
            painter.vline(
                p.x,
                plot.y_range(),
                Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_1)),
            );
            for (i, s) in self.series.iter().enumerate() {
                let near = s.nearest(time, start, end).filter(|(_, v)| v.is_finite());
                if let Some((t, v)) = near {
                    let style = series_style(&theme, i);
                    painter.circle(
                        pos2(x_of(t), y_of(f64::from(v))),
                        spacing_horizontal::XS,
                        style.color,
                        Stroke::new(stroke_width::THIN, c(theme.color.neutral_background_1)),
                    );
                }
                values.push(near.map(|(_, v)| v));
            }
            hover = Some(ChartHover { time, values });
        }

        let response = match &hover {
            Some(h) => {
                let h = h.clone();
                let theme = theme.clone();
                let title = self.fmt_time(end - h.time);
                let rows: Vec<(usize, String, String)> = self
                    .series
                    .iter()
                    .zip(&h.values)
                    .enumerate()
                    .map(|(i, (s, v))| {
                        (
                            i,
                            s.name.clone(),
                            v.map_or_else(|| "–".to_owned(), |v| self.fmt_value(f64::from(v))),
                        )
                    })
                    .collect();
                response.on_hover_ui_at_pointer(move |ui| {
                    ui.strong(title);
                    for (i, name, value) in rows {
                        ui.horizontal(|ui| {
                            let (r, _) = ui.allocate_exact_size(
                                vec2(spacing_horizontal::L, line_height::BASE_300),
                                Sense::hover(),
                            );
                            paint_swatch(ui, r, series_style(&theme, i));
                            ui.label(format!("{name}: {value}"));
                        });
                    }
                })
            }
            None => response,
        };
        describe(ui, &response, &self.summary(*range));
        LineChartResponse {
            response,
            range_changed,
            hover,
        }
    }
}

/// Colour and dash of one series.
#[derive(Clone, Copy, Debug, PartialEq)]
struct SeriesStyle {
    color: Color32,
    dashed: bool,
}

/// The categorical order: palette tokens chosen to stay apart in light and dark themes.
fn series_style(theme: &Theme, index: usize) -> SeriesStyle {
    let t = &theme.color;
    if theme.kind == ThemeKind::HighContrast {
        let color = if index.is_multiple_of(2) {
            t.neutral_foreground_1
        } else {
            t.compound_brand_foreground_1
        };
        return SeriesStyle {
            color: c(color),
            dashed: (index / 2) % 2 == 1,
        };
    }
    let palette = [
        t.palette_blue_foreground_2,
        t.palette_dark_orange_foreground_1,
        t.palette_green_foreground_1,
        t.palette_berry_foreground_1,
        t.palette_teal_foreground_2,
        t.palette_marigold_foreground_1,
        t.palette_purple_foreground_2,
        t.palette_red_foreground_1,
    ];
    let color = palette
        .get(index % palette.len())
        .copied()
        .unwrap_or(t.compound_brand_foreground_1);
    SeriesStyle {
        color: c(color),
        dashed: false,
    }
}

fn paint_swatch(ui: &Ui, r: Rect, style: SeriesStyle) {
    let y = r.center().y;
    let pts = [pos2(r.left(), y), pos2(r.right(), y)];
    paint_run(ui.painter(), pts.to_vec(), style);
}

fn paint_run(painter: &egui::Painter, run: Vec<Pos2>, style: SeriesStyle) {
    let stroke = Stroke::new(stroke_width::THICK, style.color);
    match run.len() {
        0 => {}
        1 => {
            if let Some(p) = run.first() {
                painter.circle_filled(*p, stroke_width::THICK, style.color);
            }
        }
        _ if style.dashed => {
            painter.extend(Shape::dashed_line(
                &run,
                stroke,
                spacing_horizontal::S_NUDGE,
                spacing_horizontal::XS,
            ));
        }
        _ => {
            painter.add(Shape::line(run, PathStroke::from(stroke)));
        }
    }
}

/// A triangle strip from the line down to `base_y`.
fn area_mesh(run: &[Pos2], base_y: f32, color: Color32) -> Mesh {
    let mut mesh = Mesh::default();
    for (i, p) in run.iter().enumerate() {
        mesh.colored_vertex(*p, color);
        mesh.colored_vertex(pos2(p.x, base_y), color);
        if i > 0 {
            let k = (i as u32) * 2;
            mesh.add_triangle(k - 2, k - 1, k);
            mesh.add_triangle(k - 1, k, k + 1);
        }
    }
    mesh
}

/// Map samples to screen points, at most two per pixel column (the column's min and max, in
/// the order they occur). Non-finite values become `None` (a gap). Reuses `out`.
fn decimate(
    samples: impl Iterator<Item = (f64, f32)>,
    x_of: &impl Fn(f64) -> f32,
    y_of: &impl Fn(f64) -> f32,
    out: &mut Vec<Option<Pos2>>,
) {
    out.clear();
    // Current column: its x, and the first and second extreme as (order, value).
    let mut col: Option<(f32, f32, f32, bool)> = None; // (x, min, max, min_first)
    let flush = |col: &mut Option<(f32, f32, f32, bool)>, out: &mut Vec<Option<Pos2>>| {
        if let Some((x, min, max, min_first)) = col.take() {
            let (a, b) = if min_first { (min, max) } else { (max, min) };
            out.push(Some(pos2(x, y_of(f64::from(a)))));
            if a != b {
                out.push(Some(pos2(x, y_of(f64::from(b)))));
            }
        }
    };
    for (t, v) in samples {
        if !v.is_finite() {
            flush(&mut col, out);
            if out.last().is_some_and(Option::is_some) {
                out.push(None);
            }
            continue;
        }
        let x = x_of(t).floor();
        match &mut col {
            Some((cx, min, max, min_first)) if *cx == x => {
                if v < *min {
                    *min = v;
                    *min_first = false;
                }
                if v > *max {
                    *max = v;
                    *min_first = true;
                }
            }
            _ => {
                flush(&mut col, out);
                col = Some((x, v, v, true));
            }
        }
    }
    flush(&mut col, out);
}

/// 1, 2 or 5 times a power of ten, at least `raw`.
fn nice_step(raw: f64) -> f64 {
    if !raw.is_finite() || raw <= 0.0 {
        return 1.0;
    }
    let mag = 10f64.powf(raw.log10().floor());
    let n = raw / mag;
    let m = if n <= 1.0 {
        1.0
    } else if n <= 2.0 {
        2.0
    } else if n <= 5.0 {
        5.0
    } else {
        10.0
    };
    m * mag
}

/// A round time step giving about five to six ticks over `span` seconds.
fn time_step(span: f64) -> f64 {
    const STEPS: [f64; 10] = [
        10.0, 15.0, 30.0, 60.0, 120.0, 300.0, 600.0, 900.0, 1_800.0, 3_600.0,
    ];
    STEPS
        .iter()
        .copied()
        .find(|s| span / s <= 6.0)
        .unwrap_or_else(|| (span / 6.0 / 3_600.0).ceil() * 3_600.0)
}

/// Shortest readable form: integers without a fraction, others to two decimals.
fn short(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e12 {
        format!("{v:.0}")
    } else {
        format!("{v:.2}")
    }
}

/// Default time-axis label for `seconds` before the right edge: "now", "45 s", "5 min",
/// "1 h", "1 h 30 min".
///
/// ```
/// use fluent2_egui::controls::time_ago;
/// assert_eq!(time_ago(0.0), "now");
/// assert_eq!(time_ago(90.0), "1 min 30 s");
/// assert_eq!(time_ago(5_400.0), "1 h 30 min");
/// ```
#[must_use]
pub fn time_ago(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0.5 {
        return "now".into();
    }
    let s = seconds.round() as u64;
    let (h, m, s) = (s / 3600, (s / 60) % 60, s % 60);
    match (h, m, s) {
        (0, 0, s) => format!("{s} s"),
        (0, m, 0) => format!("{m} min"),
        (0, m, s) => format!("{m} min {s} s"),
        (h, 0, _) => format!("{h} h"),
        (h, m, _) => format!("{h} h {m} min"),
    }
}

/// Formats bytes per second with binary units: "512 B/s", "1.5 KiB/s", "3.2 MiB/s".
///
/// ```
/// use fluent2_egui::controls::bytes_per_second;
/// assert_eq!(bytes_per_second(512.0), "512 B/s");
/// assert_eq!(bytes_per_second(1536.0), "1.5 KiB/s");
/// ```
#[must_use]
pub fn bytes_per_second(v: f64) -> String {
    const UNITS: [&str; 5] = ["B/s", "KiB/s", "MiB/s", "GiB/s", "TiB/s"];
    if !v.is_finite() {
        return "–".into();
    }
    let mut v = v;
    let mut unit = UNITS[0];
    for u in &UNITS[1..] {
        if v.abs() < 1024.0 {
            break;
        }
        v /= 1024.0;
        unit = u;
    }
    if unit == UNITS[0] || v.fract() == 0.0 {
        format!("{v:.0} {unit}")
    } else {
        format!("{v:.1} {unit}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_drops_oldest_and_refuses_old_times() {
        let mut s = ChartSeries::new("s", 2);
        assert!(s.push(1.0, 1.0));
        assert!(s.push(2.0, 2.0));
        assert!(s.push(3.0, 3.0));
        assert_eq!(s.iter().collect::<Vec<_>>(), vec![(2.0, 2.0), (3.0, 3.0)]);
        assert!(!s.push(2.5, 0.0));
        assert!(!s.push(f64::NAN, 0.0));
        assert_eq!(ChartSeries::new("z", 0).capacity, 1);
    }

    #[test]
    fn window_and_nearest() {
        let mut s = ChartSeries::new("s", 10);
        for t in 0..10 {
            s.push(f64::from(t), t as f32);
        }
        assert_eq!(s.window(2.5, 5.0).count(), 3);
        assert_eq!(s.nearest(4.4, 0.0, 9.0), Some((4.0, 4.0)));
        assert_eq!(s.nearest(4.6, 0.0, 9.0), Some((5.0, 5.0)));
        assert_eq!(s.nearest(20.0, 10.0, 30.0), None, "nearest is out of range");
    }

    #[test]
    fn decimate_keeps_two_points_per_column_and_breaks_on_nan() {
        let samples = (0..1000).map(|i| {
            (
                f64::from(i) / 100.0,
                if i == 500 { f32::NAN } else { (i % 7) as f32 },
            )
        });
        let mut out = Vec::new();
        decimate(samples, &|t| t as f32, &|v| v as f32, &mut out);
        assert!(out.len() <= 2 * 11 + 1, "{}", out.len());
        assert_eq!(out.iter().filter(|p| p.is_none()).count(), 1);
    }

    #[test]
    fn nice_steps() {
        assert_eq!(nice_step(0.3), 0.5);
        assert_eq!(nice_step(7.0), 10.0);
        assert_eq!(nice_step(150.0), 200.0);
        assert_eq!(nice_step(f64::NAN), 1.0);
        assert_eq!(time_step(300.0), 60.0);
        assert_eq!(time_step(3_600.0), 600.0);
        assert_eq!(time_step(21_600.0), 3_600.0);
    }

    #[test]
    fn high_contrast_alternates_and_dashes() {
        let hc = Theme::high_contrast();
        let styles: Vec<_> = (0..4).map(|i| series_style(&hc, i)).collect();
        assert_ne!(styles[0].color, styles[1].color);
        assert!(!styles[0].dashed && !styles[1].dashed && styles[2].dashed && styles[3].dashed);
        let light = Theme::light(fluent2_tokens::FLUENT);
        assert_ne!(series_style(&light, 0).color, series_style(&light, 1).color);
    }
}
