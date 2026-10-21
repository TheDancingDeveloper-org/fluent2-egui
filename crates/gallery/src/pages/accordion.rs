//! Accordion page: single and multiple open, sizes and chevron position.

use fluent2_egui::controls::Switch;
use fluent2_egui::surfaces::{Accordion, AccordionItem, AccordionSize, ExpandIconPosition};

use super::Page;

/// Gallery page for Accordion.
pub struct AccordionPage {
    single: Vec<usize>,
    multiple: Vec<usize>,
    sized: [Vec<usize>; 4],
    collapsible: bool,
    end_icon: bool,
}

impl Default for AccordionPage {
    fn default() -> Self {
        Self {
            single: vec![0],
            multiple: Vec::new(),
            sized: Default::default(),
            collapsible: true,
            end_icon: false,
        }
    }
}

impl Page for AccordionPage {
    fn name(&self) -> &'static str {
        "Accordion"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        ui.horizontal_wrapped(|ui| {
            let _ = ui.add(Switch::new(&mut self.collapsible, "Collapsible"));
            let _ = ui.add(Switch::new(&mut self.end_icon, "Chevron at end"));
        });
        let position = if self.end_icon {
            ExpandIconPosition::End
        } else {
            ExpandIconPosition::Start
        };

        ui.strong("Single open");
        let _ = Accordion::new("gallery_single", &mut self.single)
            .collapsible(self.collapsible)
            .expand_icon_position(position)
            .item(AccordionItem::new("General", |ui| {
                ui.label("Language, region and start-up options.");
            }))
            .item(AccordionItem::new("Appearance", |ui| {
                ui.label("Theme, density and font size.");
            }))
            .item(
                AccordionItem::new("Disabled", |ui| {
                    ui.label("Unreachable.");
                })
                .disabled(true),
            )
            .show(ui);

        ui.add_space(16.0);
        ui.strong("Multiple open");
        let _ = Accordion::new("gallery_multiple", &mut self.multiple)
            .multiple(true)
            .collapsible(true)
            .expand_icon_position(position)
            .item(AccordionItem::new("Notifications", |ui| {
                ui.label("Sounds, banners and badges.");
            }))
            .item(AccordionItem::new("Privacy", |ui| {
                ui.label("Telemetry and history.");
            }))
            .show(ui);

        ui.add_space(16.0);
        ui.strong("Sizes");
        let sizes = [
            ("Small", AccordionSize::Small),
            ("Medium", AccordionSize::Medium),
            ("Large", AccordionSize::Large),
            ("Extra large", AccordionSize::ExtraLarge),
        ];
        for ((name, size), open) in sizes.into_iter().zip(self.sized.iter_mut()) {
            let _ = Accordion::new(("gallery_size", name), open)
                .size(size)
                .collapsible(true)
                .expand_icon_position(position)
                .item(AccordionItem::new(format!("{name} header"), move |ui| {
                    ui.label(format!("A {} panel.", name.to_lowercase()));
                }))
                .show(ui);
        }
    }
}
