//! The committed Fluent System Icons subset, filterable.

use fluent2_egui::Icon;
use fluent2_icons::Icon as IconName;

use super::Page;

/// Every icon in the committed subset, with a name filter.
#[derive(Default)]
pub struct IconsPage {
    filter: String,
}

impl Page for IconsPage {
    fn name(&self) -> &'static str {
        "Icons"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Icons");
        ui.horizontal(|ui| {
            ui.label("Filter");
            let filter = ui.text_edit_singleline(&mut self.filter);
            ui.ctx().accesskit_node_builder(filter.id, |node| {
                node.set_label("Filter");
            });
        });
        ui.add_space(8.0);
        let needle = self.filter.to_ascii_lowercase();
        let icons: Vec<IconName> = IconName::ALL
            .iter()
            .copied()
            .filter(|icon| needle.is_empty() || icon.name().to_ascii_lowercase().contains(&needle))
            .collect();
        egui::ScrollArea::vertical().show(ui, |ui| {
            egui::Grid::new("icons")
                .num_columns(2)
                .spacing([16.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    for icon in icons {
                        ui.add(Icon::new(icon));
                        ui.label(icon.name());
                        ui.end_row();
                    }
                });
        });
    }
}
