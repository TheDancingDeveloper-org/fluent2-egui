//! HTML: the mail fixtures from `fluent2-egui/tests/fixtures/html` in an HtmlView (WI-1939).

use fluent2_egui::controls::{Button, Divider, SearchBox};
use fluent2_egui::data::html::{HtmlDocument, HtmlView, ImageDecision, ImageRef, LinkTarget};
use fluent2_egui::surfaces::TabList;

use super::Page;

const FIXTURES: [(&str, &str); 3] = [
    (
        "Newsletter",
        include_str!("../../../fluent2-egui/tests/fixtures/html/newsletter.html"),
    ),
    (
        "Outlook reply",
        include_str!("../../../fluent2-egui/tests/fixtures/html/outlook.html"),
    ),
    (
        "Plain",
        include_str!("../../../fluent2-egui/tests/fixtures/html/plain.html"),
    ),
];

pub struct HtmlPage {
    tab: usize,
    docs: Vec<HtmlDocument>,
    status: String,
    query: String,
    match_index: usize,
}

impl Default for HtmlPage {
    fn default() -> Self {
        Self {
            tab: 0,
            docs: FIXTURES
                .iter()
                .map(|(_, html)| HtmlDocument::parse(html))
                .collect(),
            status: "No link clicked".to_owned(),
            query: String::new(),
            match_index: 0,
        }
    }
}

/// The gallery installs no image loaders, so every image shows the blocked placeholder. An app
/// installs `egui_extras::install_image_loaders` and returns `ImageDecision::Embedded` (or
/// `Show`) for the images it allows.
fn block_all(_: &ImageRef) -> ImageDecision {
    ImageDecision::Blocked
}

impl Page for HtmlPage {
    fn name(&self) -> &'static str {
        "HTML"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("HTML");
        let mut list = TabList::new("html_fixtures", &mut self.tab);
        for (name, _) in FIXTURES {
            list = list.tab(name);
        }
        list.show(ui);
        let Some(doc) = self.docs.get(self.tab) else {
            return;
        };
        ui.horizontal(|ui| {
            ui.label(&self.status);
            if ui.add(Button::new("Copy as plain text")).clicked() {
                ui.ctx().copy_text(doc.to_plain_text());
            }
        });
        ui.horizontal(|ui| {
            ui.add(
                SearchBox::new(&mut self.query)
                    .placeholder("Find in message")
                    .label("Find in message"),
            );
            if ui.add(Button::new("Previous")).clicked() {
                self.match_index = self.match_index.saturating_sub(1);
            }
            if ui.add(Button::new("Next")).clicked() {
                self.match_index = self.match_index.saturating_add(1);
            }
        });
        ui.add(Divider::horizontal());
        let out = HtmlView::new(("html_view", self.tab), doc)
            .image_policy(&block_all)
            .find(&self.query, self.match_index)
            .show(ui);
        if out.find.matches > 0 {
            ui.label(format!("{} of {} matches", out.find.current, out.find.matches));
        }
        if let Some(hover) = &out.hovered {
            ui.weak(format!("Link: {}", hover.href));
        }
        if let Some(click) = out.clicked {
            self.status = match click.target {
                LinkTarget::Web(url) => format!("Clicked {url}"),
                LinkTarget::Blocked => String::new(),
            };
        }
        if out.blocked_images > 0 {
            ui.add_space(8.0);
            ui.weak(format!("{} images blocked", out.blocked_images));
        }
    }
}
