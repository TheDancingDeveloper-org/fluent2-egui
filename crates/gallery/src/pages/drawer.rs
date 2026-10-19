//! Drawer page: overlay drawers from either edge, an inline drawer and a bottom sheet.

use fluent2_egui::controls::{Button, Switch};
use fluent2_egui::surfaces::{BottomSheet, Drawer, DrawerPosition, DrawerSize};

use super::Page;

/// Gallery page for Drawer and BottomSheet.
#[derive(Default)]
pub struct DrawerPage {
    start: bool,
    end: bool,
    sheet: bool,
    inline: bool,
}

impl Page for DrawerPage {
    fn name(&self) -> &'static str {
        "Drawer"
    }

    fn ui(&mut self, ui: &mut egui::Ui) {
        let _ = Drawer::new("gallery_inline_drawer")
            .title("Inline drawer")
            .size(DrawerSize::Custom(240.0))
            .show_inline(ui, &mut self.inline, |ui| {
                ui.label("An inline drawer sits beside the page and traps nothing.");
            });

        ui.label("Overlay drawers and the bottom sheet dim the window and trap focus. Escape, the backdrop or Close dismisses them.");
        ui.horizontal_wrapped(|ui| {
            if ui
                .add(Button::new("Open start drawer").id_salt("start"))
                .clicked()
            {
                self.start = true;
            }
            if ui.add(Button::new("Open end drawer").id_salt("end")).clicked() {
                self.end = true;
            }
            if ui
                .add(Button::new("Open bottom sheet").id_salt("sheet"))
                .clicked()
            {
                self.sheet = true;
            }
        });
        let _ = ui.add(Switch::new(&mut self.inline, "Inline drawer"));

        let ctx = ui.ctx().clone();
        let _ = Drawer::new("gallery_start_drawer")
            .title("Navigation")
            .position(DrawerPosition::Start)
            .show(&ctx, &mut self.start, |ui| {
                for item in ["Home", "Connections", "Settings"] {
                    let _ = ui.add(Button::new(item).id_salt(item));
                }
            });
        let _ = Drawer::new("gallery_end_drawer")
            .title("Details")
            .position(DrawerPosition::End)
            .size(DrawerSize::Medium)
            .show(&ctx, &mut self.end, |ui| {
                ui.label("Side panel content.");
            });
        let _ = BottomSheet::new("gallery_sheet")
            .title("More")
            .show(&ctx, &mut self.sheet, |ui| {
                for item in ["Share", "Rename", "Delete"] {
                    let _ = ui.add(Button::new(item).id_salt(item));
                }
            });
    }
}
