//! Gallery page for [`fluent2_egui::surfaces::MessageBar`].
//!
//! `docs/ARCHITECTURE.md` puts the gallery crate at `gallery` and says it comes later, so this
//! is not a workspace member yet. The message bar tests compile this page.

use egui::Ui;
use fluent2_egui::surfaces::{Intent, MessageBar};

const INTENTS: [Intent; 4] = [Intent::Info, Intent::Success, Intent::Warning, Intent::Error];

/// One bar per intent, then a dismissible bar with a title and two actions.
pub fn page(ui: &mut Ui) {
    ui.label("Message bars");
    for intent in INTENTS {
        let _ = MessageBar::new(intent, format!("{intent:?} message text")).show(ui);
    }
    let _ = MessageBar::new(Intent::Warning, "Your changes are not saved.")
        .title("Unsaved changes")
        .action("Save")
        .action("Discard")
        .dismissible()
        .show(ui);
}
