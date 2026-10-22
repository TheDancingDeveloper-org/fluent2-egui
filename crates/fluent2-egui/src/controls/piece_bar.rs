//! PieceBar: the whole piece set as one strip, for a torrent overview or an availability heat bar.
//!
//! The strip reads one value per piece, from a [`PieceState`] slice, a BitTorrent bitfield or a
//! replication count. When pieces outnumber pixels, neighbouring pieces share a bucket and the
//! bucket shows their mean. [`PieceMap`](super::PieceMap) draws the same pieces as a grid.
//!
//! The track is `neutralBackground5` and the held share is `compoundBrandBackground`, so
//! availability reads as a ramp from track to brand. A downloading piece counts half. High
//! contrast maps the tokens to system colours. The AccessKit node is an image labelled with the
//! percent done.

use std::ops::Range;

use egui::{Color32, CornerRadius, Id, Pos2, Rect, Response, Sense, Stroke, StrokeKind, Ui, Vec2, Widget};
use fluent2_tokens::generated::global::{border_radius, stroke_width};

use super::sparkline::{default_size, describe};
use crate::theme::{Theme, c};

/// The state of one piece.
///
/// ```
/// use fluent2_egui::controls::PieceState;
/// assert_eq!(PieceState::default(), PieceState::Missing);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum PieceState {
    /// Not held yet.
    #[default]
    Missing,
    /// Requested or being written. Counts as half a piece in bars.
    Downloading,
    /// Held and verified.
    Have,
}

impl PieceState {
    /// How much of a piece this state counts as held: 0, 0.5 or 1.
    pub(super) fn weight(self) -> f32 {
        match self {
            Self::Missing => 0.0,
            Self::Downloading => 0.5,
            Self::Have => 1.0,
        }
    }
}

/// Where a [`PieceBar`] or [`PieceMap`](super::PieceMap) reads its values from.
#[derive(Clone, Copy, Debug)]
pub(super) enum Source<'a> {
    States(&'a [PieceState]),
    Bitfield { bytes: &'a [u8], len: usize },
    Availability { counts: &'a [u32], max: u32 },
}

impl Source<'_> {
    /// Number of pieces.
    pub(super) fn len(self) -> usize {
        match self {
            Self::States(states) => states.len(),
            Self::Bitfield { len, .. } => len,
            Self::Availability { counts, .. } => counts.len(),
        }
    }

    /// Value of piece `i` in `0..=1`. A missing index or byte reads as 0, so short input never
    /// panics. Availability is the count over the largest count, or 0 when that is 0.
    pub(super) fn value(self, i: usize) -> f32 {
        match self {
            Self::States(states) => states.get(i).map_or(0.0, |s| s.weight()),
            Self::Bitfield { bytes, len } => {
                let held = i < len
                    && bytes
                        .get(i / 8)
                        .is_some_and(|byte| (byte >> (7 - i % 8)) & 1 == 1);
                f32::from(u8::from(held))
            }
            Self::Availability { counts, max } => counts
                .get(i)
                .map_or(0.0, |&n| if max == 0 { 0.0 } else { n as f32 / max as f32 }),
        }
    }

    /// Pieces held in full, or `None` for availability, which has no notion of held.
    pub(super) fn held(self) -> Option<usize> {
        match self {
            Self::States(states) => Some(states.iter().filter(|s| **s == PieceState::Have).count()),
            Self::Bitfield { .. } => Some((0..self.len()).filter(|&i| self.value(i) >= 1.0).count()),
            Self::Availability { .. } => None,
        }
    }
}

/// One cell of a bar or map: the mean value of the pieces it covers, where they start, and
/// how many there are.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Bucket {
    pub(super) mean: f32,
    pub(super) start: usize,
    pub(super) len: usize,
}

/// Split `count` pieces into at most `cells` contiguous ranges of near-equal size. Empty when
/// there is nothing to show. Pure, so the layout can be cached.
pub(super) fn bucket_ranges(count: usize, cells: usize) -> Vec<Range<usize>> {
    if count == 0 || cells == 0 {
        return Vec::new();
    }
    let cells = cells.min(count);
    // u64 arithmetic: `k * count` overflows usize on 32-bit targets such as wasm32.
    let at = |k: usize| (k as u64 * count as u64 / cells as u64) as usize;
    (0..cells).map(|k| at(k)..at(k + 1)).collect()
}

/// The mean of each range. Pure, for tests and for [`bucket_pieces`].
pub(super) fn bucket_means(source: Source<'_>, ranges: &[Range<usize>]) -> Vec<Bucket> {
    ranges
        .iter()
        .map(|r| {
            let sum: f32 = r.clone().map(|i| source.value(i)).sum();
            Bucket {
                mean: sum / r.len() as f32,
                start: r.start,
                len: r.len(),
            }
        })
        .collect()
}

