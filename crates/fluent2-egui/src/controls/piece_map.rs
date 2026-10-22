//! PieceMap: every piece as a cell in a grid, with a hover read-out and a legend.
//!
//! Cells are `compoundBrandBackground` when held, `brandBackground2` when downloading and
//! `neutralBackground5` when missing. A cell that covers several pieces (more pieces than
//! `max_cells`) shades by the mean instead. Hovering a cell names its piece, or its range and
//! the share held. Indices are zero-based, as in a torrent's piece numbering. The AccessKit node
//! is an image labelled with the percent done, as for [`PieceBar`](super::PieceBar).

use egui::{Color32, CornerRadius, Pos2, Rect, Response, Sense, Ui, Vec2, Widget};
use fluent2_tokens::generated::global::{border_radius, spacing_horizontal};

use super::piece_bar::{Bucket, PieceState, Source, blend, bucket_pieces, done_label, finite_positive};
use super::sparkline::{default_size, describe};
use crate::theme::{Theme, c};

/// Side of one cell.
const CELL: f32 = spacing_horizontal::M;
/// Gap between cells.
const GAP: f32 = spacing_horizontal::XXS;
/// Distance from one cell's origin to the next.
const STEP: f32 = CELL + GAP;
/// Most cells drawn before pieces share cells. Keeps the grid bounded for huge torrents.
const DEFAULT_MAX_CELLS: usize = 4096;

fn state_colour(theme: &Theme, state: PieceState) -> Color32 {
    match state {
        PieceState::Have => c(theme.color.compound_brand_background),
        PieceState::Downloading => c(theme.color.brand_background_2),
        PieceState::Missing => c(theme.color.neutral_background_5),
    }
}

fn state_name(state: PieceState) -> &'static str {
    match state {
        PieceState::Have => "Have",
        PieceState::Downloading => "Downloading",
        PieceState::Missing => "Missing",
    }
}

/// The cell under `pos`, if it is inside the grid and `cells` cells exist.
fn cell_at(rect: Rect, pos: Pos2, cols: usize, cells: usize) -> Option<usize> {
    let col = ((pos.x - rect.left()) / STEP).floor();
    let row = ((pos.y - rect.top()) / STEP).floor();
    if col < 0.0 || row < 0.0 {
        return None;
    }
    let (col, row) = (col as usize, row as usize);
    (col < cols).then_some(row * cols + col).filter(|&i| i < cells)
}

/// The hover read-out for one cell: a single piece by name, or a range by its share held.
fn hover_text(bucket: Bucket, states: &[PieceState]) -> String {
    if bucket.len == 1 {
        let state = states.get(bucket.start).copied().unwrap_or_default();
        format!("Piece {}: {}", bucket.start, state_name(state))
    } else {
        let last = bucket.start + bucket.len - 1;
        format!(
            "Pieces {}–{}: {:.0}% held",
            bucket.start,
            last,
            bucket.mean * 100.0
        )
    }
}

fn legend(ui: &mut Ui, theme: &Theme, states: &[PieceState]) {
    let count = |wanted: PieceState| states.iter().filter(|s| **s == wanted).count();
    ui.horizontal_wrapped(|ui| {
        for state in [PieceState::Have, PieceState::Downloading, PieceState::Missing] {
            let (swatch, _) = ui.allocate_exact_size(Vec2::splat(CELL), Sense::hover());
            ui.painter().rect_filled(
                swatch,
                CornerRadius::same(border_radius::SMALL as u8),
                state_colour(theme, state),
            );
            ui.label(format!("{} ({})", state_name(state), count(state)));
        }
    });
}

/// A grid with one cell per piece, a hover read-out and a legend with counts.
///
/// ```
/// use fluent2_egui::controls::{PieceMap, PieceState};
/// # fn demo(ui: &mut egui::Ui, states: &[PieceState]) {
/// ui.add(PieceMap::new(states).label("Pieces").legend(true));
/// # }
/// ```
#[derive(Clone, Debug)]
pub struct PieceMap<'a> {
    states: &'a [PieceState],
    label: String,
    width: Option<f32>,
    max_cells: usize,
    legend: bool,
}

