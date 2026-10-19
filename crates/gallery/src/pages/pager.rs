//! Pager: 1,234 audit events paged with a page-size choice, plus the one-page and empty cases.

use fluent2_egui::controls::{ButtonSize, Pager, page_count};

use super::Page;

const EVENTS: usize = 1_234;

pub struct PagerPage {
    page: usize,
    page_size: usize,
    single: usize,
    empty: usize,
    status: String,
}

impl Default for PagerPage {
    fn default() -> Self {
        Self {
            page: 0,
            page_size: 25,
            single: 0,
            empty: 0,
            status: String::new(),
        }
    }
}

impl Page for PagerPage {
    fn name(&self) -> &'static str {
        "Pager"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Pager");
        let pages = page_count(EVENTS, self.page_size);
        let response = Pager::new("audit_pager", &mut self.page, pages)
            .page_sizes(&mut self.page_size, &[10, 25, 50, 100])
            .show(ui);
        if let Some(page) = response.page {
            self.status = format!(
                "Moved to page {} of {}",
                page + 1,
                page_count(EVENTS, self.page_size)
            );
        }
        if !self.status.is_empty() {
            ui.label(&self.status);
        }
        ui.add_space(8.0);
        let first = self.page * self.page_size;
        let last = (first + self.page_size).min(EVENTS);
        egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
            for event in first..last {
                ui.label(format!("#{:04}  session.start  agent:{}", event + 1, event % 7));
            }
        });

        ui.add_space(16.0);
        ui.strong("One page, small, no First and Last");
        Pager::new("single_pager", &mut self.single, 1)
            .first_last(false)
            .size(ButtonSize::Small)
            .show(ui);
        ui.add_space(8.0);
        ui.strong("No pages (everything disabled)");
        Pager::new("empty_pager", &mut self.empty, 0).show(ui);
    }
}