#[derive(Clone, Default)]
struct BucketCache {
    key: (usize, usize),
    ranges: Vec<Range<usize>>,
}

/// Buckets for `cells` cells. The range layout is cached per (piece count, cell count) under
/// `id`; the means are recomputed each call, in O(pieces).
pub(super) fn bucket_pieces(ui: &mut Ui, id: Id, source: Source<'_>, cells: usize) -> Vec<Bucket> {
    let count = source.len();
    let cells = cells.min(count);
    ui.data_mut(|data| {
        let cache = data.get_temp_mut_or_default::<BucketCache>(id);
        if cache.key != (count, cells) {
            cache.key = (count, cells);
            cache.ranges = bucket_ranges(count, cells);
        }
        bucket_means(source, &cache.ranges)
    })
}

/// The track at mean 0 and the held brand at mean 1, blended by `mean`.
pub(super) fn blend(theme: &Theme, mean: f32) -> Color32 {
    c(theme.color.neutral_background_5)
        .lerp_to_gamma(c(theme.color.compound_brand_background), mean.clamp(0.0, 1.0))
}

/// The accessible summary: `"Pieces: 50% done (2 of 4 pieces)"`, or the availability range for
/// a replication count, or `"Pieces: no pieces"`.
pub(super) fn done_label(name: &str, source: Source<'_>) -> String {
    let count = source.len();
    if count == 0 {
        return format!("{name}: no pieces");
    }
    match source {
        Source::Availability { counts, .. } => {
            let min = counts.iter().min().copied().unwrap_or(0);
            let max = counts.iter().max().copied().unwrap_or(0);
            format!("{name}: {count} pieces, availability {min} to {max}")
        }
        _ => {
            let held = source.held().unwrap_or(0);
            let pct = held as f32 * 100.0 / count as f32;
            format!("{name}: {pct:.0}% done ({held} of {count} pieces)")
        }
    }
}

/// `value` if it is finite and positive, else `fallback`.
pub(super) fn finite_positive(value: f32, fallback: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        fallback
    }
}

/// A strip with one segment per piece, or per bucket when pieces outnumber pixels.
///
/// ```
/// use fluent2_egui::controls::PieceBar;
/// # fn demo(ui: &mut egui::Ui, bitfield: &[u8]) {
/// ui.add(PieceBar::bitfield(bitfield, 2_000).label("Download"));
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct PieceBar<'a> {
    source: Source<'a>,
    label: String,
    width: Option<f32>,
    height: Option<f32>,
}

impl<'a> PieceBar<'a> {
    /// A strip over piece states, one per piece in order.
    #[must_use]
    pub fn states(states: &'a [PieceState]) -> Self {
        Self::with_source(Source::States(states))
    }

    /// A strip over a BitTorrent bitfield: `piece_count` bits, most significant bit first. Bits
    /// past the end of `bytes` read as missing.
    #[must_use]
    pub fn bitfield(bytes: &'a [u8], piece_count: usize) -> Self {
        Self::with_source(Source::Bitfield {
            bytes,
            len: piece_count,
        })
    }

    /// An availability heat strip: one replication count per piece, shaded from the track
    /// (least available) to brand (most available, relative to the largest count).
    #[must_use]
    pub fn availability(counts: &'a [u32]) -> Self {
        let max = counts.iter().copied().max().unwrap_or(0);
        Self::with_source(Source::Availability { counts, max })
    }

    fn with_source(source: Source<'a>) -> Self {
        Self {
            source,
            label: "Pieces".into(),
            width: None,
            height: None,
        }
    }

    /// Accessible name, used in the summary label. Defaults to "Pieces".
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Width. Defaults to the space available. A non-finite or non-positive width falls back to
    /// the available space.
    #[must_use]
    pub fn width(mut self, width: f32) -> Self {
        self.width = Some(width);
        self
    }

    /// Height. Defaults to one body line (20). A non-finite or non-positive height falls back to
    /// the default.
    #[must_use]
    pub fn height(mut self, height: f32) -> Self {
        self.height = Some(height);
        self
    }
}

