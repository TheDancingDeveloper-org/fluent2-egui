//! EditableList page: text rows and number rows. The page applies each event to its rows, as an
//! app would.

use fluent2_egui::data::editable_list::{EditableKind, EditableList, EditableListEvent, EditableListState};

use super::Page;

/// Gallery page for EditableList.
pub struct EditableListPage {
    filters: Vec<String>,
    filters_state: EditableListState,
    speeds: Vec<String>,
    speeds_state: EditableListState,
    last: String,
}

impl Default for EditableListPage {
    fn default() -> Self {
        Self {
            filters: vec!["10.0.0.0/8".to_owned(), "192.168.0.0/16".to_owned()],
            filters_state: EditableListState::default(),
            speeds: vec!["1.5".to_owned(), "-3".to_owned()],
            speeds_state: EditableListState::default(),
            last: "Nothing changed yet".to_owned(),
        }
    }
}

/// Apply one committed change to the rows it belongs to.
fn apply(rows: &mut Vec<String>, event: &EditableListEvent, last: &mut String) {
    match event {
        EditableListEvent::Added(value) => rows.push(value.clone()),
        EditableListEvent::Edited { index, value } => {
            if let Some(row) = rows.get_mut(*index) {
                *row = value.clone();
            }
        }
        EditableListEvent::Removed { index } => {
            if *index < rows.len() {
                rows.remove(*index);
            }
        }
    }
    *last = format!("{event:?}");
}

impl Page for EditableListPage {
    fn name(&self) -> &'static str {
        "Editable list"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.strong("Text rows");
        let response = EditableList::new("gallery_filters", &self.filters)
            .label("IP filters")
            .show(ui, &mut self.filters_state);
        for event in &response.events {
            apply(&mut self.filters, event, &mut self.last);
        }

        ui.add_space(16.0);
        ui.strong("Number rows");
        let response = EditableList::new("gallery_speeds", &self.speeds)
            .label("Alt-speed limits")
            .kind(EditableKind::Number)
            .show(ui, &mut self.speeds_state);
        for event in &response.events {
            apply(&mut self.speeds, event, &mut self.last);
        }

        ui.add_space(16.0);
        ui.label(format!("Last change: {}", self.last));
    }
}
