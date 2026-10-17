//! Window chrome demo: an undecorated window with the Fluent TitleBar and resize edges.
//!
//! Run with `cargo run -p gallery --example chrome`. The gallery window keeps its own
//! decorations, so the chrome is shown in this separate window. Drag the empty bar, double-click
//! it to maximise, use the caption buttons, and drag any border or corner to resize.
//!
//! Not affiliated with or endorsed by Microsoft.

#![warn(missing_docs)]

use eframe::egui;
use fluent2_egui::office::window_edges::window_resize_edges;
use fluent2_egui::office::{TitleBar, window_command};
use fluent2_egui::{IconName, Theme, install};

struct ChromeApp;

impl eframe::App for ChromeApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        window_resize_edges(&ctx);
        let (focused, maximized) = ctx.input(|input| {
            let viewport = input.viewport();
            (viewport.focused, viewport.maximized.unwrap_or(false))
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let response = TitleBar::new("chrome_example_title", "Window chrome")
                .maximized(maximized)
                .focused(focused)
                .show(ui, |slots| {
                    slots.app_icon(IconName::Document);
                    slots.right(|ui| {
                        ui.label("Fluent TitleBar");
                    });
                });
            if let Some(action) = response.action {
                ctx.send_viewport_cmd(window_command(action, maximized));
            }
            ui.add_space(12.0);
            ui.label("This window is undecorated. The title bar above is the only chrome.");
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
        "fluent2-egui window chrome",
        options,
        Box::new(|cc| {
            install(&cc.egui_ctx, &Theme::default());
            Ok(Box::new(ChromeApp))
        }),
    )
}
