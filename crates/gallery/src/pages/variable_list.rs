//! List with variable-height rows: 10,000 inbox-style messages of one to four lines. Rows are
//! measured as they scroll into view; "Insert at top" adds messages above the viewport and the
//! view stays still.

use fluent2_egui::controls::Button;
use fluent2_egui::data::list::{List, ListState};
use fluent2_egui::data::{RowId, SelectionMode};

use super::Page;

const ROWS: u64 = 10_000;

pub struct VariableListPage {
    /// Message ids, newest first. Inserting at the top takes ids above `ROWS`.
    ids: Vec<u64>,
    state: ListState,
    next_id: u64,
}

impl Default for VariableListPage {
    fn default() -> Self {
        Self {
            ids: (0..ROWS).collect(),
            state: ListState::default(),
            next_id: ROWS,
        }
    }
}

fn subject(id: u64) -> String {
    const SUBJECTS: [&str; 5] = [
        "Build finished",
        "Review requested",
        "Deploy to dev",
        "Weekly summary",
        "Question about the ribbon",
    ];
    let name = SUBJECTS.get((id % 5) as usize).copied().unwrap_or("Message");
    format!("#{id} · {name}")
}

/// Zero to three preview lines, so rows are one to four lines tall.
fn preview(id: u64) -> impl Iterator<Item = String> {
    let lines = (id * 7 + 3) % 4;
    (0..lines).map(move |n| match n {
        0 => "All checks passed on the merge queue; the Windows leg took 14 minutes.".to_owned(),
        1 => "Rows in this list are measured when shown and cached by their id.".to_owned(),
        _ => "Scroll anchoring keeps this row still when messages arrive above it.".to_owned(),
    })
}

impl Page for VariableListPage {
    fn name(&self) -> &'static str {
        "List rows"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("List with variable-height rows");
        ui.horizontal(|ui| {
            if ui.add(Button::new("Insert 5 at top")).clicked() {
                for _ in 0..5 {
                    self.ids.insert(0, self.next_id);
                    self.next_id += 1;
                }
            }
            ui.label(format!("{} messages", self.ids.len()));
        });
        ui.add_space(8.0);
        let ids = &self.ids;
        List::new("gallery_variable_list", ids.len())
            .label("Inbox")
            .selection_mode(SelectionMode::Multi)
            .row_id(|index| RowId(ids.get(index).copied().unwrap_or(u64::MAX)))
            .variable_height(56.0)
            .row_label(|index| ids.get(index).map(|id| subject(*id)).unwrap_or_default())
            .show(ui, &mut self.state, |cell, ui| {
                let id = cell.row_id.0;
                ui.strong(subject(id));
                for line in preview(id) {
                    ui.label(line);
                }
            });
    }
}
