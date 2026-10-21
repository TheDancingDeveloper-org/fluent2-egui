//! Breadcrumb: a file path in each size, with chevron and slash dividers, the link style, and a
//! narrow row whose middle folds into a "…" menu.

use fluent2_egui::controls::{Breadcrumb, BreadcrumbDivider, BreadcrumbItemStyle, BreadcrumbSize};

use super::Page;

const PATH: [&str; 6] = ["Home", "Documents", "Projects", "Reports", "2026", "Q3"];

#[derive(Default)]
pub struct BreadcrumbPage {
    status: String,
}

impl Page for BreadcrumbPage {
    fn name(&self) -> &'static str {
        "Breadcrumb"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Breadcrumb");
        let mut picked = None;

        ui.strong("Sizes");
        for (salt, size) in [
            ("small", BreadcrumbSize::Small),
            ("medium", BreadcrumbSize::Medium),
            ("large", BreadcrumbSize::Large),
        ] {
            let trail = Breadcrumb::new(("size", salt), PATH).size(size).show(ui);
            picked = trail.clicked.or(picked);
        }

        ui.add_space(12.0);
        ui.strong("Slash divider and link items");
        let trail = Breadcrumb::new("slash", PATH[..4].iter().copied())
            .divider(BreadcrumbDivider::Slash)
            .item_style(BreadcrumbItemStyle::Link)
            .show(ui);
        picked = trail.clicked.or(picked);

        ui.add_space(12.0);
        ui.strong("Narrow: the middle folds into \"…\"");
        ui.scope(|ui| {
            ui.set_max_width(260.0);
            let trail = Breadcrumb::new("narrow", PATH).show(ui);
            picked = trail.clicked.or(picked);
        });

        if let Some(index) = picked {
            self.status = format!("Opened {}", PATH[index]);
        }
        if !self.status.is_empty() {
            ui.add_space(12.0);
            ui.label(&self.status);
        }
    }
}
