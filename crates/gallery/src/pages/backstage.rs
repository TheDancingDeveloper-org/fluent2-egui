//! Backstage page: a File button opens the backstage with Home, New, Open and Info pages.
//!
//! The ribbon's tab strip (WI-1562) is not on main yet, so this page stands in for the Ribbon
//! page. Its File button sends the event the File tab will send. The backstage pages hold
//! placeholder content only.

use fluent2_egui::controls::Button;
use fluent2_egui::office::backstage::{Backstage, BackstageState};

use super::Page;

/// Gallery page for the Backstage (spec §7).
#[derive(Default)]
pub struct BackstagePage {
    state: BackstageState,
}

impl Page for BackstagePage {
    fn name(&self) -> &'static str {
        "Backstage"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        // The File button is drawn before `show`, so focus can return to it on the frame the
        // backstage closes. While the backstage is open the button is not drawn.
        if !self.state.open {
            ui.heading("Backstage");
            ui.label("Press File to open the backstage. The ribbon's File tab will do the same.");
            ui.add_space(8.0);
            let file = ui.add(Button::new("File").id_salt("file"));
            if file.clicked() {
                // Focus the File button first, so closing the backstage returns focus to it.
                file.request_focus();
                self.state.open = true;
            }
        }

        Backstage::new(&mut self.state)
            .page("home", "Home", |ui| {
                ui.label("Placeholder content for Home.");
            })
            .page("new", "New", |ui| {
                ui.label("Placeholder content for New.");
            })
            .page("open", "Open", |ui| {
                ui.label("Placeholder content for Open.");
            })
            .page("info", "Info", |ui| {
                ui.label("Placeholder content for Info.");
            })
            .bottom_page("options", "Options", |ui| {
                ui.label("Placeholder content for Options.");
            })
            .show(ui);
    }
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use fluent2_egui::Theme;

    use super::*;

    #[test]
    fn file_button_opens_the_backstage_with_the_gallery_pages() {
        let mut page = BackstagePage::default();
        let mut harness = Harness::new_ui(move |ui| {
            fluent2_egui::install(ui.ctx(), &Theme::default());
            page.ui(ui);
        });
        harness.run();
        harness.get_by_label("File").click();
        harness.run();
        let _ = harness.get_by_role_and_label(egui::accesskit::Role::Dialog, "Backstage");
        for label in ["Home", "New", "Open", "Info", "Options"] {
            let _ = harness.get_by_role_and_label(egui::accesskit::Role::Button, label);
        }
    }

    #[test]
    fn escape_closes_the_backstage_and_shows_file_again() {
        let mut page = BackstagePage::default();
        let mut harness = Harness::new_ui(move |ui| {
            fluent2_egui::install(ui.ctx(), &Theme::default());
            page.ui(ui);
        });
        harness.run();
        harness.get_by_label("File").click();
        harness.run();
        harness.key_press(egui::Key::Escape);
        harness.run();
        assert!(
            harness.query_by_label("Back").is_none(),
            "Escape closed the backstage"
        );
        let _ = harness.get_by_label("File");
    }
}
