//! Pieces page: PieceBar strips over states, a bitfield and availability, and a PieceMap grid.

use fluent2_egui::controls::{PieceBar, PieceMap, PieceState};

use super::Page;

/// Pieces in the large sample torrent, to show bucketing past the strip's pixel width.
const HUGE_PIECES: usize = 200_000;

/// Gallery page for PieceBar and PieceMap.
pub struct PiecesPage {
    states: Vec<PieceState>,
    bitfield: Vec<u8>,
    counts: Vec<u32>,
}

impl Default for PiecesPage {
    fn default() -> Self {
        // A fixed xorshift stream, so the sample looks the same on every run.
        let mut seed: u64 = 0x9e37_79b9_7f4a_7c15;
        let mut next = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let states = (0..1_000)
            .map(|_| match next() % 10 {
                0..=5 => PieceState::Have,
                6 => PieceState::Downloading,
                _ => PieceState::Missing,
            })
            .collect();
        let bitfield = (0..HUGE_PIECES.div_ceil(8)).map(|_| next() as u8).collect();
        let counts = (0..2_000).map(|_| (next() % 12) as u32).collect();
        Self {
            states,
            bitfield,
            counts,
        }
    }
}

impl Page for PiecesPage {
    fn name(&self) -> &'static str {
        "Pieces"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Download overview");
        ui.add(PieceBar::states(&self.states).label("Download"));

        ui.add_space(16.0);
        ui.strong("Bitfield, 200,000 pieces");
        ui.add(
            PieceBar::bitfield(&self.bitfield, HUGE_PIECES)
                .label("Torrent with 200,000 pieces")
                .height(12.0),
        );

        ui.add_space(16.0);
        ui.strong("Availability");
        ui.add(PieceBar::availability(&self.counts).label("Availability"));

        ui.add_space(16.0);
        ui.strong("Piece map");
        ui.add(PieceMap::new(&self.states).label("Download"));
    }
}
