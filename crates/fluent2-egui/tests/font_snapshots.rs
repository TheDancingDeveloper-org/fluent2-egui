//! Pixel snapshots of the Typography and Buttons samples, for the open UI font review (WI-1588).
//!
//! Needs a wgpu adapter: `cargo test -p fluent2-egui --features snapshots --test font_snapshots`.
//! The image for each font option is written to `docs/screenshots/wi-1588/` when
//! `UPDATE_SNAPSHOTS=1` is set. The option is the cargo feature this run was built with: the
//! default build renders Inter, `--no-default-features` renders egui's default face.

#![cfg(feature = "snapshots")]

use egui_kittest::{Harness, SnapshotOptions};
use fluent2_egui::controls::{Appearance, Button, ButtonSize};
use fluent2_egui::foundation::text::{TypeRamp, rich};
use fluent2_egui::{Theme, install};

#[cfg(feature = "bundled-font")]
const OPTION: &str = "inter";
#[cfg(not(feature = "bundled-font"))]
const OPTION: &str = "egui-default";

/// Output directory, relative to this crate (cargo runs tests from the crate directory).
const OUT: &str = "../../docs/screenshots/wi-1588";

fn typography(ui: &mut egui::Ui) {
    ui.heading("Type ramp");
    ui.add_space(8.0);
    for step in TypeRamp::ALL {
        ui.label(rich(
            format!("{}  The quick brown fox jumps over the lazy dog", step.name()),
            step,
        ));
        ui.add_space(4.0);
    }
}

fn buttons(ui: &mut egui::Ui) {
    let appearances = [
        Appearance::Secondary,
        Appearance::Primary,
        Appearance::Outline,
        Appearance::Subtle,
        Appearance::Transparent,
    ];
    for appearance in appearances {
        ui.horizontal(|ui| {
            for size in [ButtonSize::Small, ButtonSize::Medium, ButtonSize::Large] {
                let label = format!("{appearance:?} {size:?}");
                ui.add(Button::new(label).appearance(appearance).size(size));
            }
        });
        ui.add_space(6.0);
    }
}

/// Render `content` at `scale` percent and write or compare `{OPTION}-{name}-{scale}.png`.
///
/// The harness draws one frame while it is built. That frame installs the theme and draws
/// nothing, so the sample is first drawn under the installed text styles and fonts.
fn snapshot(name: &str, scale: u32, size: [f32; 2], content: fn(&mut egui::Ui)) {
    let mut installed = false;
    let mut harness = Harness::builder()
        .with_size(size)
        .with_pixels_per_point(scale as f32 / 100.0)
        .build_ui(move |ui| {
            if !installed {
                install(ui.ctx(), &Theme::default());
                installed = true;
                ui.ctx().request_repaint();
                return;
            }
            content(ui);
        });
    harness.run();
    let snapshot_name = format!("{OPTION}-{name}-{scale}");
    harness.snapshot_options(snapshot_name, &SnapshotOptions::new().output_path(OUT));
}

#[test]
fn typography_at_100_percent() {
    snapshot("typography", 100, [640.0, 1000.0], typography);
}

#[test]
fn typography_at_150_percent() {
    snapshot("typography", 150, [640.0, 1000.0], typography);
}

#[test]
fn buttons_at_100_percent() {
    snapshot("buttons", 100, [640.0, 300.0], buttons);
}

#[test]
fn buttons_at_150_percent() {
    snapshot("buttons", 150, [640.0, 300.0], buttons);
}