impl<'a> PieceMap<'a> {
    /// A grid over piece states, one cell per piece in order.
    #[must_use]
    pub fn new(states: &'a [PieceState]) -> Self {
        Self {
            states,
            label: "Pieces".into(),
            width: None,
            max_cells: DEFAULT_MAX_CELLS,
            legend: true,
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

    /// Most cells drawn before pieces share cells. Defaults to 4096. Zero is treated as one.
    #[must_use]
    pub fn max_cells(mut self, max_cells: usize) -> Self {
        self.max_cells = max_cells.max(1);
        self
    }

    /// Show the legend with counts below the grid. Default on.
    #[must_use]
    pub fn legend(mut self, on: bool) -> Self {
        self.legend = on;
        self
    }
}

impl Widget for PieceMap<'_> {
    fn ui(self, ui: &mut Ui) -> Response {
        let theme = Theme::get(ui.ctx());
        let source = Source::States(self.states);
        let width = finite_positive(
            self.width.unwrap_or_else(|| ui.available_width()),
            default_size().x * 4.0,
        );
        let cols = ((width + GAP) / STEP).floor().max(1.0) as usize;
        let cells = self.max_cells.min(self.states.len());
        let rows = cells.div_ceil(cols);
        let height = if rows == 0 {
            0.0
        } else {
            rows as f32 * CELL + (rows - 1) as f32 * GAP
        };
        let label = done_label(&self.label, source);
        let show_legend = self.legend;

        ui.vertical(|ui| {
            let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
            let buckets = bucket_pieces(ui, response.id.with("buckets"), source, cells);
            if ui.is_rect_visible(rect) {
                let painter = ui.painter_at(rect);
                for (i, bucket) in buckets.iter().enumerate() {
                    let origin = Pos2::new(
                        rect.left() + (i % cols) as f32 * STEP,
                        rect.top() + (i / cols) as f32 * STEP,
                    );
                    let colour = if bucket.len == 1 {
                        let state = self.states.get(bucket.start).copied().unwrap_or_default();
                        state_colour(&theme, state)
                    } else {
                        blend(&theme, bucket.mean)
                    };
                    painter.rect_filled(
                        Rect::from_min_size(origin, Vec2::splat(CELL)),
                        CornerRadius::same(border_radius::SMALL as u8),
                        colour,
                    );
                }
            }

            let mut response = response;
            if let Some(pos) = response.hover_pos()
                && let Some(bucket) =
                    cell_at(rect, pos, cols, buckets.len()).and_then(|i| buckets.get(i).copied())
            {
                let text = hover_text(bucket, self.states);
                response = response.on_hover_ui(|ui| {
                    ui.label(text);
                });
            }

            if show_legend {
                legend(ui, &theme, self.states);
            }
            describe(ui, &response, &label);
            response
        })
        .inner
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cells_wrap_by_column_count() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(100.0, 100.0));
        // Three columns: the first cell of row 2 is index 6.
        assert_eq!(cell_at(rect, Pos2::new(0.5, STEP * 2.0 + 0.5), 3, 10), Some(6));
        assert_eq!(cell_at(rect, Pos2::new(STEP * 2.0 + 0.5, 0.5), 3, 10), Some(2));
        // Past the last cell, and past the last column, there is nothing.
        assert_eq!(
            cell_at(rect, Pos2::new(STEP * 2.0 + 0.5, STEP * 3.0 + 0.5), 3, 10),
            None
        );
        assert_eq!(cell_at(rect, Pos2::new(STEP * 3.0 + 0.5, 0.5), 3, 10), None);
        assert_eq!(cell_at(rect, Pos2::new(-1.0, 0.5), 3, 10), None);
    }

    #[test]
    fn hover_names_a_piece_or_a_range() {
        let states = [PieceState::Have, PieceState::Downloading, PieceState::Missing];
        let single = Bucket {
            mean: 1.0,
            start: 0,
            len: 1,
        };
        assert_eq!(hover_text(single, &states), "Piece 0: Have");
        let range = Bucket {
            mean: 0.5,
            start: 1,
            len: 2,
        };
        assert_eq!(hover_text(range, &states), "Pieces 1–2: 50% held");
    }

    #[test]
    fn legend_counts_each_state() {
        let states = [
            PieceState::Have,
            PieceState::Have,
            PieceState::Downloading,
            PieceState::Missing,
        ];
        let count = |wanted| states.iter().filter(|s| **s == wanted).count();
        assert_eq!(count(PieceState::Have), 2);
        assert_eq!(count(PieceState::Downloading), 1);
        assert_eq!(count(PieceState::Missing), 1);
    }
}
