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
        Box::new(|cc| Ok(Box::new(GalleryApp::from_creation(cc)))),
    ) {
        eprintln!("gallery: {error}");
    }
}

#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(async {
        if let Err(error) = start_web().await {
            log::error!("gallery failed to start: {error}");
        }
    });
}

#[cfg(target_arch = "wasm32")]
async fn start_web() -> Result<(), String> {
    use eframe::wasm_bindgen::JsCast as _;

    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or("no document")?;
    let canvas = document
        .get_element_by_id("the_canvas_id")
        .ok_or("no #the_canvas_id element")?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| "#the_canvas_id is not a canvas")?;
    eframe::WebRunner::new()
        .start(
            canvas,
            eframe::WebOptions::default(),
            Box::new(|cc| Ok(Box::new(GalleryApp::from_creation(cc)))),
        )
        .await
        .map_err(|error| format!("{error:?}"))
}