impl Widget for PieceBar<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let width = finite_positive(
            self.width.unwrap_or_else(|| ui.available_width()),
            default_size().x,
        );
        let height = finite_positive(self.height.unwrap_or(default_size().y), default_size().y);
        let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        let cells = rect.width().floor().max(1.0) as usize;
        let buckets = bucket_pieces(ui, response.id.with("buckets"), self.source, cells);
        if ui.is_rect_visible(rect) && !buckets.is_empty() {
            let painter = ui.painter_at(rect);
            let cell_width = rect.width() / buckets.len() as f32;
            for (i, bucket) in buckets.iter().enumerate() {
                let x = rect.left() + i as f32 * cell_width;
                let cell =
                    Rect::from_min_max(Pos2::new(x, rect.top()), Pos2::new(x + cell_width, rect.bottom()));
                painter.rect_filled(cell, CornerRadius::ZERO, blend(&theme, bucket.mean));
            }
            painter.rect_stroke(
                rect,
                CornerRadius::same(border_radius::SMALL as u8),
                Stroke::new(stroke_width::THIN, c(theme.color.neutral_stroke_2)),
                StrokeKind::Inside,
            );
        }
        describe(ui, &response, &done_label(&self.label, self.source));
        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitfield_reads_most_significant_bit_first_and_pads_missing_bits() {
        let bytes = [0b1010_0000, 0b0000_0001];
        let source = Source::Bitfield {
            bytes: &bytes,
            len: 16,
        };
        let bits: Vec<f32> = (0..16).map(|i| source.value(i)).collect();
        // Piece 0 is the top bit of byte 0; piece 15 is the bottom bit of byte 1.
        assert_eq!(
            bits,
            [
                1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0
            ]
        );
        // Bits past the bytes, and past `len`, read as missing.
        let short = Source::Bitfield {
            bytes: &[0xff],
            len: 12,
        };
        assert_eq!(short.value(7), 1.0);
        assert_eq!(short.value(8), 0.0);
        assert_eq!(short.value(40), 0.0);
    }

    #[test]
    fn bucket_ranges_are_contiguous_and_cover_every_piece() {
        assert_eq!(bucket_ranges(10, 3), vec![0..3, 3..6, 6..10]);
        assert_eq!(
            bucket_ranges(2, 600),
            vec![0..1, 1..2],
            "never more cells than pieces"
        );
        assert!(bucket_ranges(0, 600).is_empty());
        assert!(bucket_ranges(5, 0).is_empty());

        let ranges = bucket_ranges(200_000, 600);
        assert_eq!(ranges.len(), 600);
        assert_eq!(ranges.first().map(|r| r.start), Some(0));
        assert_eq!(ranges.last().map(|r| r.end), Some(200_000));
        assert!(ranges.windows(2).all(|w| w[0].end == w[1].start));
        assert!(ranges.iter().all(|r| r.len() == 333 || r.len() == 334));
    }

    #[test]
    fn bucket_means_average_each_range() {
        let states = [
            PieceState::Have,
            PieceState::Missing,
            PieceState::Downloading,
            PieceState::Have,
        ];
        let buckets = bucket_means(Source::States(&states), &bucket_ranges(4, 2));
        assert_eq!(
            buckets,
            vec![
                Bucket {
                    mean: 0.5,
                    start: 0,
                    len: 2
                },
                Bucket {
                    mean: 0.75,
                    start: 2,
                    len: 2
                },
            ]
        );
    }

    #[test]
    fn availability_scales_by_the_largest_count_and_never_divides_by_zero() {
        let counts = [1, 4, 2];
        let source = Source::Availability {
            counts: &counts,
            max: 4,
        };
        assert_eq!(source.value(1), 1.0);
        assert_eq!(source.value(0), 0.25);
        let zeros = Source::Availability {
            counts: &[0, 0],
            max: 0,
        };
        assert_eq!(zeros.value(0), 0.0);
        assert!(zeros.held().is_none());
    }

    #[test]
    fn percent_done_counts_only_held_pieces() {
        let states = [
            PieceState::Have,
            PieceState::Downloading,
            PieceState::Missing,
            PieceState::Have,
        ];
        assert_eq!(
            done_label("Pieces", Source::States(&states)),
            "Pieces: 50% done (2 of 4 pieces)"
        );
        assert_eq!(done_label("Pieces", Source::States(&[])), "Pieces: no pieces");
        let counts = [3, 1, 2];
        assert_eq!(
            done_label(
                "Rarity",
                Source::Availability {
                    counts: &counts,
                    max: 3
                }
            ),
            "Rarity: 3 pieces, availability 1 to 3"
        );
    }

    #[test]
    fn two_hundred_thousand_pieces_fold_into_one_bucket_per_pixel() {
        // The acceptance case: 200,000 pieces in a 600 px strip.
        let bytes = vec![0xaa_u8; 25_000];
        let source = Source::Bitfield {
            bytes: &bytes,
            len: 200_000,
        };
        let buckets = bucket_means(source, &bucket_ranges(200_000, 600));
        assert_eq!(buckets.len(), 600);
        assert!(buckets.iter().all(|b| (b.mean - 0.5).abs() < 0.01));
        assert_eq!(buckets.iter().map(|b| b.len).sum::<usize>(), 200_000);
    }
}
