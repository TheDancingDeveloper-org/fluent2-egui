//! Undecorated window with resize edges (WI-1560).
//!
//! Run with `cargo run -p gallery --example chrome`. Drag any border or corner to resize.
//!
//! Not affiliated with or endorsed by Microsoft.

#![warn(missing_docs)]

use eframe::egui;
use fluent2_egui::{Theme, install, office::window_edges::window_resize_edges};

struct ChromeApp;

impl eframe::App for ChromeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        window_resize_edges(&ctx);
        egui::CentralPanel::default().show(ui, |ui| {
            ui.label("Drag the 6 pt border to resize this window.");
        });
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_inner_size([640.0, 420.0]),
        ..Default::default()
    };
    eframe::run_native(
        "fluent2-egui chrome",
        options,
        Box::new(|cc| {
            install(&cc.egui_ctx, &Theme::default());
            Ok(Box::new(ChromeApp))
        }),
    )
}
