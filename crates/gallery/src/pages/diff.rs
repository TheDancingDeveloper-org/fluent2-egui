//! DiffView: a generated 10,000-line source file against an edited copy, inline or side by
//! side. The model is built once; the view paints only the rows on screen.

use fluent2_egui::controls::ToggleButton;
use fluent2_egui::data::diff::{DEFAULT_CONTEXT, DiffMode, DiffModel, DiffView};

use super::Page;

const LINES: usize = 10_000;

pub struct DiffPage {
    model: DiffModel,
    side_by_side: bool,
    whole_file: bool,
    built_whole: bool,
    old: String,
    new: String,
}

impl Default for DiffPage {
    fn default() -> Self {
        let (old, new) = sample();
        Self {
            model: DiffModel::new(&old, &new),
            side_by_side: false,
            whole_file: true,
            built_whole: false,
            old,
            new,
        }
    }
}

/// A Rust-looking file and an edit of it: renamed values, inserted and deleted lines.
fn sample() -> (String, String) {
    let mut old = String::new();
    let mut new = String::new();
    for i in 0..LINES {
        let line = match i % 8 {
            0 => format!("fn step_{i}(input: &str) -> usize {{"),
            1 => format!("\tlet count = input.len() + {i};"),
            2 => "\tlet doubled = count * 2;".to_owned(),
            3 => "\tlog::debug!(\"doubled {doubled}\");".to_owned(),
            4 => "\tdoubled".to_owned(),
            5 => "}".to_owned(),
            _ => String::new(),
        };
        old.push_str(&line);
        old.push('\n');
        match i % 97 {
            11 => new.push_str(&line.replace("count", "total")),
            33 => new.push_str(&line.replace("2", "3")),
            50 => continue,
            70 => {
                new.push_str(&line);
                new.push_str("\n\t// inserted by the edit");
            }
            _ => new.push_str(&line),
        }
        new.push('\n');
    }
    (old, new)
}

impl Page for DiffPage {
    fn name(&self) -> &'static str {
        "Diff"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("DiffView");
        ui.horizontal(|ui| {
            ui.add(ToggleButton::new(&mut self.side_by_side, "Side by side"));
            ui.add(ToggleButton::new(&mut self.whole_file, "Whole file"));
            ui.label(format!(
                "{LINES} lines · {} hunks · +{} −{}",
                self.model.hunks().len(),
                self.model.additions(),
                self.model.removals()
            ));
        });
        if self.whole_file != self.built_whole {
            let context = if self.whole_file {
                usize::MAX
            } else {
                DEFAULT_CONTEXT
            };
            self.model = DiffModel::with_context(&self.old, &self.new, context);
            self.built_whole = self.whole_file;
        }
        ui.add_space(8.0);
        let mode = if self.side_by_side {
            DiffMode::SideBySide
        } else {
            DiffMode::Inline
        };
        DiffView::new("gallery_diff")
            .label("Gallery diff")
            .mode(mode)
            .show(ui, &self.model);
    }
}
