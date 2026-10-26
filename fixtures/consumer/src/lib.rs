//! Compiles as a downstream crate of `fluent2-egui`.
//!
//! CI checks this crate with default features off, with each optional feature
//! on its own, and for `wasm32-unknown-unknown`. It does not paint anything.

/// A function a consumer can call without turning a feature on.
pub fn uses_library() -> fluent2_egui::ThemeKind {
    fluent2_egui::ThemeKind::Light
}
