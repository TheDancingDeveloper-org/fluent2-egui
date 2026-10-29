//! Grouped virtual list: date headers stick while their messages scroll, and each group
//! expands and collapses.

use fluent2_egui::data::RowId;
use fluent2_egui::data::grouped_list::{Group, GroupedList, GroupedListState};

use super::Page;

pub struct GroupedListPage {
    state: GroupedListState,
}

impl Default for GroupedListPage {
    fn default() -> Self {
        Self {
            state: GroupedListState::default(),
        }
    }
}

fn groups() -> Vec<Group> {
    ["Today", "Yesterday", "Last week", "Last month"]
        .into_iter()
        .enumerate()
        .map(|(index, label)| Group {
            id: RowId(index as u64 + 1),
            label: label.to_owned(),
            row_count: 12,
        })
        .collect()
}

impl Page for GroupedListPage {
    fn name(&self) -> &'static str {
        "Grouped list"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Grouped list");
        ui.label("Headers stick while their group is on screen. Click a header, or press Left and Right, to collapse and expand.");
        ui.add_space(8.0);
        let groups = groups();
        GroupedList::new("gallery_grouped_list", &groups)
            .label("Messages by date")
            .estimate(28.0)
            .show(ui, &mut self.state, |cell, ui| {
                ui.label(format!("Thread {}", cell.row + 1));
            });
    }
}
