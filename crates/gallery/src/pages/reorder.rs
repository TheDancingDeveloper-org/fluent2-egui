//! Reorder page: a three-column board and a horizontal strip, both reordered by drag or by
//! Alt+arrow keys. The page applies every move it is given and shows the last one.

use fluent2_egui::controls::Switch;
use fluent2_egui::data::reorder::{MoveSource, Reorder, ReorderAxis, ReorderMove};
use fluent2_egui::surfaces::Card;

use super::Page;

/// Width of one board column.
const COLUMN_WIDTH: f32 = 180.0;
/// Minimum height of a column, so an empty column still takes drops.
const COLUMN_HEIGHT: f32 = 220.0;

/// Gallery page for the Reorder helper.
pub struct ReorderPage {
    titles: [&'static str; 3],
    columns: Vec<Vec<String>>,
    strip: Vec<String>,
    read_only: bool,
    last: Option<ReorderMove>,
}

impl Default for ReorderPage {
    fn default() -> Self {
        let cards = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
        Self {
            titles: ["To do", "Doing", "Done"],
            columns: vec![
                cards(&["Write spec", "Draw icons", "Plan release"]),
                cards(&["Review PR"]),
                Vec::new(),
            ],
            strip: cards(&["Home", "Insert", "Layout", "View"]),
            read_only: false,
            last: None,
        }
    }
}

impl Page for ReorderPage {
    fn name(&self) -> &'static str {
        "Reorder"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Reorder");
        ui.label(
            "Drag a card to reorder it or move it to another column; Escape cancels. With a card \
             focused, Alt+Up/Down moves it within its column and Alt+Left/Right to the next \
             column. The helper only reports moves; this page applies them.",
        );
        ui.add_space(8.0);
        ui.add(Switch::new(&mut self.read_only, "Read-only"));
        ui.add_space(8.0);

        let mut board = Reorder::new("gallery_board");
        ui.add_enabled_ui(!self.read_only, |ui| {
            ui.horizontal_top(|ui| {
                for (col, cards) in self.columns.iter().enumerate() {
                    ui.vertical(|ui| {
                        ui.set_width(COLUMN_WIDTH);
                        ui.strong(self.titles.get(col).copied().unwrap_or_default());
                        board.container(ui, col, ReorderAxis::Vertical, |list, ui| {
                            ui.set_min_size(egui::vec2(COLUMN_WIDTH, COLUMN_HEIGHT));
                            for card in cards {
                                list.item(ui, card, card, |ui| {
                                    Card::new().show(ui, |ui| {
                                        ui.set_width(COLUMN_WIDTH - 24.0);
                                        ui.label(card);
                                    });
                                });
                            }
                        });
                    });
                }
            });
        });
        if let Some(mv) = board.finish(ui).moved
            && mv.apply_between(&mut self.columns)
        {
            self.last = Some(mv);
        }

        ui.add_space(12.0);
        ui.strong("Horizontal strip");
        let mut strip = Reorder::new("gallery_strip");
        ui.add_enabled_ui(!self.read_only, |ui| {
            strip.container(ui, 0, ReorderAxis::Horizontal, |list, ui| {
                for tab in &self.strip {
                    list.item(ui, tab, tab, |ui| {
                        ui.label(tab);
                    });
                }
            });
        });
        if let Some(mv) = strip.finish(ui).moved
            && mv.apply(&mut self.strip)
        {
            self.last = Some(mv);
        }

        ui.add_space(8.0);
        ui.label(match self.last {
            Some(mv) => format!(
                "Last move ({}): column {} row {} to column {} row {}",
                match mv.source {
                    MoveSource::Pointer => "pointer",
                    MoveSource::Keyboard => "keyboard",
                },
                mv.from.container,
                mv.from.index,
                mv.to.container,
                mv.to.index,
            ),
            None => "No move yet.".to_owned(),
        });
    }
}
