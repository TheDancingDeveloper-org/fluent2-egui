//! fluent2-egui gallery.
//!
//! Native: `cargo run -p gallery`. Wasm: `trunk serve crates/gallery/index.html`.
//!
//! Not affiliated with or endorsed by Microsoft.

#![warn(missing_docs)]

mod app;
mod pages;

use app::GalleryApp;

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let native = eframe::NativeOptions::default();
    if let Err(error) = eframe::run_native(
        "fluent2-egui gallery",
        native,
        Box::new(|cc| Ok(Box::new(GalleryApp::new(&cc.egui_ctx)))),
    ) {
        eprintln!("gallery: {error}");
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async move {
        let document = web_sys::window()
            .and_then(|window| window.document())
            .expect("no document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("no #the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("not a canvas");
        let start = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| Ok(Box::new(GalleryApp::new(&cc.egui_ctx)))),
            )
            .await;
        if let Err(error) = start {
            log::error!("gallery failed to start: {error:?}");
        }
    });
}
