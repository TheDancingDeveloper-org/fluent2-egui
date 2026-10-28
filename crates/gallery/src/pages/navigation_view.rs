//! NavigationView page: a pane of items beside the content for the selected item.

use fluent2_egui::data::nav_view::{NavState, NavigationView, PaneMode};
use fluent2_icons::Icon;

use super::Page;

/// Gallery page for NavigationView.
pub struct NavigationViewPage {
    state: NavState,
}

impl Default for NavigationViewPage {
    fn default() -> Self {
        Self {
            state: NavState {
                selected: Some("home".to_owned()),
                ..NavState::default()
            },
        }
    }
}

impl Page for NavigationViewPage {
    fn name(&self) -> &'static str {
        "Navigation view"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Navigation view");
        ui.label(
            "Up and Down move between items; Enter or Space selects one. The pane follows the \
             window width: expanded, an icon rail, or a hamburger over the content. The mode \
             switch below forces one of those.",
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label("Pane");
            for (mode, label) in [
                (PaneMode::Auto, "Auto"),
                (PaneMode::Expanded, "Expanded"),
                (PaneMode::Compact, "Compact"),
                (PaneMode::Minimal, "Minimal"),
            ] {
                ui.selectable_value(&mut self.state.mode, mode, label);
            }
        });
        ui.add_space(8.0);
        ui.set_min_height(360.0);
        let title = match self.state.selected.as_deref() {
            Some("home") => "Home",
            Some("files") => "Files",
            Some("mail") => "Mail",
            Some("settings") => "Settings",
            _ => "Nothing selected",
        };
        NavigationView::new("gallery_navigation")
            .title("Library")
            .item("home", Icon::Home, "Home")
            .item("files", Icon::Folder, "Files")
            .section("Other")
            .item("mail", Icon::Mail, "Mail")
            .footer_item("settings", Icon::Settings, "Settings")
            .show(ui, &mut self.state, |ui| {
                ui.heading(title);
                ui.label("Content for the selected item.");
            });
    }
}
