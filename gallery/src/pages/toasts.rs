//! Gallery page for [`fluent2_egui::surfaces::Toaster`].
//!
//! The page only pushes toasts. The app shell calls [`Toaster::show`] once per frame, so the
//! stack is not drawn twice.

use std::time::Duration;

use egui::Ui;
use fluent2_egui::controls::{Appearance, Button};
use fluent2_egui::surfaces::{Intent, Toast, Toaster};

const INTENTS: [Intent; 4] = [Intent::Info, Intent::Success, Intent::Warning, Intent::Error];

/// One button per intent. Each press pushes a toast that expires after five seconds.
pub fn page(ui: &mut Ui) {
    ui.label("Toasts");
    ui.horizontal_wrapped(|ui| {
        for intent in INTENTS {
            let clicked = ui
                .add(Button::new(format!("{intent:?} toast")).appearance(Appearance::Outline))
                .clicked();
            if clicked {
                let _ = Toaster::push(
                    ui.ctx(),
                    Toast {
                        intent,
                        title: format!("{intent:?} toast"),
                        body: Some("Pushed from the gallery.".to_owned()),
                        timeout: Some(Duration::from_secs(5)),
                        action: Some("Undo".to_owned()),
                    },
                );
            }
        }
    });
}
